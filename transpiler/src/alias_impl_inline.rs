//! Inherent impls on type aliases of crate-local nominal types.
//!
//! Rust treats `impl<K, V> Root<K, V> { … }` with
//! `type Root<K, V> = NodeRef<marker::Owned, K, V, marker::LeafOrInternal>`
//! as an impl on `NodeRef` itself. The alias-helper lowering
//! (`__rusty_alias_Root_<method>(auto& self_, …)`) cannot express that: the
//! alias's generic parameters appear in no deducible position, so every
//! call needs explicit template arguments nothing can recover
//! (`Root::new_pillar(h, alloc)` inside `split_off`). Rewriting the impl
//! header to the aliased type before codegen gives the ordinary member
//! lowering — the shape the vendored btree port has always had.
//!
//! Only aliases whose target is a struct, enum or union declared in this
//! crate qualify; aliases of runtime, foreign, tuple or primitive types keep
//! the helper path, and a nominal type of the alias's own name wins.

use std::collections::{HashMap, HashSet};
use syn::visit_mut::VisitMut;

/// Rewrite every qualifying inherent impl header in `file` (recursing into
/// inline modules). `crate_aliases` and `crate_nominal` carry the crate-wide
/// declarations in crate mode; the file's own are added here. Returns the
/// number of impls rewritten.
pub(crate) fn inline_alias_impl_headers(
    file: &mut syn::File,
    crate_aliases: &[syn::ItemType],
    crate_nominal: &HashSet<String>,
) -> usize {
    let mut aliases: HashMap<String, Vec<syn::ItemType>> = HashMap::new();
    for alias in crate_aliases {
        aliases
            .entry(alias.ident.to_string())
            .or_default()
            .push(alias.clone());
    }
    collect_aliases(&file.items, &mut aliases);
    let mut nominal = crate_nominal.clone();
    collect_nominal(&file.items, &mut nominal);
    // One declaration per name: a name declared differently in two modules
    // is ambiguous here and is left to the helper path.
    let aliases: HashMap<String, syn::ItemType> = aliases
        .into_iter()
        .filter_map(|(name, mut decls)| {
            decls.dedup_by(|a, b| a.generics == b.generics && a.ty == b.ty);
            (decls.len() == 1).then(|| (name, decls.pop().unwrap()))
        })
        .collect();
    let mut rewritten = 0;
    rewrite_items(&mut file.items, &aliases, &nominal, &mut rewritten);
    rewritten
}

fn collect_aliases(items: &[syn::Item], out: &mut HashMap<String, Vec<syn::ItemType>>) {
    for item in items {
        match item {
            syn::Item::Type(alias) => out
                .entry(alias.ident.to_string())
                .or_default()
                .push(alias.clone()),
            syn::Item::Mod(module) => {
                if let Some((_, items)) = &module.content {
                    collect_aliases(items, out);
                }
            }
            _ => {}
        }
    }
}

fn collect_nominal(items: &[syn::Item], out: &mut HashSet<String>) {
    for item in items {
        match item {
            syn::Item::Struct(s) => {
                out.insert(s.ident.to_string());
            }
            syn::Item::Enum(e) => {
                out.insert(e.ident.to_string());
            }
            syn::Item::Union(u) => {
                out.insert(u.ident.to_string());
            }
            syn::Item::Mod(module) => {
                if let Some((_, items)) = &module.content {
                    collect_nominal(items, out);
                }
            }
            _ => {}
        }
    }
}

fn rewrite_items(
    items: &mut [syn::Item],
    aliases: &HashMap<String, syn::ItemType>,
    nominal: &HashSet<String>,
    rewritten: &mut usize,
) {
    for item in items.iter_mut() {
        match item {
            syn::Item::Impl(block) => {
                if let Some(target) = inlined_self_ty(block, aliases, nominal) {
                    *block.self_ty = target;
                    *rewritten += 1;
                }
            }
            syn::Item::Mod(module) => {
                if let Some((_, items)) = &mut module.content {
                    rewrite_items(items, aliases, nominal, rewritten);
                }
            }
            _ => {}
        }
    }
}

