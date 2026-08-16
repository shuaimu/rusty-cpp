//! Deliberately narrow source-owned C++ names for root free functions.
//!
//! Rust has no free-function overloading, while an existing C++ ABI may expose
//! several overloads under one name.  `cpp_name` bridges only that gap.  The
//! accepted spelling is inert for rustc:
//!
//! ```ignore
//! #[cfg_attr(any(), cpp_name(existing_cpp_name))]
//! fn distinct_rust_name(value: i32) {}
//! ```
//!
//! The marker is intentionally fail-closed.  It is reserved in opaque macro
//! tokens, is valid only on root free functions, and accepts one unqualified
//! ASCII C++ identifier.  Code generation performs a second preflight after
//! type collection to prove that every shared C++ name is a real overload.

use quote::ToTokens;
use std::collections::{BTreeMap, BTreeSet};
use syn::ext::IdentExt;
use syn::parse::Parser;
use syn::punctuated::Punctuated;
use syn::visit::{self, Visit};
use syn::{Attribute, Item, Meta, Token};

#[derive(Clone, Debug, Default)]
pub(crate) struct CppNamePlan {
    functions: BTreeMap<String, String>,
}

impl CppNamePlan {
    pub(crate) fn is_empty(&self) -> bool {
        self.functions.is_empty()
    }

    pub(crate) fn function_name(&self, rust_name: &str) -> Option<&str> {
        self.functions.get(rust_name).map(String::as_str)
    }

    pub(crate) fn target_names(&self) -> BTreeSet<String> {
        self.functions.values().cloned().collect()
    }

    pub(crate) fn rust_names(&self) -> BTreeSet<String> {
        self.functions.keys().cloned().collect()
    }
}

fn semantic_ident(ident: &proc_macro2::Ident) -> String {
    ident.unraw().to_string()
}

