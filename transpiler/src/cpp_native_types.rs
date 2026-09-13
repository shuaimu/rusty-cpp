//! Explicit bindings to types declared by a native C header.
//!
//! Rust retains the repr(C) definition for its FFI boundary. The checked type
//! map supplies a C typedef or tag name; C++ includes the owning header and
//! does not emit a second definition of the Rust binding.
use crate::types::UserTypeMap;
use quote::ToTokens;
use std::collections::BTreeMap;
use syn::visit::Visit;
use syn::{Attribute, Item, Meta, Token, Type};

const MARKER: &str = "cpp_native_type";

fn marker_count(tokens: proc_macro2::TokenStream) -> usize {
    tokens
        .into_iter()
        .map(|token| match token {
            proc_macro2::TokenTree::Ident(ident) => usize::from(ident == MARKER),
            proc_macro2::TokenTree::Group(group) => marker_count(group.stream()),
            _ => 0,
        })
        .sum()
}

fn mentions(attr: &Attribute) -> bool {
    marker_count(attr.to_token_stream()) != 0
}

pub(crate) fn has_marker(attrs: &[Attribute]) -> bool {
    attrs.iter().any(mentions)
}

fn exact_marker(attr: &Attribute) -> bool {
    if !attr.path().is_ident("cfg_attr") {
        return false;
    }
    let Ok(args) =
        attr.parse_args_with(syn::punctuated::Punctuated::<Meta, Token![,]>::parse_terminated)
    else {
        return false;
    };
    if args.len() != 2 || args.trailing_punct() {
        return false;
    }
    matches!(&args[0], Meta::List(list) if list.path.is_ident("any") && list.tokens.is_empty())
        && matches!(&args[1], Meta::Path(path) if path.is_ident(MARKER))
}

fn has_c_repr(attrs: &[Attribute]) -> bool {
    attrs.iter().any(|attr| {
        attr.path().is_ident("repr")
            && attr
                .parse_args::<syn::Path>()
                .is_ok_and(|path| path.is_ident("C"))
    })
}

fn c_identifier(name: &str) -> bool {
    let name = name.strip_prefix("::").unwrap_or(name);
    let mut chars = name.chars();
    chars
        .next()
        .is_some_and(|c| c == '_' || c.is_ascii_alphabetic())
        && chars.all(|c| c == '_' || c.is_ascii_alphanumeric())
        && !matches!(
            name,
            "void" | "bool" | "char" | "int" | "long" | "short" | "float" | "double" | "auto"
        )
}

#[derive(Default)]
pub(crate) struct NativeTypes {
    /// Keys are source paths within the current Rust file, before C++ mapping.
    types: BTreeMap<String, Option<String>>,
}

impl NativeTypes {
    pub(crate) fn mapped_file_pointer(&self, ty: &Type) -> bool {
        let Type::Ptr(pointer) = ty else {
            return false;
        };
        if pointer.mutability.is_none() {
            return false;
        }
        let Type::Path(path) = pointer.elem.as_ref() else {
            return false;
        };
        if path.qself.is_some()
            || path.path.leading_colon.is_some()
            || path.path.segments.iter().any(|s| !s.arguments.is_none())
        {
            return false;
        }
        let name = path
            .path
            .segments
            .iter()
            .map(|s| s.ident.to_string())
            .collect::<Vec<_>>()
            .join("::");
        self.types.get(&name).is_some_and(|mapped| {
            mapped
                .as_deref()
                .is_none_or(|name| name.trim_start_matches("::") == "FILE")
        })
    }
}