/// The aliased type with the impl's arguments substituted for the alias's
/// parameters, when `block` is an inherent impl on a qualifying alias.
fn inlined_self_ty(
    block: &syn::ItemImpl,
    aliases: &HashMap<String, syn::ItemType>,
    nominal: &HashSet<String>,
) -> Option<syn::Type> {
    if block.trait_.is_some() {
        return None;
    }
    let syn::Type::Path(self_path) = &*block.self_ty else {
        return None;
    };
    if self_path.qself.is_some() {
        return None;
    }
    let last = self_path.path.segments.last()?;
    let name = last.ident.to_string();
    if nominal.contains(&name) {
        return None;
    }
    let alias = aliases.get(&name)?;
    let syn::Type::Path(target) = &*alias.ty else {
        return None;
    };
    if target.qself.is_some() || !nominal.contains(&target.path.segments.last()?.ident.to_string())
    {
        return None;
    }
    if alias
        .generics
        .params
        .iter()
        .any(|param| matches!(param, syn::GenericParam::Const(_)))
    {
        return None;
    }
    let type_params: Vec<String> = alias
        .generics
        .params
        .iter()
        .filter_map(|param| match param {
            syn::GenericParam::Type(t) => Some(t.ident.to_string()),
            _ => None,
        })
        .collect();
    let lifetime_params: Vec<String> = alias
        .generics
        .params
        .iter()
        .filter_map(|param| match param {
            syn::GenericParam::Lifetime(l) => Some(l.lifetime.ident.to_string()),
            _ => None,
        })
        .collect();
    let (type_args, lifetime_args): (Vec<syn::Type>, Vec<syn::Lifetime>) = match &last.arguments {
        syn::PathArguments::None => (Vec::new(), Vec::new()),
        syn::PathArguments::AngleBracketed(args) => (
            args.args
                .iter()
                .filter_map(|arg| match arg {
                    syn::GenericArgument::Type(ty) => Some(ty.clone()),
                    _ => None,
                })
                .collect(),
            args.args
                .iter()
                .filter_map(|arg| match arg {
                    syn::GenericArgument::Lifetime(l) => Some(l.clone()),
                    _ => None,
                })
                .collect(),
        ),
        syn::PathArguments::Parenthesized(_) => return None,
    };
    if type_args.len() != type_params.len() {
        return None;
    }
    let types: HashMap<String, syn::Type> = type_params.into_iter().zip(type_args).collect();
    // Lifetimes are erased downstream; map them only when supplied positionally.
    let lifetimes: HashMap<String, syn::Lifetime> = if lifetime_args.len() == lifetime_params.len() {
        lifetime_params.into_iter().zip(lifetime_args).collect()
    } else {
        HashMap::new()
    };
    let mut target = (*alias.ty).clone();
    Substitute { types: &types, lifetimes: &lifetimes }.visit_type_mut(&mut target);
    Some(target)
}

struct Substitute<'a> {
    types: &'a HashMap<String, syn::Type>,
    lifetimes: &'a HashMap<String, syn::Lifetime>,
}

impl VisitMut for Substitute<'_> {
    fn visit_type_mut(&mut self, ty: &mut syn::Type) {
        if let syn::Type::Path(path) = ty
            && path.qself.is_none()
            && path.path.segments.len() == 1
            && matches!(path.path.segments[0].arguments, syn::PathArguments::None)
            && let Some(replacement) = self.types.get(&path.path.segments[0].ident.to_string())
        {
            *ty = replacement.clone();
            return;
        }
        syn::visit_mut::visit_type_mut(self, ty);
    }

    fn visit_lifetime_mut(&mut self, lifetime: &mut syn::Lifetime) {
        if let Some(replacement) = self.lifetimes.get(&lifetime.ident.to_string()) {
            *lifetime = replacement.clone();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rewrite(source: &str) -> (usize, String) {
        let mut file: syn::File = syn::parse_str(source).unwrap();
        let n = inline_alias_impl_headers(&mut file, &[], &HashSet::new());
        (n, quote::quote!(#file).to_string())
    }

    #[test]
    fn generic_alias_of_a_crate_struct_is_inlined_with_its_arguments() {
        let (n, out) = rewrite(
            r#"
            pub mod marker { pub struct Owned; pub struct LeafOrInternal; }
            pub struct NodeRef<B, K, V, T> { h: usize, _p: core::marker::PhantomData<(B, K, V, T)> }
            pub(super) type Root<K, V> = NodeRef<marker::Owned, K, V, marker::LeafOrInternal>;
            impl<A, B> Root<A, B> { pub fn fix_top(&mut self) { self.h += 1; } }
            "#,
        );
        assert_eq!(n, 1);
        assert!(
            out.contains("impl < A , B > NodeRef < marker :: Owned , A , B , marker :: LeafOrInternal >"),
            "{out}"
        );
    }

    #[test]
    fn aliases_of_foreign_or_runtime_types_keep_their_header() {
        let (n, out) = rewrite(
            r#"
            type Bytes = Vec<u8>;
            impl Bytes { fn len2(&self) -> usize { self.len() * 2 } }
            type Pair<T> = (T, T);
            trait Tr { fn f(&self); }
            type Alias = Pair<u8>;
            impl Tr for Alias { fn f(&self) {} }
            "#,
        );
        assert_eq!(n, 0);
        assert!(out.contains("impl Bytes"), "{out}");
    }

    #[test]
    fn a_nominal_type_of_the_alias_name_wins_and_arity_must_match() {
        let (n, out) = rewrite(
            r#"
            pub struct Bucket<T, V> { t: T, v: V }
            pub mod set {
                pub type Bucket<T> = super::Bucket<T, ()>;
                pub struct Other;
                impl<T> Bucket<T> { pub fn key(&self) -> &T { &self.t } }
            }
            pub mod map { impl<T, V> super::Bucket<T, V> { pub fn value(&self) -> &V { &self.v } } }
            "#,
        );
        // `Bucket` names a declared struct: the inherent impl stays on it.
        assert_eq!(n, 0);
        assert!(out.contains("impl < T > Bucket < T >"), "{out}");
    }
}