fn marker_ident(ident: &proc_macro2::Ident) -> bool {
    semantic_ident(ident) == "cpp_name"
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

fn token_stream_contains_unaudited_nested_macro(tokens: proc_macro2::TokenStream) -> bool {
    let tokens = tokens.into_iter().collect::<Vec<_>>();
    for (index, token) in tokens.iter().enumerate() {
        if let proc_macro2::TokenTree::Group(group) = token
            && token_stream_contains_unaudited_nested_macro(group.stream())
        {
            return true;
        }
        let proc_macro2::TokenTree::Ident(ident) = token else {
            continue;
        };
        let is_macro_invocation = tokens.get(index + 1).is_some_and(
            |next| matches!(next, proc_macro2::TokenTree::Punct(punct) if punct.as_char() == '!'),
        ) && tokens
            .get(index + 2)
            .is_some_and(|next| matches!(next, proc_macro2::TokenTree::Group(_)));
        if !is_macro_invocation {
            continue;
        }
        if semantic_ident(ident) != "assert" {
            return true;
        }
        let Some(proc_macro2::TokenTree::Group(group)) = tokens.get(index + 2) else {
            return true;
        };
        if token_stream_contains_unaudited_nested_macro(group.stream()) {
            return true;
        }
    }
    false
}

fn meta_mentions_marker(meta: &Meta) -> bool {
    path_mentions_marker(meta.path())
        || match meta {
            Meta::List(list) => {
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
    path_mentions_marker(attr.path()) || meta_mentions_marker(&attr.meta)
}

/// Attributes in a source file with `cpp_name` must be known not to expand
/// new items.  In particular, an attribute or derive procedural macro can add
/// a hidden root function after this syntax-only preflight has proved overload
/// ownership.  Keep this list deliberately small: these are the inert builtin
/// attributes used by the callbacks source and the provenance regression
/// fixtures.
fn attribute_is_audited_inert(attr: &Attribute) -> bool {
    if attr.path().leading_colon.is_some() || attr.path().segments.len() != 1 {
        return false;
    }
    attr.path().segments.first().is_some_and(|segment| {
        matches!(
            semantic_ident(&segment.ident).as_str(),
            "doc" | "allow" | "warn" | "deny" | "forbid" | "no_std"
        )
    })
}

fn is_cpp_trait_member_dispatch_name(name: &str) -> bool {
    name == "cpp_trait_member_dispatch"
}

/// This compiler-owned trait marker is safe for cpp_name's syntax-only audit
/// only in its one inert spelling. Keeping the whole `cfg_attr` shape exact
/// prevents an active, qualified, nested, or argument-bearing attribute macro
/// from expanding items behind overload ownership preflight.
fn attribute_is_exact_inactive_cpp_trait_member_dispatch(attr: &Attribute) -> bool {
    let Meta::List(list) = &attr.meta else {
        return false;
    };
    if !is_simple_path(&list.path, "cfg_attr") {
        return false;
    }
    let parser = Punctuated::<Meta, Token![,]>::parse_terminated;
    parser.parse2(list.tokens.clone()).is_ok_and(|nested| {
        if nested.len() != 2 {
            return false;
        }
        let Some(Meta::List(predicate)) = nested.first() else {
            return false;
        };
        let Some(Meta::Path(marker)) = nested.iter().nth(1) else {
            return false;
        };
        is_simple_path(&predicate.path, "any")
            && predicate.tokens.is_empty()
            && is_simple_path(marker, "cpp_trait_member_dispatch")
    })
}

/// Compiler-owned markers that are hidden from rustc with the exact
/// `cfg_attr(any(), marker)` spelling cannot run an attribute macro or add
/// hidden Rust items.  Their C++ effects are still validated by each marker's
/// ordinary lowering path.
fn attribute_is_exact_inactive_transpiler_marker(attr: &Attribute) -> bool {
    let Meta::List(list) = &attr.meta else {
        return false;
    };
    if !is_simple_path(&list.path, "cfg_attr") {
        return false;
    }
    let parser = Punctuated::<Meta, Token![,]>::parse_terminated;
    parser.parse2(list.tokens.clone()).is_ok_and(|nested| {
        let Some(Meta::List(predicate)) = nested.first() else {
            return false;
        };
        nested.len() >= 2
            && is_simple_path(&predicate.path, "any")
            && predicate.tokens.is_empty()
            && nested.iter().skip(1).all(audited_transpiler_marker_meta)
    })
}

fn audited_builtin_macro_name(name: &str) -> bool {
    matches!(
        name,
        "assert"
            | "Clone"
            | "Copy"
            | "Debug"
            | "Default"
            | "Eq"
            | "Hash"
            | "Ord"
            | "PartialEq"
            | "PartialOrd"
    )
}

fn audited_builtin_or_attribute_name(name: &str) -> bool {
    audited_builtin_macro_name(name)
        || is_cpp_trait_member_dispatch_name(name)
        || matches!(
            name,
            "allow"
                | "cfg"
                | "cfg_attr"
                | "deny"
                | "doc"
                | "forbid"
                | "format"
                | "no_mangle"
                | "no_std"
                | "repr"
                | "warn"
        )
}

fn audited_builtin_derive(meta: &Meta) -> bool {
    let Meta::List(list) = meta else {
        return false;
    };
    if !is_simple_path(&list.path, "derive") {
        return false;
    }
    let parser = Punctuated::<syn::Path, Token![,]>::parse_terminated;
    parser.parse2(list.tokens.clone()).is_ok_and(|paths| {
        !paths.is_empty()
            && paths.iter().all(|path| {
                path.leading_colon.is_none()
                    && path.segments.len() == 1
                    && path.segments.first().is_some_and(|segment| {
                        audited_builtin_macro_name(&semantic_ident(&segment.ident))
                            && semantic_ident(&segment.ident) != "assert"
                    })
            })
    })
}

fn audited_transpiler_marker_meta(meta: &Meta) -> bool {
    let path = meta.path();
    path.leading_colon.is_none()
        && path.segments.len() == 1
        && path.segments.first().is_some_and(|segment| {
            matches!(
                semantic_ident(&segment.ident).as_str(),
                "cpp_name"
                    | "cpp_abi"
                    | "cpp_abi_alias"
                    | "cpp_ctor"
                    | "cpp_default_argument"
                    | "cpp_explicit"
                    | "cpp_import_namespace"
                    | "cpp_internal"
                    | "cpp_marker_impl"
                    | "cpp_marker_trait"
                    | "cpp_namespace"
                    | "cpp_no_auto_traits"
                    | "cpp_no_fieldwise_ctor"
                    | "cpp_noexcept"
                    | "thread_local"
            )
        })
}

fn cfg_attr_payload_is_audited(
    meta: &Meta,
    allow_cpp_inherit: bool,
    allow_transpiler_marker: bool,
) -> bool {
    if audited_builtin_derive(meta)
        || (allow_transpiler_marker && audited_transpiler_marker_meta(meta))
    {
        return true;
    }
    let path = meta.path();
    if path.leading_colon.is_none() && path.segments.len() == 1 {
        let name = path
            .segments
            .first()
            .map(|segment| semantic_ident(&segment.ident));
        if name.as_deref().is_some_and(|name| {
            matches!(
                name,
                "doc" | "allow" | "warn" | "deny" | "forbid" | "repr" | "cfg"
            ) || (allow_cpp_inherit && name == "cpp_inherit")
        }) {
            return true;
        }
    }
    let Meta::List(list) = meta else {
        return false;
    };
    if !is_simple_path(&list.path, "cfg_attr") {
        return false;
    }
    let parser = Punctuated::<Meta, Token![,]>::parse_terminated;
    parser.parse2(list.tokens.clone()).is_ok_and(|nested| {
        let marker_predicate_is_exact_any = nested.first().is_some_and(|predicate| {
            let Meta::List(predicate) = predicate else {
                return false;
            };
            is_simple_path(&predicate.path, "any") && predicate.tokens.is_empty()
        });
        nested.len() >= 2
            && nested.iter().skip(1).all(|payload| {
                cfg_attr_payload_is_audited(
                    payload,
                    allow_cpp_inherit,
                    marker_predicate_is_exact_any,
                )
            })
    })
}

/// Extra compiler-owned/inert surfaces needed by marker-free siblings in the
/// production callbacks crate. Unlike the strict same-file allowlist, nested
/// `cfg_attr` payloads are parsed recursively and every possible payload must
/// itself be audited; evaluating a currently-false predicate is not trusted.
fn attribute_is_audited_crate_wide(attr: &Attribute, allow_cpp_inherit: bool) -> bool {
    if attribute_is_audited_inert(attr) {
        return true;
    }
    if attr.path().leading_colon.is_some() || attr.path().segments.len() != 1 {
        return false;
    }
    let name = attr
        .path()
        .segments
        .first()
        .map(|segment| semantic_ident(&segment.ident));
    match name.as_deref() {
        Some("repr" | "cfg") => true,
        Some("no_mangle") => matches!(attr.meta, Meta::Path(_)),
        Some("derive") => audited_builtin_derive(&attr.meta),
        Some("cfg_attr") => cfg_attr_payload_is_audited(&attr.meta, allow_cpp_inherit, false),
        Some("cpp_inherit") => allow_cpp_inherit,
        _ => false,
    }
}

fn attribute_is_authenticated_cpp_inherit(attr: &Attribute) -> bool {
    fn meta_contains_cpp_inherit(meta: &Meta) -> bool {
        if is_simple_path(meta.path(), "cpp_inherit") {
            return true;
        }
        let Meta::List(list) = meta else {
            return false;
        };
        if !is_simple_path(&list.path, "cfg_attr") {
            return false;
        }
        let parser = Punctuated::<Meta, Token![,]>::parse_terminated;
        parser.parse2(list.tokens.clone()).is_ok_and(|nested| {
            nested
                .iter()
                .skip(1)
                .any(meta_contains_cpp_inherit)
        })
    }

    meta_contains_cpp_inherit(&attr.meta) && attribute_is_audited_crate_wide(attr, true)
}

fn is_simple_path(path: &syn::Path, expected: &str) -> bool {
    path.leading_colon.is_none()
        && path.segments.len() == 1
        && path
            .segments
            .first()
            .is_some_and(|segment| segment.ident == expected)
}

fn validate_cpp_identifier(ident: &proc_macro2::Ident) -> Result<String, String> {
    let spelling = ident.to_string();
    if spelling.starts_with("r#") {
        return Err("cpp_name requires a non-raw C++ identifier".to_string());
    }
    if spelling.is_empty()
        || !spelling.bytes().enumerate().all(|(index, byte)| {
            byte == b'_' || byte.is_ascii_alphabetic() || (index > 0 && byte.is_ascii_digit())
        })
        || spelling.as_bytes()[0].is_ascii_digit()
    {
        return Err("cpp_name requires one simple ASCII C++ identifier".to_string());
    }
    if crate::codegen::escape_cpp_keyword(&spelling) != spelling {
        return Err(format!("cpp_name target `{spelling}` is a C++ keyword"));
    }
    if spelling == "main"
        || spelling == "cpp_name"
        || spelling == "std"
        || spelling == "rusty"
        || spelling.starts_with('_')
        || spelling.contains("__")
        || spelling.starts_with("rusty_cpp")
    {
        return Err(format!(
            "cpp_name target `{spelling}` is reserved for C++ or generated-code use"
        ));
    }
    Ok(spelling)
}

fn parse_marker_attr(attr: &Attribute) -> Result<Option<String>, String> {
    if !attribute_mentions_marker(attr) {
        return Ok(None);
    }
    if path_mentions_marker(attr.path()) {
        return Err(
            "cpp_name must use the inert form #[cfg_attr(any(), cpp_name(...))]".to_string(),
        );
    }
    if !is_simple_path(attr.path(), "cfg_attr") {
        return Err(
            "cpp_name marker must be the sole payload of #[cfg_attr(any(), ...)]".to_string(),
        );
    }
    let Meta::List(cfg_attr) = &attr.meta else {
        return Err("cpp_name requires list-form cfg_attr".to_string());
    };
    let parser = Punctuated::<Meta, Token![,]>::parse_terminated;
    let metas = parser
        .parse2(cfg_attr.tokens.clone())
        .map_err(|error| format!("malformed cpp_name cfg_attr: {error}"))?;
    if metas.len() != 2 || meta_mentions_marker(&metas[0]) || !meta_mentions_marker(&metas[1]) {
        return Err(
            "cpp_name marker must be the sole payload of #[cfg_attr(any(), ...)]".to_string(),
        );
    }
    let Meta::List(predicate) = &metas[0] else {
        return Err("cpp_name cfg_attr predicate must be exactly any()".to_string());
    };
    if !is_simple_path(&predicate.path, "any") || !predicate.tokens.is_empty() {
        return Err("cpp_name cfg_attr predicate must be exactly any()".to_string());
    }
    let Meta::List(marker) = &metas[1] else {
        return Err("cpp_name requires one parenthesized C++ identifier".to_string());
    };
    if !is_simple_path(&marker.path, "cpp_name") {
        return Err(
            "cpp_name marker path must be the exact unqualified identifier `cpp_name`".to_string(),
        );
    }
    let target = syn::parse2::<proc_macro2::Ident>(marker.tokens.clone())
        .map_err(|_| "cpp_name requires exactly one C++ identifier".to_string())?;
    validate_cpp_identifier(&target).map(Some)
}

pub(crate) fn attribute_is_marker(attr: &Attribute) -> bool {
    parse_marker_attr(attr).ok().flatten().is_some()
}

#[derive(Default)]
struct DescendantMarkerCollector {
    marker_attr: Option<String>,
    opaque_macro: Option<String>,
}

impl<'ast> Visit<'ast> for DescendantMarkerCollector {
    fn visit_attribute(&mut self, attr: &'ast Attribute) {
        if self.marker_attr.is_none() && attribute_mentions_marker(attr) {
            self.marker_attr = Some(attr.to_token_stream().to_string());
        }
        visit::visit_attribute(self, attr);
    }

    fn visit_macro(&mut self, mac: &'ast syn::Macro) {
        if self.opaque_macro.is_none()
            && (path_mentions_marker(&mac.path) || token_stream_mentions_marker(mac.tokens.clone()))
        {
            self.opaque_macro = Some(mac.path.to_token_stream().to_string());
        }
        // Opaque tokens are deliberately not interpreted as Rust AST.
    }
}

fn reject_descendant_marker<T>(
    node: &T,
    context: &str,
    visit_node: impl FnOnce(&mut DescendantMarkerCollector, &T),
) -> Result<(), String> {
    let mut collector = DescendantMarkerCollector::default();
    visit_node(&mut collector, node);
    if let Some(attr) = collector.marker_attr {
        return Err(format!(
            "cpp_name is supported only on a crate-file root free function; found `{attr}` in {context}"
        ));
    }
    if let Some(mac) = collector.opaque_macro {
        return Err(format!(
            "reserved cpp_name identifier inside opaque macro tokens in {context}: `{mac}`"
        ));
    }
    Ok(())
}

pub(crate) fn validate_cpp_name_function_shape(function: &syn::ItemFn) -> Result<(), String> {
    let sig = &function.sig;
    if sig.ident.to_string().starts_with("r#") {
        return Err(format!(
            "cpp_name function `{}` must use a non-raw Rust identifier",
            sig.ident
        ));
    }
    if !sig.generics.params.is_empty() || sig.generics.where_clause.is_some() {
        if sig.generics.params.len() != 1 {
            return Err(format!(
                "generic cpp_name function `{}` must declare exactly one type parameter",
                sig.ident
            ));
        }
        let Some(syn::GenericParam::Type(type_param)) = sig.generics.params.first() else {
            return Err(format!(
                "generic cpp_name function `{}` must declare exactly one type parameter",
                sig.ident
            ));
        };
        if sig.generics.where_clause.is_some()
            || !type_param.attrs.is_empty()
            || type_param.default.is_some()
            || type_param.bounds.iter().any(|bound| {
                !matches!(bound, syn::TypeParamBound::Lifetime(lifetime) if lifetime.ident == "static")
            })
        {
            return Err(format!(
                "generic cpp_name function `{}` supports only one unconstrained or `'static` type parameter and no where-clause",
                sig.ident
            ));
        }

        struct ReturnTypeParamUse<'a> {
            type_param: &'a str,
            found: bool,
        }
        impl<'ast> Visit<'ast> for ReturnTypeParamUse<'_> {
            fn visit_path(&mut self, path: &'ast syn::Path) {
                if path
                    .segments
                    .iter()
                    .any(|segment| semantic_ident(&segment.ident) == self.type_param)
                {
                    self.found = true;
                    return;
                }
                visit::visit_path(self, path);
            }
        }
        if let syn::ReturnType::Type(_, return_type) = &sig.output {
            let type_param_name = semantic_ident(&type_param.ident);
            let mut use_collector = ReturnTypeParamUse {
                type_param: &type_param_name,
                found: false,
            };
            use_collector.visit_type(return_type);
            if use_collector.found {
                return Err(format!(
                    "generic cpp_name function `{}` cannot mention its type parameter in the return type",
                    sig.ident
                ));
            }
        }
    }
    if sig.constness.is_some()
        || sig.asyncness.is_some()
        || sig.abi.is_some()
        || sig.variadic.is_some()
    {
        return Err(format!(
            "cpp_name function `{}` must be an ordinary Rust free function",
            sig.ident
        ));
    }
    if sig.unsafety.is_some() && !matches!(function.vis, syn::Visibility::Public(_)) {
        return Err(format!(
            "unsafe cpp_name function `{}` must be public",
            sig.ident
        ));
    }
    for input in &sig.inputs {
        let syn::FnArg::Typed(input) = input else {
            return Err(format!(
                "cpp_name function `{}` cannot have a receiver",
                sig.ident
            ));
        };
        if !matches!(input.pat.as_ref(), syn::Pat::Ident(_)) {
            return Err(format!(
                "cpp_name function `{}` requires simple identifier parameters",
                sig.ident
            ));
        }
    }
    Ok(())
}

pub(crate) fn validate_cpp_name_companion_attrs(attrs: &[Attribute]) -> Result<(), String> {
    fn is_exact_unsafe_code_allow(attr: &Attribute) -> bool {
        let Meta::List(list) = &attr.meta else {
            return false;
        };
        if !is_simple_path(&list.path, "allow") {
            return false;
        }
        let parser = Punctuated::<syn::Path, Token![,]>::parse_terminated;
        parser.parse2(list.tokens.clone()).is_ok_and(|paths| {
            paths.len() == 1
                && paths
                    .first()
                    .is_some_and(|path| is_simple_path(path, "unsafe_code"))
        })
    }

    for attr in attrs {
        if parse_marker_attr(attr)?.is_some()
            || attr.path().is_ident("doc")
            || is_exact_unsafe_code_allow(attr)
        {
            continue;
        }
        return Err(format!(
            "cpp_name functions support only doc attributes and exact #[allow(unsafe_code)] in addition to the name marker; found `{}`",
            attr.path().to_token_stream()
        ));
    }
    Ok(())
}

fn item_decl_name(item: &Item) -> Option<String> {
    match item {
        Item::Const(item) => Some(semantic_ident(&item.ident)),
        Item::Enum(item) => Some(semantic_ident(&item.ident)),
        Item::ExternCrate(item) => Some(
            item.rename
                .as_ref()
                .map(|(_, ident)| semantic_ident(ident))
                .unwrap_or_else(|| semantic_ident(&item.ident)),
        ),
        Item::Fn(item) => Some(semantic_ident(&item.sig.ident)),
        Item::Macro(item) => item.ident.as_ref().map(semantic_ident),
        Item::Mod(item) => Some(semantic_ident(&item.ident)),
        Item::Static(item) => Some(semantic_ident(&item.ident)),
        Item::Struct(item) => Some(semantic_ident(&item.ident)),
        Item::Trait(item) => Some(semantic_ident(&item.ident)),
        Item::TraitAlias(item) => Some(semantic_ident(&item.ident)),
        Item::Type(item) => Some(semantic_ident(&item.ident)),
        Item::Union(item) => Some(semantic_ident(&item.ident)),
        _ => None,
    }
}

fn collect_use_bindings(
    tree: &syn::UseTree,
    parent_tail: Option<&str>,
    out: &mut Vec<String>,
) -> bool {
    match tree {
        syn::UseTree::Name(name) => {
            if name.ident == "self" {
                if let Some(parent) = parent_tail {
                    out.push(parent.to_string());
                }
            } else {
                out.push(semantic_ident(&name.ident));
            }
            false
        }
        syn::UseTree::Rename(rename) => {
            out.push(semantic_ident(&rename.rename));
            false
        }
        syn::UseTree::Path(path) => {
            let tail = semantic_ident(&path.ident);
            collect_use_bindings(&path.tree, Some(&tail), out)
        }
        syn::UseTree::Group(group) => group.items.iter().fold(false, |glob, tree| {
            collect_use_bindings(tree, parent_tail, out) || glob
        }),
        syn::UseTree::Glob(_) => true,
    }
}

fn collect_use_binding_origins(
    tree: &syn::UseTree,
    prefix: &mut Vec<String>,
    out: &mut BTreeMap<String, BTreeSet<Vec<String>>>,
) {
    match tree {
        syn::UseTree::Name(name) => {
            let source = semantic_ident(&name.ident);
            if source == "self" {
                if let Some(local) = prefix.last().cloned() {
                    out.entry(local).or_default().insert(prefix.clone());
                }
            } else {
                let mut origin = prefix.clone();
                origin.push(source.clone());
                out.entry(source).or_default().insert(origin);
            }
        }
        syn::UseTree::Rename(rename) => {
            let mut origin = prefix.clone();
            origin.push(semantic_ident(&rename.ident));
            out.entry(semantic_ident(&rename.rename))
                .or_default()
                .insert(origin);
        }
        syn::UseTree::Path(path) => {
            prefix.push(semantic_ident(&path.ident));
            collect_use_binding_origins(&path.tree, prefix, out);
            prefix.pop();
        }
        syn::UseTree::Group(group) => {
            for tree in &group.items {
                collect_use_binding_origins(tree, prefix, out);
            }
        }
        syn::UseTree::Glob(_) => {}
    }
}

struct ShadowCollector<'a> {
    forbidden: &'a BTreeSet<String>,
    allow_crate_wide_builtins: bool,
    allow_cpp_inherit: bool,
    error: Option<String>,
}