/// Validate shapes even during the crate's preflight before the map is loaded.
/// Exact emission must call this again with the configured map.
pub(crate) fn collect(
    file: &syn::File,
    type_map: Option<&UserTypeMap>,
) -> Result<NativeTypes, String> {
    fn walk(
        items: &[Item],
        scope: &mut Vec<String>,
        map: Option<&UserTypeMap>,
        plan: &mut NativeTypes,
    ) -> Result<usize, String> {
        let mut count = 0;
        for item in items {
            if let Item::Mod(module) = item {
                if let Some((_, children)) = &module.content {
                    scope.push(module.ident.to_string());
                    count += walk(children, scope, map, plan)?;
                    scope.pop();
                }
            }
            let Item::Struct(value) = item else {
                continue;
            };
            if !has_marker(&value.attrs) {
                continue;
            }
            let markers = value
                .attrs
                .iter()
                .filter(|attr| mentions(attr))
                .collect::<Vec<_>>();
            if markers.len() != 1 || !exact_marker(markers[0]) {
                return Err(format!(
                    "{MARKER} requires exactly #[cfg_attr(any(), {MARKER})]"
                ));
            }
            if value.attrs.iter().any(|attr| {
                !mentions(attr)
                    && ![
                        "repr", "cfg", "doc", "allow", "warn", "deny", "forbid", "expect",
                    ]
                    .iter()
                    .any(|name| attr.path().is_ident(name))
            }) {
                return Err(format!(
                    "{MARKER} bindings may carry only native layout, cfg, and documentation/lint attributes"
                ));
            }
            if !has_c_repr(&value.attrs)
                || !value.generics.params.is_empty()
                || value.generics.where_clause.is_some()
            {
                return Err(format!("{MARKER} requires a nongeneric #[repr(C)] struct"));
            }
            if !matches!(value.fields, syn::Fields::Named(_)) {
                return Err(format!(
                    "{MARKER} requires named C fields (opaque pointer bindings may use a private zero-length field)"
                ));
            }
            let name = value.ident.to_string();
            let qualified = scope
                .iter()
                .chain(std::iter::once(&name))
                .cloned()
                .collect::<Vec<_>>()
                .join("::");
            let mapped = if let Some(map) = map {
                let target = map
                    .lookup(&qualified)
                    .or_else(|| map.lookup(&name))
                    .ok_or_else(|| {
                        format!("{MARKER} `{qualified}` requires an explicit native C type map")
                    })?;
                if !c_identifier(target) {
                    return Err(format!(
                        "{MARKER} `{qualified}` requires a C typedef/tag identifier, found `{target}`"
                    ));
                }
                Some(target.to_string())
            } else {
                None
            };
            plan.types.insert(qualified, mapped);
            count += 1;
        }
        Ok(count)
    }
    let mut plan = NativeTypes::default();
    let count = walk(&file.items, &mut Vec::new(), type_map, &mut plan)?;
    if count != marker_count(file.to_token_stream()) {
        return Err(format!(
            "{MARKER} is reserved for the exact inert attribute on native repr(C) structs"
        ));
    }
    // Rust methods and trait implementations are not native C declarations.
    struct Impls<'a> {
        plan: &'a NativeTypes,
        invalid: bool,
    }
    impl<'ast> Visit<'ast> for Impls<'_> {
        fn visit_item_impl(&mut self, item: &'ast syn::ItemImpl) {
            if let Type::Path(path) = item.self_ty.as_ref() {
                let name = path
                    .path
                    .segments
                    .iter()
                    .map(|s| s.ident.to_string())
                    .collect::<Vec<_>>()
                    .join("::");
                self.invalid |= self.plan.types.contains_key(&name);
            }
            syn::visit::visit_item_impl(self, item);
        }
    }
    let mut implementations = Impls {
        plan: &plan,
        invalid: false,
    };
    implementations.visit_file(file);
    if implementations.invalid {
        return Err(format!(
            "{MARKER} bindings cannot define Rust inherent or trait implementations"
        ));
    }
    Ok(plan)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn map(name: &str, target: &str) -> UserTypeMap {
        let mut result = UserTypeMap::default();
        result.mappings.insert(name.to_string(), target.to_string());
        result
    }
    #[test]
    fn native_file_requires_explicit_struct_and_mapping() {
        let file = syn::parse_file(
            "#[repr(C)] #[cfg_attr(any(), cpp_native_type)] pub struct CFile { _opaque: [u8; 0] }",
        )
        .unwrap();
        let plan = collect(&file, Some(&map("CFile", "FILE"))).unwrap();
        assert!(plan.mapped_file_pointer(&syn::parse_quote!(*mut CFile)));
        assert!(!plan.mapped_file_pointer(&syn::parse_quote!(*const CFile)));
        assert!(collect(&file, Some(&UserTypeMap::default())).is_err());
        assert!(collect(&file, Some(&map("CFile", "rusty::File"))).is_err());
    }
    #[test]
    fn invalid_native_contracts_fail_closed() {
        for source in [
            "#[cpp_native_type] #[repr(C)] struct Native { x: i32 }",
            "#[cfg_attr(all(), cpp_native_type)] #[repr(C)] struct Native { x: i32 }",
            "#[cfg_attr(any(), cpp_native_type)] struct Native { x: i32 }",
            "#[cfg_attr(any(), cpp_native_type)] #[repr(C)] struct Native<T> { x: T }",
            "#[cfg_attr(any(), cpp_native_type)] fn native() {}",
            "#[cfg_attr(any(), cpp_native_type)] #[repr(C)] #[derive(Clone)] struct Native { x: i32 }",
            "macro_rules! make { () => { cpp_native_type } }",
            "#[cfg_attr(any(), cpp_native_type)] #[repr(C)] struct Native { x: i32 } impl Native { fn new() -> Self { Self {x:1} } }",
        ] {
            let file = syn::parse_file(source).unwrap();
            assert!(
                collect(&file, Some(&map("Native", "native_t"))).is_err(),
                "{source}"
            );
        }
    }
}