impl ShadowCollector<'_> {
    fn check(&mut self, name: String, context: &str) {
        if self.error.is_none() && self.forbidden.contains(&name) {
            self.error = Some(format!(
                "source {context} `{name}` shadows a cpp_name source or C++ function name"
            ));
        }
    }
}

impl<'ast> Visit<'ast> for ShadowCollector<'_> {
    fn visit_attribute(&mut self, attr: &'ast Attribute) {
        fn mentions_forbidden(
            tokens: proc_macro2::TokenStream,
            forbidden: &BTreeSet<String>,
        ) -> bool {
            tokens.into_iter().any(|token| match token {
                proc_macro2::TokenTree::Ident(ident) => forbidden.contains(&semantic_ident(&ident)),
                proc_macro2::TokenTree::Group(group) => {
                    mentions_forbidden(group.stream(), forbidden)
                }
                _ => false,
            })
        }
        if self.error.is_none() && mentions_forbidden(attr.meta.to_token_stream(), self.forbidden) {
            self.error = Some(
                "attribute metadata mentions a cpp_name source or target identifier and could bypass shadow validation"
                    .to_string(),
            );
        }
        let audited = attribute_is_audited_inert(attr)
            || attribute_is_exact_inactive_cpp_trait_member_dispatch(attr)
            || attribute_is_exact_inactive_transpiler_marker(attr)
            || (self.allow_cpp_inherit && attribute_is_authenticated_cpp_inherit(attr))
            || (self.allow_crate_wide_builtins
                && attribute_is_audited_crate_wide(attr, self.allow_cpp_inherit));
        if self.error.is_none() && !audited {
            self.error = Some(format!(
                "unaudited attribute `{}` is not supported in a file containing cpp_name because attribute or derive macro expansion can add hidden root items",
                attr.meta.to_token_stream()
            ));
        }
        visit::visit_attribute(self, attr);
    }

    fn visit_item_fn(&mut self, function: &'ast syn::ItemFn) {
        self.check(semantic_ident(&function.sig.ident), "nested function");
        visit::visit_item_fn(self, function);
    }

    fn visit_item_const(&mut self, item: &'ast syn::ItemConst) {
        self.check(semantic_ident(&item.ident), "nested const");
        visit::visit_item_const(self, item);
    }

    fn visit_item_static(&mut self, item: &'ast syn::ItemStatic) {
        self.check(semantic_ident(&item.ident), "nested static");
        visit::visit_item_static(self, item);
    }

    fn visit_item_type(&mut self, item: &'ast syn::ItemType) {
        self.check(semantic_ident(&item.ident), "nested type alias");
        visit::visit_item_type(self, item);
    }

    fn visit_item_struct(&mut self, item: &'ast syn::ItemStruct) {
        self.check(semantic_ident(&item.ident), "nested struct");
        visit::visit_item_struct(self, item);
    }

    fn visit_item_enum(&mut self, item: &'ast syn::ItemEnum) {
        self.check(semantic_ident(&item.ident), "nested enum");
        visit::visit_item_enum(self, item);
    }

    fn visit_item_union(&mut self, item: &'ast syn::ItemUnion) {
        self.check(semantic_ident(&item.ident), "nested union");
        visit::visit_item_union(self, item);
    }

    fn visit_item_trait(&mut self, item: &'ast syn::ItemTrait) {
        self.check(semantic_ident(&item.ident), "nested trait");
        visit::visit_item_trait(self, item);
    }

    fn visit_item_trait_alias(&mut self, item: &'ast syn::ItemTraitAlias) {
        self.check(semantic_ident(&item.ident), "nested trait alias");
        visit::visit_item_trait_alias(self, item);
    }

    fn visit_item_mod(&mut self, item: &'ast syn::ItemMod) {
        self.check(semantic_ident(&item.ident), "nested module");
        visit::visit_item_mod(self, item);
    }

    fn visit_item_macro(&mut self, item: &'ast syn::ItemMacro) {
        // `--expand` is deliberately unavailable when cpp_name provenance is
        // active.  An item-position invocation (`include!`, a derive helper,
        // or any other expansion macro) could therefore add a root function,
        // alias, import, or shadow that neither this preflight nor overload
        // canonicalization can see.  A macro_rules definition is inert until
        // an invocation (which is checked separately), so keep definitions
        // available for expression/statement helpers.
        let is_macro_rules_definition =
            item.ident.is_some() && item.mac.path.is_ident("macro_rules");
        if !is_macro_rules_definition {
            self.error.get_or_insert_with(|| {
                format!(
                    "unexpanded item macro `{}` is not supported in a file containing cpp_name because it can bypass overload ownership and shadow validation",
                    item.mac.path.to_token_stream()
                )
            });
        }
        if let Some(ident) = &item.ident {
            if (self.allow_crate_wide_builtins
                && audited_builtin_or_attribute_name(&semantic_ident(ident)))
                || is_cpp_trait_member_dispatch_name(&semantic_ident(ident))
            {
                self.error.get_or_insert_with(|| {
                    format!(
                        "source macro definition `{}` shadows an audited compiler-owned macro in a crate containing cpp_name",
                        semantic_ident(ident)
                    )
                });
            }
            self.check(semantic_ident(ident), "nested macro");
        }
        visit::visit_item_macro(self, item);
    }

    fn visit_item_extern_crate(&mut self, item: &'ast syn::ItemExternCrate) {
        if semantic_ident(&item.ident) == "self" {
            self.error.get_or_insert_with(|| {
                "aliasing `extern crate self` is not supported in a file containing cpp_name because it can bypass root-call rewriting"
                    .to_string()
            });
        }
        let local = item
            .rename
            .as_ref()
            .map(|(_, ident)| semantic_ident(ident))
            .unwrap_or_else(|| semantic_ident(&item.ident));
        if (self.allow_crate_wide_builtins && audited_builtin_or_attribute_name(&local))
            || is_cpp_trait_member_dispatch_name(&local)
        {
            self.error.get_or_insert_with(|| {
                format!(
                    "extern-crate binding `{local}` shadows an audited compiler-owned macro in a crate containing cpp_name"
                )
            });
        }
        visit::visit_item_extern_crate(self, item);
    }

    fn visit_pat_ident(&mut self, pattern: &'ast syn::PatIdent) {
        self.check(semantic_ident(&pattern.ident), "binding");
        visit::visit_pat_ident(self, pattern);
    }

    fn visit_const_param(&mut self, param: &'ast syn::ConstParam) {
        self.check(semantic_ident(&param.ident), "const generic parameter");
        visit::visit_const_param(self, param);
    }

    fn visit_type_param(&mut self, param: &'ast syn::TypeParam) {
        self.check(semantic_ident(&param.ident), "type parameter");
        visit::visit_type_param(self, param);
    }

    fn visit_field(&mut self, field: &'ast syn::Field) {
        if let Some(ident) = &field.ident {
            self.check(semantic_ident(ident), "field");
        }
        visit::visit_field(self, field);
    }

    fn visit_impl_item_fn(&mut self, method: &'ast syn::ImplItemFn) {
        self.check(semantic_ident(&method.sig.ident), "method");
        visit::visit_impl_item_fn(self, method);
    }

    fn visit_trait_item_fn(&mut self, method: &'ast syn::TraitItemFn) {
        self.check(semantic_ident(&method.sig.ident), "trait method");
        visit::visit_trait_item_fn(self, method);
    }

    fn visit_foreign_item_fn(&mut self, function: &'ast syn::ForeignItemFn) {
        self.check(semantic_ident(&function.sig.ident), "foreign function");
        visit::visit_foreign_item_fn(self, function);
    }

    fn visit_variant(&mut self, variant: &'ast syn::Variant) {
        self.check(semantic_ident(&variant.ident), "enum variant");
        visit::visit_variant(self, variant);
    }

    fn visit_item_use(&mut self, item: &'ast syn::ItemUse) {
        fn aliases_rust_root(tree: &syn::UseTree) -> bool {
            match tree {
                syn::UseTree::Rename(rename) => {
                    matches!(
                        semantic_ident(&rename.ident).as_str(),
                        "crate" | "self" | "super"
                    )
                }
                syn::UseTree::Path(path) => aliases_rust_root(&path.tree),
                syn::UseTree::Group(group) => group.items.iter().any(aliases_rust_root),
                syn::UseTree::Name(_) | syn::UseTree::Glob(_) => false,
            }
        }
        if aliases_rust_root(&item.tree) {
            self.error.get_or_insert_with(|| {
                "aliasing crate/self/super is not supported in a file containing cpp_name because it can bypass root-call rewriting"
                    .to_string()
            });
        }
        fn forbidden_path_segment(
            tree: &syn::UseTree,
            forbidden: &BTreeSet<String>,
        ) -> Option<String> {
            match tree {
                syn::UseTree::Name(name) => {
                    let name = semantic_ident(&name.ident);
                    forbidden.contains(&name).then_some(name)
                }
                syn::UseTree::Rename(rename) => {
                    let source = semantic_ident(&rename.ident);
                    forbidden.contains(&source).then_some(source)
                }
                syn::UseTree::Path(path) => {
                    let segment = semantic_ident(&path.ident);
                    forbidden
                        .contains(&segment)
                        .then_some(segment)
                        .or_else(|| forbidden_path_segment(&path.tree, forbidden))
                }
                syn::UseTree::Group(group) => group
                    .items
                    .iter()
                    .find_map(|tree| forbidden_path_segment(tree, forbidden)),
                syn::UseTree::Glob(_) => None,
            }
        }
        if let Some(segment) = forbidden_path_segment(&item.tree, self.forbidden) {
            self.error.get_or_insert_with(|| {
                format!(
                    "import path mentions cpp_name source or target identifier `{segment}` and could bypass call rewriting"
                )
            });
        }
        let mut bindings = Vec::new();
        if collect_use_bindings(&item.tree, None, &mut bindings) {
            self.error.get_or_insert_with(|| {
                "glob imports are not supported in a file containing cpp_name because they can shadow renamed functions"
                    .to_string()
            });
        }
        for binding in bindings {
            if (self.allow_crate_wide_builtins && audited_builtin_or_attribute_name(&binding))
                || is_cpp_trait_member_dispatch_name(&binding)
            {
                self.error.get_or_insert_with(|| {
                    format!(
                        "import binding `{binding}` can shadow an audited compiler-owned macro in a crate containing cpp_name"
                    )
                });
            }
            self.check(binding, "import binding");
        }
        visit::visit_item_use(self, item);
    }

    fn visit_macro(&mut self, mac: &'ast syn::Macro) {
        fn tokens_mention_forbidden(
            tokens: proc_macro2::TokenStream,
            forbidden: &BTreeSet<String>,
        ) -> bool {
            tokens.into_iter().any(|token| match token {
                proc_macro2::TokenTree::Ident(ident) => forbidden.contains(&semantic_ident(&ident)),
                proc_macro2::TokenTree::Group(group) => {
                    tokens_mention_forbidden(group.stream(), forbidden)
                }
                _ => false,
            })
        }

        if self.error.is_none()
            && (mac
                .path
                .segments
                .iter()
                .any(|segment| self.forbidden.contains(&semantic_ident(&segment.ident)))
                || tokens_mention_forbidden(mac.tokens.clone(), self.forbidden))
        {
            self.error = Some(
                "opaque macro tokens mention a cpp_name source or target identifier and could bypass shadow validation"
                    .to_string(),
            );
            return;
        }

        // A `macro_rules! name { ... }` definition is inert until invoked, so
        // its fully visible tokens can be audited above. Invocations are
        // opaque: an expression `include!` or external function-like proc
        // macro can synthesize a call to a renamed root function without
        // mentioning either identity in its source tokens. A local wrapper is
        // no safer because it can invoke that external macro transitively.
        // The owning file rejects every invocation. Marker-free siblings keep
        // only explicitly shadow-checked compiler builtins whose lowerings are
        // implemented by this transpiler, and their arguments cannot nest an
        // unaudited macro.
        let audited_compiler_builtin = self.allow_crate_wide_builtins
            && (is_simple_path(&mac.path, "assert") || is_simple_path(&mac.path, "format"))
            && !token_stream_contains_unaudited_nested_macro(mac.tokens.clone());
        if self.error.is_none()
            && !mac.path.is_ident("macro_rules")
            && !audited_compiler_builtin
        {
            self.error = Some(format!(
                "unexpanded macro invocation `{}` is not supported in a file containing cpp_name because it can synthesize hidden calls, items, or types",
                mac.path.to_token_stream()
            ));
        }
        // Do not interpret opaque tokens as Rust AST.
    }
}

/// Apply the opaque-expansion and identifier-shadow audit to one complete
/// source unit. Exact cpp_name marker attributes on root functions are skipped
/// because `collect` validates their grammar separately; every other
/// attribute and every macro invocation is handled by `ShadowCollector`.
fn audit_source_unit(
    file: &syn::File,
    forbidden: &BTreeSet<String>,
    allow_crate_wide_builtins: bool,
    allow_cpp_inherit: bool,
) -> Result<(), String> {
    let mut collector = ShadowCollector {
        forbidden,
        allow_crate_wide_builtins,
        allow_cpp_inherit,
        error: None,
    };
    for attr in &file.attrs {
        collector.visit_attribute(attr);
    }
    for item in &file.items {
        match item {
            // Root free-function names are checked separately so genuine
            // overload groups remain possible. Visit their complete
            // signatures and bodies, and skip only the exact inert marker.
            Item::Fn(function) => {
                for attr in &function.attrs {
                    if !attribute_is_marker(attr) {
                        collector.visit_attribute(attr);
                    }
                }
                collector.visit_signature(&function.sig);
                collector.visit_block(&function.block);
            }
            other => collector.visit_item(other),
        }
    }
    collector.error.map_or(Ok(()), Err)
}

fn source_imports_audited_cpp_inherit(file: &syn::File) -> bool {
    fn meta_is_or_contains_cpp_inherit(meta: &Meta) -> bool {
        if is_simple_path(meta.path(), "cpp_inherit") {
            return true;
        }
        let Meta::List(list) = meta else {
            return false;
        };
        if !is_simple_path(&list.path, "cfg_attr") {
            return false;
        }
        let parser = Punctuated::<Meta, Token![,]>::parse_terminated;
        parser.parse2(list.tokens.clone()).is_ok_and(|nested| {
            nested
                .iter()
                .skip(1)
                .any(meta_is_or_contains_cpp_inherit)
        })
    }

    fn attribute_is_or_contains_cpp_inherit(attribute: &Attribute) -> bool {
        meta_is_or_contains_cpp_inherit(&attribute.meta)
    }

    fn direct_item_cpp_inherit_count(item: &Item) -> usize {
        let attrs: &[Attribute] = match item {
            Item::Const(item) => &item.attrs,
            Item::Enum(item) => &item.attrs,
            Item::ExternCrate(item) => &item.attrs,
            Item::Fn(item) => &item.attrs,
            Item::ForeignMod(item) => &item.attrs,
            Item::Impl(item) => &item.attrs,
            Item::Macro(item) => &item.attrs,
            Item::Mod(item) => &item.attrs,
            Item::Static(item) => &item.attrs,
            Item::Struct(item) => &item.attrs,
            Item::Trait(item) => &item.attrs,
            Item::TraitAlias(item) => &item.attrs,
            Item::Type(item) => &item.attrs,
            Item::Union(item) => &item.attrs,
            Item::Use(item) => &item.attrs,
            Item::Verbatim(_) => &[],
            _ => &[],
        };
        attrs
            .iter()
            .filter(|attribute| attribute_is_or_contains_cpp_inherit(attribute))
            .count()
    }

    fn scope_has_exact_cpp_inherit_import(items: &[Item]) -> bool {
        let mut origins = BTreeMap::<String, BTreeSet<Vec<String>>>::new();
        let mut competing_declaration = false;
        for item in items {
            match item {
                Item::Use(item) => {
                    collect_use_binding_origins(&item.tree, &mut Vec::new(), &mut origins);
                }
                Item::ExternCrate(item) => {
                    let source = semantic_ident(&item.ident);
                    let local = item
                        .rename
                        .as_ref()
                        .map(|(_, ident)| semantic_ident(ident))
                        .unwrap_or_else(|| source.clone());
                    origins.entry(local).or_default().insert(vec![source]);
                }
                _ => {}
            }
            competing_declaration |= item_decl_name(item)
                .is_some_and(|name| matches!(name.as_str(), "rusty" | "cpp_inherit"));
        }
        !competing_declaration
            && !origins.contains_key("rusty")
            && origins.get("cpp_inherit").is_some_and(|bindings| {
                bindings.len() == 1
                    && bindings.contains(&vec![
                        "rusty".to_string(),
                        "cpp_inherit".to_string(),
                    ])
            })
    }

    fn audit_module_scope(items: &[Item], scoped_count: &mut usize) -> bool {
        let exact_import = scope_has_exact_cpp_inherit_import(items);
        for item in items {
            let direct_count = direct_item_cpp_inherit_count(item);
            if direct_count != 0 && !exact_import {
                return false;
            }
            *scoped_count += direct_count;
            if let Item::Mod(module) = item
                && let Some((_, nested_items)) = &module.content
                && !audit_module_scope(nested_items, scoped_count)
            {
                return false;
            }
        }
        true
    }

    #[derive(Default)]
    struct AllCppInheritAttributes {
        count: usize,
    }
    impl<'ast> Visit<'ast> for AllCppInheritAttributes {
        fn visit_attribute(&mut self, attribute: &'ast Attribute) {
            if attribute_is_or_contains_cpp_inherit(attribute) {
                self.count += 1;
            }
            visit::visit_attribute(self, attribute);
        }
    }

    let mut all = AllCppInheritAttributes::default();
    all.visit_file(file);
    let mut scoped_count = 0usize;
    let scopes_are_exact = audit_module_scope(&file.items, &mut scoped_count);
    all.count != 0 && scopes_are_exact && scoped_count == all.count
}

/// Fail closed around source-level aliases that the deliberately small
/// overload canonicalizer does not resolve semantically.
///
/// Code generation recursively expands ordinary crate-root `type` aliases and
/// compares the final mapped C++ parameter strings.  A Rust `use`/re-export,
/// `extern crate ... as ...`, or projection through a source-local module can
/// hide another alias layer from that canonicalizer, however.  The same is
/// true at any depth inside a tuple, reference, function pointer, generic
/// argument, qself projection, array length, or other type form.  Reject only
/// when such a binding is actually reachable from a participating function's
/// parameter type; unrelated imports remain supported.
fn validate_parameter_type_provenance(file: &syn::File, plan: &CppNamePlan) -> Result<(), String> {
    let mut root_import_bindings = BTreeMap::<String, BTreeSet<Vec<String>>>::new();
    let mut root_modules = BTreeSet::<String>::new();
    let mut root_aliases = BTreeMap::<String, syn::Type>::new();

    for item in &file.items {
        match item {
            Item::Use(item) => {
                collect_use_binding_origins(&item.tree, &mut Vec::new(), &mut root_import_bindings);
            }
            Item::ExternCrate(item) => {
                let source = semantic_ident(&item.ident);
                let local = item
                    .rename
                    .as_ref()
                    .map(|(_, ident)| semantic_ident(ident))
                    .unwrap_or_else(|| source.clone());
                root_import_bindings
                    .entry(local)
                    .or_default()
                    .insert(vec![source]);
            }
            Item::Mod(item) => {
                root_modules.insert(semantic_ident(&item.ident));
            }
            Item::Type(item) => {
                root_aliases.insert(semantic_ident(&item.ident), (*item.ty).clone());
            }
            _ => {}
        }
    }

    struct ParameterProvenance<'a> {
        function: &'a str,
        root_import_bindings: &'a BTreeMap<String, BTreeSet<Vec<String>>>,
        root_modules: &'a BTreeSet<String>,
        root_aliases: &'a BTreeMap<String, syn::Type>,
        alias_stack: Vec<String>,
        error: Option<String>,
    }

    impl ParameterProvenance<'_> {
        fn reject(&mut self, path: &syn::Path, reason: String) {
            if self.error.is_none() {
                self.error = Some(format!(
                    "cpp_name cannot prove overload identity for function `{}` parameter type path `{}`: {reason}",
                    self.function,
                    path.to_token_stream()
                ));
            }
        }

        fn root_path_segments(path: &syn::Path) -> Vec<String> {
            let mut segments = path
                .segments
                .iter()
                .map(|segment| semantic_ident(&segment.ident))
                .collect::<Vec<_>>();
            if segments
                .first()
                .is_some_and(|segment| matches!(segment.as_str(), "crate" | "self"))
            {
                segments.remove(0);
            }
            segments
        }

        fn root_alias_name(path: &syn::Path) -> Option<String> {
            let segments = Self::root_path_segments(path);
            (segments.len() == 1).then(|| segments[0].clone())
        }

        fn import_binding_is_compiler_owned(&self, binding: &str) -> bool {
            fn check(
                binding: &str,
                imports: &BTreeMap<String, BTreeSet<Vec<String>>>,
                modules: &BTreeSet<String>,
                visiting: &mut BTreeSet<String>,
            ) -> bool {
                if !visiting.insert(binding.to_string()) {
                    return false;
                }
                let result = imports.get(binding).is_some_and(|origins| {
                    !origins.is_empty()
                        && origins.iter().all(|origin| {
                            let Some(root) = origin.first() else {
                                return false;
                            };
                            if matches!(root.as_str(), "crate" | "self" | "super")
                                || modules.contains(root)
                            {
                                return false;
                            }
                            // Source ownership always wins over a spelling
                            // that normally denotes the compiler/runtime
                            // prelude. `#![no_std] mod std { ... }` and local
                            // `core`/`alloc`/`rusty` modules are all valid Rust;
                            // treating their re-exports as external would hide
                            // local type aliases from collision proof.
                            if matches!(root.as_str(), "std" | "core" | "alloc" | "rusty") {
                                return true;
                            }
                            imports
                                .contains_key(root)
                                .then(|| check(root, imports, modules, visiting))
                                .unwrap_or(false)
                        })
                });
                visiting.remove(binding);
                result
            }
            check(
                binding,
                self.root_import_bindings,
                self.root_modules,
                &mut BTreeSet::new(),
            )
        }
    }

    impl<'ast> Visit<'ast> for ParameterProvenance<'_> {
        fn visit_path(&mut self, path: &'ast syn::Path) {
            if self.error.is_some() {
                return;
            }
            let segments = Self::root_path_segments(path);
            if let Some(first) = segments.first() {
                if self.root_import_bindings.contains_key(first)
                    && !self.import_binding_is_compiler_owned(first)
                {
                    self.reject(
                        path,
                        format!(
                            "root binding `{first}` comes from use/re-export or extern-crate alias"
                        ),
                    );
                    return;
                }
                if segments.len() > 1 && self.root_modules.contains(first) {
                    self.reject(
                        path,
                        format!(
                            "projection through source-local module `{first}` is outside the checked root-alias canonicalizer"
                        ),
                    );
                    return;
                }
            }

            // Follow every reachable non-generic root alias target for the
            // provenance audit.  Generic/conditional/cyclic aliases retain
            // the existing stricter codegen rejection, but their target paths
            // still cannot conceal an import or local-module projection here.
            if let Some(alias_name) = Self::root_alias_name(path)
                && let Some(target) = self.root_aliases.get(&alias_name)
                && !self.alias_stack.contains(&alias_name)
            {
                self.alias_stack.push(alias_name);
                self.visit_type(target);
                self.alias_stack.pop();
                if self.error.is_some() {
                    return;
                }
            }
            visit::visit_path(self, path);
        }

        fn visit_macro(&mut self, mac: &'ast syn::Macro) {
            if self.error.is_none() {
                self.error = Some(format!(
                    "cpp_name cannot prove overload identity for function `{}` through opaque parameter type macro `{}`",
                    self.function,
                    mac.path.to_token_stream()
                ));
            }
        }
    }

    let targets = plan.target_names();
    for item in &file.items {
        let Item::Fn(function) = item else {
            continue;
        };
        let rust_name = semantic_ident(&function.sig.ident);
        let emitted_name = plan.function_name(&rust_name).unwrap_or(&rust_name);
        if !targets.contains(emitted_name) {
            continue;
        }
        let mut provenance = ParameterProvenance {
            function: &rust_name,
            root_import_bindings: &root_import_bindings,
            root_modules: &root_modules,
            root_aliases: &root_aliases,
            alias_stack: Vec::new(),
            error: None,
        };
        for input in &function.sig.inputs {
            let syn::FnArg::Typed(input) = input else {
                continue;
            };
            provenance.visit_type(&input.ty);
            if provenance.error.is_some() {
                break;
            }
        }
        if let Some(error) = provenance.error {
            return Err(error);
        }
    }
    Ok(())
}

fn validate_shadowing(
    file: &syn::File,
    plan: &CppNamePlan,
    allow_crate_wide_builtins: bool,
    allow_cpp_inherit: bool,
) -> Result<(), String> {
    let targets = plan.target_names();
    let rust_names = plan.rust_names();
    let forbidden: BTreeSet<String> = targets.union(&rust_names).cloned().collect();

    let mut root_function_counts = BTreeMap::<String, usize>::new();
    for item in &file.items {
        if let Item::Fn(function) = item {
            *root_function_counts
                .entry(semantic_ident(&function.sig.ident))
                .or_default() += 1;
        }
    }
    if let Some((name, count)) = root_function_counts
        .iter()
        .find(|(name, count)| forbidden.contains(*name) && **count != 1)
    {
        return Err(format!(
            "cpp_name identity `{name}` has {count} root free-function declarations; every Rust identity must be unique"
        ));
    }

    fn check_item_names(
        items: &[Item],
        depth: usize,
        targets: &BTreeSet<String>,
        marked: &BTreeSet<String>,
        forbidden: &BTreeSet<String>,
    ) -> Result<(), String> {
        for item in items {
            if let Some(name) = item_decl_name(item)
                && forbidden.contains(&name)
            {
                let allowed_root_function = depth == 0
                    && matches!(item, Item::Fn(_))
                    && (targets.contains(&name) || marked.contains(&name));
                if !allowed_root_function {
                    return Err(format!(
                        "source item `{name}` shadows a cpp_name source or C++ function name"
                    ));
                }
            }
            if let Item::Mod(module) = item
                && let Some((_, nested)) = &module.content
            {
                check_item_names(nested, depth + 1, targets, marked, forbidden)?;
            }
            if let Item::ForeignMod(foreign) = item {
                for foreign_item in &foreign.items {
                    if let syn::ForeignItem::Fn(function) = foreign_item {
                        let name = semantic_ident(&function.sig.ident);
                        if forbidden.contains(&name) {
                            return Err(format!(
                                "foreign function `{name}` shadows a cpp_name source or C++ function name"
                            ));
                        }
                    }
                }
            }
        }
        Ok(())
    }

    check_item_names(&file.items, 0, &targets, &rust_names, &forbidden)?;
    audit_source_unit(
        file,
        &forbidden,
        allow_crate_wide_builtins,
        allow_cpp_inherit,
    )?;

    // A target that is another marked function's Rust identity makes a call
    // spelling depend on rename order.  Refuse chains and cycles outright.
    for (rust_name, cpp_name) in &plan.functions {
        if rust_name != cpp_name && rust_names.contains(cpp_name) {
            return Err(format!(
                "cpp_name target `{cpp_name}` shadows another marked Rust function name"
            ));
        }
    }
    Ok(())
}

/// Collect the marker plan and perform all syntax/provenance/shadow checks.
pub(crate) fn collect(file: &syn::File) -> Result<CppNamePlan, String> {
    collect_with_provenance(file, false, false)
}

pub(crate) fn collect_with_crate_provenance(
    file: &syn::File,
    allow_cpp_inherit: bool,
) -> Result<CppNamePlan, String> {
    collect_with_provenance(file, true, allow_cpp_inherit)
}

fn collect_with_provenance(
    file: &syn::File,
    allow_crate_wide_builtins: bool,
    allow_cpp_inherit: bool,
) -> Result<CppNamePlan, String> {
    reject_descendant_marker(&file.attrs, "crate-level attributes", |collector, attrs| {
        for attr in attrs {
            collector.visit_attribute(attr);
        }
    })?;

    let mut plan = CppNamePlan::default();
    for item in &file.items {
        match item {
            Item::Fn(function) => {
                let mut target = None;
                for attr in &function.attrs {
                    if let Some(parsed) = parse_marker_attr(attr)?
                        && target.replace(parsed).is_some()
                    {
                        return Err(format!(
                            "duplicate cpp_name marker attributes on `{}`",
                            function.sig.ident
                        ));
                    }
                }
                reject_descendant_marker(
                    &function.sig,
                    "root free-function signature",
                    |collector, signature| collector.visit_signature(signature),
                )?;
                reject_descendant_marker(
                    &function.block,
                    "root free-function body",
                    |collector, block| collector.visit_block(block),
                )?;
                if let Some(target) = target {
                    validate_cpp_name_companion_attrs(&function.attrs)?;
                    validate_cpp_name_function_shape(function)?;
                    let rust_name = semantic_ident(&function.sig.ident);
                    if plan.functions.insert(rust_name.clone(), target).is_some() {
                        return Err(format!("duplicate Rust free function `{rust_name}`"));
                    }
                }
            }
            other => {
                reject_descendant_marker(other, "non-root-function item", |collector, item| {
                    collector.visit_item(item)
                })?
            }
        }
    }
    if !plan.is_empty() {
        validate_shadowing(
            file,
            &plan,
            allow_crate_wide_builtins,
            allow_cpp_inherit,
        )?;
        validate_parameter_type_provenance(file, &plan)?;
    }
    Ok(plan)
}

pub(crate) fn source_mentions_reserved_marker(source: &str) -> bool {
    match syn::parse_file(source) {
        Ok(file) => match collect(&file) {
            Ok(plan) => !plan.is_empty(),
            Err(_) => true,
        },
        Err(_) => source
            .parse::<proc_macro2::TokenStream>()
            .ok()
            .is_some_and(token_stream_mentions_marker),
    }
}

/// B: every namespace-scope item name a source unit DECLARES (recursing into
/// inline modules). Used by the crate-wide audit to tell a cross-file
/// REFERENCE — which now resolves to the owner's audited C++ identity — from
/// SHADOWING, which still rejects.
fn declared_item_names_of_source(source: &str) -> Result<BTreeSet<String>, String> {
    let file = syn::parse_file(source).map_err(|error| error.to_string())?;
    fn walk(items: &[syn::Item], out: &mut BTreeSet<String>) {
        for item in items {
            match item {
                syn::Item::Fn(f) => {
                    out.insert(semantic_ident(&f.sig.ident));
                }
                syn::Item::Struct(s) => {
                    out.insert(semantic_ident(&s.ident));
                }
                syn::Item::Enum(e) => {
                    out.insert(semantic_ident(&e.ident));
                }
                syn::Item::Trait(t) => {
                    out.insert(semantic_ident(&t.ident));
                }
                syn::Item::Type(t) => {
                    out.insert(semantic_ident(&t.ident));
                }
                syn::Item::Const(c) => {
                    out.insert(semantic_ident(&c.ident));
                }
                syn::Item::Static(st) => {
                    out.insert(semantic_ident(&st.ident));
                }
                syn::Item::Mod(m) => {
                    out.insert(semantic_ident(&m.ident));
                    if let Some((_, nested)) = &m.content {
                        walk(nested, out);
                    }
                }
                _ => {}
            }
        }
    }
    let mut out = BTreeSet::new();
    walk(&file.items, &mut out);
    Ok(out)
}

/// B: the crate-wide (Rust name -> audited C++ name) map, so a file that
/// CALLS a sibling's renamed item can emit the owner's identity. Collected
/// with the same authenticated `collect` the owner's own plan uses.
pub(crate) fn crate_wide_function_targets(
    sources: &[(std::path::PathBuf, String)],
) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    for (_, source) in sources {
        if !source_mentions_reserved_marker(source) {
            continue;
        }
        let Ok(file) = syn::parse_file(source) else {
            continue;
        };
        let allow_cpp_inherit = source_imports_audited_cpp_inherit(&file);
        let Ok(plan) = collect_with_crate_provenance(&file, allow_cpp_inherit) else {
            continue;
        };
        for rust_name in plan.rust_names() {
            if let Some(target) = plan.function_name(&rust_name) {
                out.insert(rust_name, target.to_string());
            }
        }
    }
    out
}

/// Crate mode emits each source file as a separate named module.  One overload
/// set must therefore be owned by exactly one source file; cross-module pieces
/// cannot see one another's declarations reliably.
pub(crate) fn preflight_crate_sources(
    sources: &[(std::path::PathBuf, String)],
) -> Result<bool, String> {
    preflight_crate_sources_with_cpp_inherit_provenance(sources, false)
}

/// Crate-mode cpp_inherit is the sole opaque attribute allowed beside an
/// active cpp_name contract. Its spelling is insufficient: the caller must
/// have authenticated the exact local `rusty` facade -> inert proc-macro
/// dependency chain before this audit begins.
pub(crate) fn preflight_crate_sources_with_cpp_inherit_provenance(
    sources: &[(std::path::PathBuf, String)],
    trusted_cpp_inherit_provenance: bool,
) -> Result<bool, String> {
    let mut owner_by_target = BTreeMap::<String, std::path::PathBuf>::new();
    let mut owner_by_identity = BTreeMap::<String, std::path::PathBuf>::new();
    let mut any = false;
    for (path, source) in sources {
        // Preserve the historical crate preflight for marker-free inputs.  In
        // particular, do not make this narrow feature's `syn` parser a new
        // gate for source that the existing expansion/lowering path accepts.
        if !source_mentions_reserved_marker(source) {
            continue;
        }
        let file = syn::parse_file(source)
            .map_err(|error| format!("{}: cpp_name parse error: {error}", path.display()))?;
        let allow_cpp_inherit = trusted_cpp_inherit_provenance
            && source_imports_audited_cpp_inherit(&file);
        let plan = collect_with_crate_provenance(&file, allow_cpp_inherit)
            .map_err(|error| format!("{}: {error}", path.display()))?;
        if plan.is_empty() {
            continue;
        }
        any = true;
        for identity in plan.rust_names().into_iter().chain(plan.target_names()) {
            owner_by_identity
                .entry(identity)
                .or_insert_with(|| path.clone());
        }
        for target in plan.target_names() {
            if let Some(first) = owner_by_target.insert(target.clone(), path.clone())
                && first != *path
            {
                return Err(format!(
                    "cpp_name target `{target}` is split across source files {} and {}; one named module must own the complete overload set",
                    first.display(),
                    path.display()
                ));
            }
        }
    }

    // A source-owned name can be called from any sibling module. Once this
    // narrow ABI feature is active, an opaque expansion in a marker-free
    // sibling can synthesize that call without spelling either the Rust or C++
    // identity in the auditable source tokens. Apply the same no-expansion
    // invariant to every collected source unit before output creation. Crates
    // with no cpp_name marker retain the historical expansion behavior.
    if any {
        let no_forbidden_identities = BTreeSet::new();
        for (path, source) in sources {
            let file = syn::parse_file(source).map_err(|error| {
                format!(
                    "{}: cpp_name crate-wide expansion audit could not parse source: {error}",
                    path.display()
                )
            })?;
            audit_source_unit(
                &file,
                &no_forbidden_identities,
                true,
                trusted_cpp_inherit_provenance && source_imports_audited_cpp_inherit(&file),
            )
            .map_err(|error| {
                format!(
                    "{}: cpp_name crate-wide audit failed: {error}",
                    path.display()
                )
            })?;
        }
    }

    // Each per-file codegen plan can rewrite only paths whose Rust identity it
    // owns. A sibling-file mention would otherwise silently retain the Rust
    // spelling at the C++ call site. Reserve both sides of every mapping in
    // all other source files and fail before emitting anything.
    for (path, source) in sources {
        let forbidden = owner_by_identity
            .iter()
            .filter_map(|(identity, owner)| (owner != path).then_some(identity.clone()))
            .collect::<BTreeSet<_>>();
        if forbidden.is_empty() {
            continue;
        }
        let tokens = source
            .parse::<proc_macro2::TokenStream>()
            .map_err(|error| {
                format!(
                    "{}: cpp_name could not audit cross-file identities: {error}",
                    path.display()
                )
            })?;
        fn find_forbidden(
            tokens: proc_macro2::TokenStream,
            forbidden: &BTreeSet<String>,
        ) -> Option<String> {
            tokens.into_iter().find_map(|token| match token {
                proc_macro2::TokenTree::Ident(ident) => {
                    let ident = semantic_ident(&ident);
                    forbidden.contains(&ident).then_some(ident)
                }
                proc_macro2::TokenTree::Group(group) => find_forbidden(group.stream(), forbidden),
                _ => None,
            })
        }
        // B: a cross-file MENTION is not automatically a violation. A call to
        // a renamed sibling item resolves to the owner's C++ identity and
        // emits the owner's audited name (the caller's emission is rewritten;
        // see CodeGen::cpp_name_call_target and the crate-wide
        // foreign-target map). What the audit must still forbid is
        // SHADOWING: a non-owner file that DECLARES the identity, because
        // then the same spelling names two different entities and the call
        // site's rewrite would silently retarget it.
        if let Some(identity) = find_forbidden(tokens, &forbidden) {
            let declared = declared_item_names_of_source(source).map_err(|error| {
                format!(
                    "{}: cpp_name could not audit cross-file identities: {error}",
                    path.display()
                )
            })?;
            let shadowed = owner_by_identity
                .iter()
                .filter(|(_, owner)| *owner != path)
                .map(|(identity, _)| identity)
                .find(|identity| declared.contains(*identity));
            if let Some(identity) = shadowed {
                let owner = &owner_by_identity[identity];
                return Err(format!(
                    "cpp_name source or C++ identity `{identity}` owned by {} is also DECLARED in {}; shadowing an audited identity is not supported",
                    owner.display(),
                    path.display()
                ));
            }
            let _ = identity;
        }
    }
    Ok(any)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plan(source: &str) -> Result<CppNamePlan, String> {
        collect(&syn::parse_file(source).expect("test source parses"))
    }

    #[test]
    fn accepts_exact_inert_root_function_marker() {
        let plan = plan(
            r#"
            #[cfg_attr(any(), cpp_name(overloaded))]
            fn rust_name(value: i32) -> i32 { value }
            "#,
        )
        .unwrap();
        assert_eq!(plan.function_name("rust_name"), Some("overloaded"));
    }

    #[test]
    fn accepts_only_narrow_generic_root_function_shape() {
        let accepted = plan(
            r#"
            #[cfg_attr(any(), cpp_name(make_proxy))]
            fn make_default<T: 'static>() -> i32 { 7 }
            #[cfg_attr(any(), cpp_name(make_proxy))]
            fn make_copy<T: 'static>(_value: &T) -> i32 { 9 }
            "#,
        )
        .expect("one `'static` type parameter is the supported generic lane");
        assert_eq!(accepted.function_name("make_default"), Some("make_proxy"));
        assert_eq!(accepted.function_name("make_copy"), Some("make_proxy"));

        for source in [
            "#[cfg_attr(any(), cpp_name(make_proxy))] fn f<T, U>() {}",
            "#[cfg_attr(any(), cpp_name(make_proxy))] fn f<'a>() {}",
            "#[cfg_attr(any(), cpp_name(make_proxy))] fn f<const N: usize>() {}",
            "#[cfg_attr(any(), cpp_name(make_proxy))] fn f<T: Clone>() {}",
            "#[cfg_attr(any(), cpp_name(make_proxy))] fn f<T>() where T: Clone {}",
            "#[cfg_attr(any(), cpp_name(make_proxy))] fn f<T>() -> T { panic!() }",
        ] {
            assert!(
                plan(source).is_err(),
                "accepted unsupported generic cpp_name shape: {source}"
            );
        }
    }

    #[test]
    fn accepts_public_unsafe_root_functions_but_rejects_other_unsafe_placements_and_forms() {
        let accepted = plan(
            r#"
            #[cfg_attr(any(), cpp_name(make_proxy))]
            #[allow(unsafe_code)]
            pub unsafe fn make_buffer(value: *mut i32) -> i32 { *value }
            #[cfg_attr(any(), cpp_name(make_proxy))]
            #[allow(unsafe_code)]
            pub unsafe fn make_fd(value: *mut bool) -> i32 { *value as i32 }
            "#,
        )
        .expect("public unsafe root free functions are supported");
        assert_eq!(accepted.function_name("make_buffer"), Some("make_proxy"));
        assert_eq!(accepted.function_name("make_fd"), Some("make_proxy"));

        for source in [
            "#[cfg_attr(any(), cpp_name(make_proxy))] unsafe fn private(value: *mut i32) {}",
            "struct Host; impl Host { #[cfg_attr(any(), cpp_name(make_proxy))] pub unsafe fn method(value: *mut i32) {} }",
            "trait Host { #[cfg_attr(any(), cpp_name(make_proxy))] unsafe fn method(value: *mut i32); }",
            "extern \"C\" { #[cfg_attr(any(), cpp_name(make_proxy))] fn foreign(value: *mut i32); }",
            "#[cfg_attr(any(), cpp_name(make_proxy))] pub unsafe extern \"C\" fn abi(value: *mut i32) {}",
            "#[cfg_attr(any(), cpp_name(make_proxy))] #[allow(dead_code)] pub unsafe fn broad_allow(value: *mut i32) {}",
            "#[cfg_attr(any(), cpp_name(make_proxy))] #[allow(unsafe_code, dead_code)] pub unsafe fn mixed_allow(value: *mut i32) {}",
        ] {
            assert!(
                plan(source).is_err(),
                "accepted unsupported unsafe cpp_name placement/form: {source}"
            );
        }
    }

    #[test]
    fn rejects_active_malformed_duplicate_and_qualified_markers() {
        for source in [
            "#[cpp_name(x)] fn f() {}",
            "#[cfg_attr(any(), cpp_name)] fn f() {}",
            "#[cfg_attr(any(), cpp_name(x), allow(dead_code))] fn f() {}",
            "#[cfg_attr(all(), cpp_name(x))] fn f() {}",
            "#[cfg_attr(any(), crate::cpp_name(x))] fn f() {}",
            "#[cfg_attr(any(), cpp_name(foo::bar))] fn f() {}",
            "#[cfg_attr(any(), cpp_name(foo, bar))] fn f() {}",
            "#[cfg_attr(any(), cpp_name(x))] #[cfg_attr(any(), cpp_name(x))] fn f() {}",
        ] {
            assert!(plan(source).is_err(), "accepted invalid marker: {source}");
        }
    }

    #[test]
    fn rejects_non_function_nested_and_opaque_macro_placements() {
        for source in [
            "#[cfg_attr(any(), cpp_name(x))] struct S;",
            "mod m { #[cfg_attr(any(), cpp_name(x))] fn f() {} }",
            "fn outer() { #[cfg_attr(any(), cpp_name(x))] fn f() {} }",
            "struct S; impl S { #[cfg_attr(any(), cpp_name(x))] fn f() {} }",
            "macro_rules! make { () => { #[cfg_attr(any(), cpp_name(x))] fn f() {} } }",
            "fn f() { cpp_name!(); }",
        ] {
            assert!(
                plan(source).is_err(),
                "accepted invalid placement: {source}"
            );
        }
    }

    #[test]
    fn rejects_cpp_keywords_reserved_and_non_ascii_targets() {
        for source in [
            "#[cfg_attr(any(), cpp_name(r#type))] fn f() {}",
            "#[cfg_attr(any(), cpp_name(concept))] fn f() {}",
            "#[cfg_attr(any(), cpp_name(__hidden))] fn f() {}",
            "#[cfg_attr(any(), cpp_name(_Hidden))] fn f() {}",
            "#[cfg_attr(any(), cpp_name(_hidden))] fn f() {}",
            "#[cfg_attr(any(), cpp_name(hidden__name))] fn f() {}",
            "#[cfg_attr(any(), cpp_name(main))] fn f() {}",
            "#[cfg_attr(any(), cpp_name(std))] fn f() {}",
            "#[cfg_attr(any(), cpp_name(rusty))] fn f() {}",
            "#[cfg_attr(any(), cpp_name(café))] fn f() {}",
            "#[cfg_attr(any(), cpp_name(valid_name))] fn r#type() {}",
        ] {
            assert!(plan(source).is_err(), "accepted reserved target: {source}");
        }
    }

    #[test]
    fn rejects_shadowing_and_glob_imports() {
        for source in [
            "#[cfg_attr(any(), cpp_name(overloaded))] fn f(overloaded: i32) {}",
            "#[cfg_attr(any(), cpp_name(overloaded))] fn f() { let overloaded = 1; }",
            "use somewhere::overloaded; #[cfg_attr(any(), cpp_name(overloaded))] fn f() {}",
            "use crate::f as renamed; #[cfg_attr(any(), cpp_name(overloaded))] fn f() {}",
            "use crate as root_alias; #[cfg_attr(any(), cpp_name(overloaded))] fn f() {}",
            "use somewhere::*; #[cfg_attr(any(), cpp_name(overloaded))] fn f() {}",
            "const overloaded: i32 = 1; #[cfg_attr(any(), cpp_name(overloaded))] fn f() {}",
            "mod overloaded {} #[cfg_attr(any(), cpp_name(overloaded))] fn f() {}",
            "struct S { overloaded: i32 } #[cfg_attr(any(), cpp_name(overloaded))] fn f() {}",
            "macro_rules! use_name { () => { overloaded } } #[cfg_attr(any(), cpp_name(overloaded))] fn f() {}",
            "fn helper() { overloaded!(); } #[cfg_attr(any(), cpp_name(overloaded))] fn f() {}",
            "#[some_attr(overloaded)] struct S; #[cfg_attr(any(), cpp_name(overloaded))] fn f() {}",
            "fn overloaded(value: i32) {} fn overloaded(value: bool) {} #[cfg_attr(any(), cpp_name(overloaded))] fn f() {}",
            "#[cfg_attr(any(), cpp_name(overloaded))] fn f() { fn overloaded() {} }",
            "fn helper<const overloaded: i32>() {} #[cfg_attr(any(), cpp_name(overloaded))] fn f() {}",
        ] {
            assert!(plan(source).is_err(), "accepted shadowing source: {source}");
        }
    }

    #[test]
    fn rejects_import_bound_and_local_module_parameter_type_projections_recursively() {
        for source in [
            r#"
                mod types { pub type A = i32; pub type B = i32; }
                pub use types::{A, B};
                #[cfg_attr(any(), cpp_name(overloaded))]
                fn first(value: A) -> i32 { value }
                #[cfg_attr(any(), cpp_name(overloaded))]
                fn second(value: B) -> i32 { value }
            "#,
            r#"
                mod types { pub type A = i32; pub type B = i32; }
                use types as left;
                use types as right;
                #[cfg_attr(any(), cpp_name(overloaded))]
                fn first(value: left::A) -> i32 { value }
                #[cfg_attr(any(), cpp_name(overloaded))]
                fn second(value: right::B) -> i32 { value }
            "#,
            r#"
                mod types { pub type A = i32; pub type B = i32; }
                pub use types::{A, B};
                #[cfg_attr(any(), cpp_name(overloaded))]
                fn first(value: (A, bool)) -> i32 { value.0 }
                #[cfg_attr(any(), cpp_name(overloaded))]
                fn second(value: (B, bool)) -> i32 { value.0 }
            "#,
            r#"
                mod types { pub type A = i32; pub type B = i32; }
                pub use types::{A, B};
                #[cfg_attr(any(), cpp_name(overloaded))]
                fn first(value: fn(A) -> i32) -> i32 { value(1) }
                #[cfg_attr(any(), cpp_name(overloaded))]
                fn second(value: fn(B) -> i32) -> i32 { value(1) }
            "#,
            r#"
                mod types { pub type A = i32; pub type B = i32; }
                type Left = types::A;
                type Right = types::B;
                #[cfg_attr(any(), cpp_name(overloaded))]
                fn first(value: Left) -> i32 { value }
                #[cfg_attr(any(), cpp_name(overloaded))]
                fn second(value: Right) -> i32 { value }
            "#,
        ] {
            let error = plan(source).expect_err("accepted unprovable parameter projection");
            assert!(
                error.contains("cannot prove overload identity"),
                "unexpected provenance diagnostic: {error}"
            );
        }

        for root in ["rusty", "std", "core", "alloc"] {
            let source = format!(
                r#"
                    #![no_std]
                    mod {root} {{ pub type A = i32; pub type B = i32; }}
                    use {root}::{{A, B}};
                    #[cfg_attr(any(), cpp_name(overloaded))]
                    fn first(value: A) -> i32 {{ value }}
                    #[cfg_attr(any(), cpp_name(overloaded))]
                    fn second(value: B) -> i32 {{ value }}
                "#
            );
            let error =
                plan(&source).expect_err(&format!("accepted source-owned `{root}` module aliases"));
            assert!(
                error.contains("cannot prove overload identity"),
                "unexpected local {root} diagnostic: {error}"
            );
        }
    }

    #[test]
    fn unrelated_import_aliases_and_compiler_owned_std_imports_remain_accepted() {
        let source = r#"
            use std::sync::Arc;
            mod types { pub type A = i32; pub type B = i32; }
            pub use types::{A, B};
            type Wrapped = Arc<i32>;
            #[cfg_attr(any(), cpp_name(overloaded))]
            fn first(value: Wrapped) -> i32 { *value }
            #[cfg_attr(any(), cpp_name(overloaded))]
            fn second(value: bool) -> i32 { value as i32 }
        "#;
        assert!(plan(source).is_ok());
    }

    #[test]
    fn rejects_every_macro_invocation_but_allows_inert_uninvoked_definitions() {
        let include_source = r#"
            include!("hidden.rs");
            #[cfg_attr(any(), cpp_name(overloaded))]
            fn renamed(value: i32) -> i32 { value }
        "#;
        let error = plan(include_source).expect_err("accepted unexpanded include item");
        assert!(error.contains("unexpanded item macro"), "{error}");

        for source in [
            r#"
                #[cfg_attr(any(), cpp_name(overloaded))]
                fn first(value: i32) -> i32 { include!("hidden_expr.rs") }
            "#,
            r#"
                fn route() -> i32 { call_hidden!() }
                #[cfg_attr(any(), cpp_name(overloaded))]
                fn first(value: i32) -> i32 { value }
            "#,
            r#"
                macro_rules! wrapper { () => { call_hidden!() }; }
                fn route() -> i32 { wrapper!() }
                #[cfg_attr(any(), cpp_name(overloaded))]
                fn first(value: i32) -> i32 { value }
            "#,
            r#"
                macro_rules! one { () => { 1 }; }
                #[cfg_attr(any(), cpp_name(overloaded))]
                fn first(value: i32) -> i32 { value + one!() }
            "#,
        ] {
            let error = plan(source).expect_err("accepted opaque macro invocation");
            assert!(error.contains("unexpanded macro invocation"), "{error}");
        }

        let unused_definition = r#"
            macro_rules! one { () => { 1 }; }
            #[cfg_attr(any(), cpp_name(overloaded))]
            fn first(value: i32) -> i32 { value }
            #[cfg_attr(any(), cpp_name(overloaded))]
            fn second(value: bool) -> i32 { value as i32 }
        "#;
        assert!(plan(unused_definition).is_ok());
    }

    #[test]
    fn rejects_opaque_attribute_and_derive_expansion_but_allows_audited_inert_attrs() {
        for source in [
            r#"
                #[derive(Clone)]
                struct Host;
                #[cfg_attr(any(), cpp_name(overloaded))]
                fn renamed(value: i32) -> i32 { value }
            "#,
            r#"
                #[make_overloaded]
                struct Host;
                #[cfg_attr(any(), cpp_name(overloaded))]
                fn renamed(value: i32) -> i32 { value }
            "#,
            r#"
                #[cfg_attr(not(any()), make_overloaded)]
                struct Host;
                #[cfg_attr(any(), cpp_name(overloaded))]
                fn renamed(value: i32) -> i32 { value }
            "#,
        ] {
            let error = plan(source).expect_err("accepted opaque attribute expansion");
            assert!(error.contains("unaudited attribute"), "{error}");
        }

        let inert_source = r#"
            #![allow(dead_code)]
            /// Kept as an inert companion item.
            #[warn(unused_variables)]
            struct Host;
            #[cfg_attr(any(), cpp_name(overloaded))]
            fn first(value: i32) -> i32 { value }
            #[cfg_attr(any(), cpp_name(overloaded))]
            fn second(value: bool) -> i32 { value as i32 }
        "#;
        assert!(plan(inert_source).is_ok());
    }

    #[test]
    fn accepts_only_exact_inactive_cpp_trait_member_dispatch_without_shadowing() {
        let accepted = r#"
            #[cfg_attr(any(), cpp_trait_member_dispatch)]
            trait SinkBase { fn write(&mut self); }
            #[cfg_attr(any(), cpp_trait_member_dispatch)]
            trait SourceBase { fn read(&mut self); }
            #[cfg_attr(any(), cpp_name(make_proxy))]
            fn first(value: i32) -> i32 { value }
            #[cfg_attr(any(), cpp_name(make_proxy))]
            fn second(value: bool) -> i32 { value as i32 }
        "#;
        assert!(plan(accepted).is_ok());

        for source in [
            "#[cpp_trait_member_dispatch] trait T {}\n#[cfg_attr(any(), cpp_name(make_proxy))] fn first(value: i32) {}",
            "#[cfg_attr(all(), cpp_trait_member_dispatch)] trait T {}\n#[cfg_attr(any(), cpp_name(make_proxy))] fn first(value: i32) {}",
            "#[cfg_attr(any(), cpp_trait_member_dispatch(extra))] trait T {}\n#[cfg_attr(any(), cpp_name(make_proxy))] fn first(value: i32) {}",
            "#[cfg_attr(any(), maker::cpp_trait_member_dispatch)] trait T {}\n#[cfg_attr(any(), cpp_name(make_proxy))] fn first(value: i32) {}",
            "#[cfg_attr(any(), cpp_trait_member_dispatch, allow(dead_code))] trait T {}\n#[cfg_attr(any(), cpp_name(make_proxy))] fn first(value: i32) {}",
            "use maker::cpp_trait_member_dispatch;\n#[cfg_attr(any(), cpp_trait_member_dispatch)] trait T {}\n#[cfg_attr(any(), cpp_name(make_proxy))] fn first(value: i32) {}",
            "macro_rules! cpp_trait_member_dispatch { () => {} }\n#[cfg_attr(any(), cpp_trait_member_dispatch)] trait T {}\n#[cfg_attr(any(), cpp_name(make_proxy))] fn first(value: i32) {}",
        ] {
            assert!(
                plan(source).is_err(),
                "accepted malformed, active, or shadowed trait dispatch marker: {source}"
            );
        }
    }

    #[test]
    fn strings_comments_and_longer_identifiers_do_not_trigger_marker_probe() {
        let source = r#"
            #[doc = "cpp_name is documentation"]
            fn my_cpp_name_helper() {
                let cpp_name_suffix = "cpp_name";
                my_cpp_name!();
            }
        "#;
        assert!(!source_mentions_reserved_marker(source));
        assert!(plan(source).unwrap().is_empty());
    }

    #[test]
    fn crate_preflight_rejects_cross_file_identity_mentions() {
        // B: a cross-file CALL is a reference to the owner's identity, not a
        // violation — it resolves to the owner's audited C++ name at the call
        // site (see CodeGen::cpp_name_call_target and the crate-wide map).
        let sources = vec![
            (
                std::path::PathBuf::from("owner.rs"),
                "#[cfg_attr(any(), cpp_name(overloaded))] fn rust_name(value: i32) {}".to_string(),
            ),
            (
                std::path::PathBuf::from("caller.rs"),
                "fn caller() { crate::owner::rust_name(1); }".to_string(),
            ),
        ];
        preflight_crate_sources(&sources)
            .expect("a cross-file call to an audited identity resolves to its owner");

        // SHADOWING still rejects: a non-owner file that DECLARES the identity
        // makes one spelling name two entities, and the call-site rewrite
        // would silently retarget it.
        let shadowed = vec![
            (
                std::path::PathBuf::from("owner.rs"),
                "#[cfg_attr(any(), cpp_name(overloaded))] fn rust_name(value: i32) {}".to_string(),
            ),
            (
                std::path::PathBuf::from("shadow.rs"),
                "fn rust_name(value: bool) {}".to_string(),
            ),
        ];
        let error = preflight_crate_sources(&shadowed)
            .expect_err("shadowing an audited identity must reject");
        assert!(error.contains("shadowing an audited identity"), "{error}");

        // Shadowing the C++ TARGET side rejects for the same reason.
        let shadowed_target = vec![
            (
                std::path::PathBuf::from("owner.rs"),
                "#[cfg_attr(any(), cpp_name(overloaded))] fn rust_name(value: i32) {}".to_string(),
            ),
            (
                std::path::PathBuf::from("shadow.rs"),
                "fn overloaded(value: bool) {}".to_string(),
            ),
        ];
        assert!(
            preflight_crate_sources(&shadowed_target)
                .expect_err("shadowing the C++ target must reject")
                .contains("shadowing an audited identity")
        );

        let split_overload = vec![
            (
                std::path::PathBuf::from("first.rs"),
                "#[cfg_attr(any(), cpp_name(overloaded))] fn first(value: i32) {}".to_string(),
            ),
            (
                std::path::PathBuf::from("second.rs"),
                "#[cfg_attr(any(), cpp_name(overloaded))] fn second(value: bool) {}".to_string(),
            ),
        ];
        assert!(preflight_crate_sources(&split_overload).is_err());
    }

    #[test]
    fn crate_preflight_audits_opaque_expansion_in_marker_free_siblings_only_when_active() {
        let owner = (
            std::path::PathBuf::from("lib.rs"),
            "#[cfg_attr(any(), cpp_name(overloaded))] fn renamed(value: i32) -> i32 { value }"
                .to_string(),
        );
        for (label, owner_source, sibling_source, expected) in [
            (
                "expression include",
                owner.1.clone(),
                "fn route_hidden() -> i32 { include!(\"hidden.inc\") }".to_string(),
                "unexpanded macro invocation",
            ),
            (
                "local wrapper to external proc macro",
                format!(
                    "macro_rules! wrapper {{ () => {{ maker::call_hidden!() }}; }}\n{}",
                    owner.1
                ),
                "fn route_hidden() -> i32 { wrapper!() }".to_string(),
                "unexpanded macro invocation",
            ),
            (
                "procedural attribute",
                owner.1.clone(),
                "#[make_route] struct Host;".to_string(),
                "unaudited attribute",
            ),
        ] {
            let sources = vec![
                (owner.0.clone(), owner_source),
                (std::path::PathBuf::from("child.rs"), sibling_source),
            ];
            let error = preflight_crate_sources(&sources)
                .expect_err(&format!("accepted marker-free sibling {label}"));
            assert!(
                error.contains(expected),
                "unexpected {label} diagnostic: {error}"
            );
        }

        let marker_free = vec![(
            std::path::PathBuf::from("lib.rs"),
            "fn route_hidden() -> i32 { include!(\"hidden.inc\") }".to_string(),
        )];
        assert_eq!(preflight_crate_sources(&marker_free), Ok(false));

        let audited_builtins = vec![
            owner.clone(),
            (
                std::path::PathBuf::from("child.rs"),
                r#"
                    use rusty::cpp_inherit;
                    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
                    #[repr(C)]
                    #[cfg_attr(any(), cpp_ctor)]
                    struct Value(i32);
                    trait Marker {}
                    #[cpp_inherit]
                    impl Marker for Value {}
                    fn check(value: Value) {
                        assert!(value == value);
                        let _message = format!("{}", 1);
                    }
                    #[no_mangle]
                    pub unsafe extern "C" fn fiber_task_entry_thunk() {}
                "#
                .to_string(),
            ),
        ];
        assert_eq!(
            preflight_crate_sources_with_cpp_inherit_provenance(&audited_builtins, true),
            Ok(true)
        );
        assert!(preflight_crate_sources(&audited_builtins).is_err());

        let nested_serializable_shape = vec![
            owner.clone(),
            (
                std::path::PathBuf::from("serializable.rs"),
                r#"
                    trait SerializableBase {}
                    struct Holder<T>(T);
                    mod details {
                        use super::{Holder, SerializableBase};
                        use rusty::cpp_inherit;
                        #[cpp_inherit]
                        impl<T: SerializableBase> SerializableBase for Holder<T> {}
                    }
                "#
                .to_string(),
            ),
        ];
        assert_eq!(
            preflight_crate_sources_with_cpp_inherit_provenance(
                &nested_serializable_shape,
                true,
            ),
            Ok(true)
        );
        assert!(
            preflight_crate_sources_with_cpp_inherit_provenance(
                &nested_serializable_shape,
                false,
            )
            .is_err()
        );

        let same_file_serializable_shape = vec![(
            std::path::PathBuf::from("serializable.rs"),
            r#"
                #[cfg_attr(any(), cpp_name(make_sink_proxy))]
                pub fn make_value(value: i32) -> i32 { value }
                #[cfg_attr(any(), cpp_name(make_sink_proxy))]
                pub fn make_flag(value: bool) -> i32 { value as i32 }
                mod details {
                    use rusty::cpp_inherit;
                    trait SerializableBase {}
                    struct SerializableSharedPtrHolder;
                    #[cfg_attr(any(), thread_local)]
                    static LAST_REPORT_US: u64 = 0;
                    #[cpp_inherit]
                    impl SerializableBase for SerializableSharedPtrHolder {}
                }
            "#
            .to_string(),
        )];
        assert_eq!(
            preflight_crate_sources_with_cpp_inherit_provenance(
                &same_file_serializable_shape,
                true,
            ),
            Ok(true)
        );

        for unsafe_thread_local in [
            "#[thread_local] static VALUE: u64 = 0;",
            "#[cfg_attr(all(), thread_local)] static VALUE: u64 = 0;",
            "#[cfg_attr(any(), maker::thread_local)] static VALUE: u64 = 0;",
        ] {
            let sources = vec![
                owner.clone(),
                (
                    std::path::PathBuf::from("reactor.rs"),
                    unsafe_thread_local.to_string(),
                ),
            ];
            assert!(
                preflight_crate_sources_with_cpp_inherit_provenance(&sources, true).is_err(),
                "accepted non-inert thread_local spelling: {unsafe_thread_local}"
            );
        }

        for unaudited_no_mangle in [
            "#[no_mangle(extra)] pub unsafe extern \"C\" fn thunk() {}",
            "#[maker::no_mangle] pub unsafe extern \"C\" fn thunk() {}",
        ] {
            let sources = vec![
                owner.clone(),
                (
                    std::path::PathBuf::from("reactor.rs"),
                    unaudited_no_mangle.to_string(),
                ),
            ];
            assert!(
                preflight_crate_sources_with_cpp_inherit_provenance(&sources, true).is_err(),
                "accepted inexact no_mangle spelling: {unaudited_no_mangle}"
            );
        }

        for nested_spoof in [
            "mod details { use maker::cpp_inherit; trait T {} struct S; #[cpp_inherit] impl T for S {} }",
            "mod details { mod rusty { pub use maker::cpp_inherit; } use rusty::cpp_inherit; trait T {} struct S; #[cpp_inherit] impl T for S {} }",
            "use rusty::cpp_inherit; mod details { use maker::cpp_inherit; trait T {} struct S; #[cpp_inherit] impl T for S {} }",
        ] {
            let sources = vec![
                owner.clone(),
                (
                    std::path::PathBuf::from("serializable.rs"),
                    nested_spoof.to_string(),
                ),
            ];
            assert!(
                preflight_crate_sources_with_cpp_inherit_provenance(&sources, true).is_err(),
                "accepted nested cpp_inherit spoof: {nested_spoof}"
            );
        }

        for sibling in [
            "use maker::Clone; #[derive(Clone)] struct Value;",
            "fn check() { assert!(external!()); }",
            "use maker::repr; #[repr(C)] struct Value;",
            "use maker::cpp_inherit; trait T {} struct S; #[cpp_inherit] impl T for S {}",
            "mod rusty { pub use maker::cpp_inherit; } use rusty::cpp_inherit; trait T {} struct S; #[cpp_inherit] impl T for S {}",
            "#[cfg_attr(not(any()), cpp_ctor)] struct Value;",
        ] {
            let sources = vec![
                owner.clone(),
                (std::path::PathBuf::from("child.rs"), sibling.to_string()),
            ];
            assert!(
                preflight_crate_sources(&sources).is_err(),
                "accepted shadowed or nested opaque builtin: {sibling}"
            );
        }
    }
}
