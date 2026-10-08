//! Book §3.2.1 tier census (phase-0 / phase-1 gate metric, §3.2.16 (p)): a
//! READ-ONLY pass that applies the two lexical axes to every crate-trait
//! `(trait, impl)` pair of one source file and reports, per pair, whether it
//! is tier 1 or which test excludes it. Nothing here changes emission; the
//! numbers it prints are the measure every phase-0/1 widening is judged by.
//!
//! Enabled by `RUSTY_CPP_TIER_CENSUS=1`; prints to stderr, one line per pair:
//! `census\t<crate-or-file>\t<module::Trait>\t<self type>\tTIER1|TIER2\t<reason>`
//! and a summary `census-summary\t<name>\tpairs=<n>\ttier1=<n>`.

use std::collections::{BTreeMap, BTreeSet, HashMap};

#[derive(Debug, Clone)]
struct TraitInfo {
    key: String,
    supertraits: Vec<String>, // as written (last segment kept with path)
    methods: Vec<MethodInfo>,
    has_generics: bool,
}

#[derive(Debug, Clone)]
struct MethodInfo {
    name: String,
    required: bool,
    has_receiver: bool,
    generic_over_type: bool, // type params / impl Trait params / async
    returns_impl_trait: bool,
    mentions_self_outside_receiver: bool,
    where_self_sized: bool,
}

#[derive(Debug, Clone)]
struct TypeInfo {
    key: String,
    params: Vec<String>,
    repr_c_or_transparent: bool,
    inherent_methods: BTreeSet<String>,
}

#[derive(Debug, Clone)]
struct ImplInfo {
    trait_key: Option<String>, // resolved crate trait key, None when foreign/unresolved
    trait_written: String,
    trait_args: usize,
    self_ty: syn::Type,
    self_written: String,
    generics: syn::Generics,
    module: Vec<String>,
}

#[derive(Default)]
struct Census {
    traits: BTreeMap<String, TraitInfo>,
    types: BTreeMap<String, TypeInfo>,
    impls: Vec<ImplInfo>,
}

fn scoped(module: &[String], name: &str) -> String {
    if module.is_empty() {
        name.to_string()
    } else {
        format!("{}::{}", module.join("::"), name)
    }
}

fn type_mentions_self(ty: &syn::Type) -> bool {
    use syn::visit::Visit;
    struct V(bool);
    impl<'a> Visit<'a> for V {
        fn visit_path(&mut self, p: &'a syn::Path) {
            if p.segments.first().is_some_and(|s| s.ident == "Self") {
                self.0 = true;
            }
            syn::visit::visit_path(self, p);
        }
    }
    let mut v = V(false);
    v.visit_type(ty);
    v.0
}

fn where_self_sized(generics: &syn::Generics) -> bool {
    generics.where_clause.as_ref().is_some_and(|wc| {
        wc.predicates.iter().any(|p| match p {
            syn::WherePredicate::Type(pt) => {
                matches!(&pt.bounded_ty, syn::Type::Path(tp) if tp.path.is_ident("Self"))
                    && pt.bounds.iter().any(|b| match b {
                        syn::TypeParamBound::Trait(tb) => {
                            tb.path.segments.last().is_some_and(|s| s.ident == "Sized")
                        }
                        _ => false,
                    })
            }
            _ => false,
        })
    })
}

fn method_info(sig: &syn::Signature, required: bool) -> MethodInfo {
    let has_receiver = matches!(sig.inputs.first(), Some(syn::FnArg::Receiver(_)));
    let generic_over_type = sig.asyncness.is_some()
        || sig.generics.params.iter().any(|p| matches!(p, syn::GenericParam::Type(_)))
        || sig.inputs.iter().any(|a| match a {
            syn::FnArg::Typed(t) => matches!(t.ty.as_ref(), syn::Type::ImplTrait(_)),
            _ => false,
        });
    let returns_impl_trait = matches!(&sig.output, syn::ReturnType::Type(_, t) if matches!(t.as_ref(), syn::Type::ImplTrait(_)));
    let mut mentions = false;
    for a in sig.inputs.iter() {
        if let syn::FnArg::Typed(t) = a
            && type_mentions_self(&t.ty)
        {
            mentions = true;
        }
    }
    if let syn::ReturnType::Type(_, t) = &sig.output
        && type_mentions_self(t)
    {
        mentions = true;
    }
    MethodInfo {
        name: sig.ident.to_string(),
        required,
        has_receiver,
        generic_over_type,
        returns_impl_trait,
        mentions_self_outside_receiver: mentions,
        where_self_sized: where_self_sized(&sig.generics),
    }
}

fn repr_c_or_transparent(attrs: &[syn::Attribute]) -> bool {
    attrs.iter().any(|a| {
        a.path().is_ident("repr")
            && a.meta
                .require_list()
                .ok()
                .map(|l| {
                    let s = l.tokens.to_string();
                    s.contains("C") || s.contains("transparent")
                })
                .unwrap_or(false)
    })
}

fn collect(items: &[syn::Item], module: &mut Vec<String>, c: &mut Census) {
    for item in items {
        match item {
            syn::Item::Trait(t) => {
                let key = scoped(module, &t.ident.to_string());
                let supertraits: Vec<String> = t
                    .supertraits
                    .iter()
                    .filter_map(|b| match b {
                        syn::TypeParamBound::Trait(tb) => Some(
                            tb.path.segments.iter().map(|s| s.ident.to_string()).collect::<Vec<_>>().join("::"),
                        ),
                        _ => None,
                    })
                    .collect();
                let methods = t
                    .items
                    .iter()
                    .filter_map(|ti| match ti {
                        syn::TraitItem::Fn(f) => Some(method_info(&f.sig, f.default.is_none())),
                        _ => None,
                    })
                    .collect();
                c.traits.insert(
                    key.clone(),
                    TraitInfo {
                        key,
                        supertraits,
                        methods,
                        has_generics: t.generics.params.iter().any(|p| matches!(p, syn::GenericParam::Type(_))),
                    },
                );
            }
            syn::Item::Struct(s) => {
                let key = scoped(module, &s.ident.to_string());
                c.types.entry(key.clone()).or_insert(TypeInfo {
                    key,
                    params: s.generics.params.iter().filter_map(|p| match p { syn::GenericParam::Type(tp) => Some(tp.ident.to_string()), _ => None }).collect(),
                    repr_c_or_transparent: repr_c_or_transparent(&s.attrs),
                    inherent_methods: BTreeSet::new(),
                });
            }
            syn::Item::Enum(e) => {
                let data_carrying = e.variants.iter().any(|v| !matches!(v.fields, syn::Fields::Unit));
                if data_carrying {
                    let key = scoped(module, &e.ident.to_string());
                    c.types.entry(key.clone()).or_insert(TypeInfo {
                        key,
                        params: e.generics.params.iter().filter_map(|p| match p { syn::GenericParam::Type(tp) => Some(tp.ident.to_string()), _ => None }).collect(),
                        repr_c_or_transparent: repr_c_or_transparent(&e.attrs),
                        inherent_methods: BTreeSet::new(),
                    });
                }
            }
            syn::Item::Impl(i) => {
                let self_written = quote::ToTokens::to_token_stream(&i.self_ty).to_string();
                match &i.trait_ {
                    None => {
                        // inherent impl: record method names on the self type (by leaf)
                        if let syn::Type::Path(tp) = i.self_ty.as_ref()
                            && let Some(last) = tp.path.segments.last()
                        {
                            let leaf = last.ident.to_string();
                            let names: Vec<String> = i.items.iter().filter_map(|it| match it { syn::ImplItem::Fn(f) => Some(f.sig.ident.to_string()), _ => None }).collect();
                            for (k, ty) in c.types.iter_mut() {
                                if k.rsplit("::").next() == Some(leaf.as_str()) {
                                    ty.inherent_methods.extend(names.iter().cloned());
                                }
                            }
                            // also queue for types declared later (two-pass simplification: store under leaf)
                            c.types.entry(format!("?inherent?::{}", leaf)).or_insert(TypeInfo { key: leaf.clone(), params: vec![], repr_c_or_transparent: false, inherent_methods: BTreeSet::new() }).inherent_methods.extend(names);
                        }
                    }
                    Some((_, path, _)) => {
                        let written = path.segments.iter().map(|s| s.ident.to_string()).collect::<Vec<_>>().join("::");
                        let trait_args = match path.segments.last().map(|s| &s.arguments) {
                            Some(syn::PathArguments::AngleBracketed(a)) => a.args.iter().filter(|g| matches!(g, syn::GenericArgument::Type(_))).count(),
                            _ => 0,
                        };
                        c.impls.push(ImplInfo {
                            trait_key: None,
                            trait_written: written,
                            trait_args,
                            self_ty: (*i.self_ty).clone(),
                            self_written,
                            generics: i.generics.clone(),
                            module: module.clone(),
                        });
                    }
                }
            }
            syn::Item::Mod(m) => {
                if let Some((_, nested)) = &m.content {
                    module.push(m.ident.to_string());
                    collect(nested, module, c);
                    module.pop();
                }
            }
            _ => {}
        }
    }
}

fn resolve_trait(c: &Census, written: &str, module: &[String]) -> Option<String> {
    let leaf = written.rsplit("::").next().unwrap_or(written);
    // exact scoped match from the impl's module outward
    let mut m = module.to_vec();
    loop {
        let cand = scoped(&m, leaf);
        if c.traits.contains_key(&cand) {
            return Some(cand);
        }
        if m.pop().is_none() {
            break;
        }
    }
    // a written path relative to the crate root (`de::Error`)
    if written.contains("::") {
        let rel = written.trim_start_matches("crate::").to_string();
        if c.traits.contains_key(&rel) {
            return Some(rel);
        }
    }
    // unique leaf
    let hits: Vec<&String> = c.traits.keys().filter(|k| k.rsplit("::").next() == Some(leaf)).collect();
    if hits.len() == 1 { Some(hits[0].clone()) } else { None }
}

const MARKER_SUPERTRAITS: &[&str] = &["Sized", "Send", "Sync", "Copy", "Unpin"];

fn axis1(c: &Census, key: &str, seen: &mut Vec<String>) -> Result<(), String> {
    let t = c.traits.get(key).ok_or_else(|| format!("unknown trait {key}"))?;
    if seen.contains(&t.key) {
        return Ok(()); // cycle guard
    }
    seen.push(t.key.clone());
    for m in &t.methods {
        if m.required && (m.generic_over_type || m.returns_impl_trait) && !m.where_self_sized {
            return Err(format!("A1 generic required method `{}`", m.name));
        }
        if m.mentions_self_outside_receiver && !m.where_self_sized {
            return Err(format!("A1 `Self` outside receiver in `{}`", m.name));
        }
        if !m.required && m.generic_over_type && !m.where_self_sized {
            return Err(format!("A1 generic default `{}` without `where Self: Sized`", m.name));
        }
    }
    let _ = &t.has_generics;
    // supertraits
    let mut super_methods: BTreeSet<String> = BTreeSet::new();
    for s in &t.supertraits {
        let leaf = s.rsplit("::").next().unwrap_or(s);
        if MARKER_SUPERTRAITS.contains(&leaf) {
            continue;
        }
        let Some(sk) = resolve_trait(c, s, &[]) else {
            return Err(format!("A1 foreign supertrait `{s}`"));
        };
        axis1(c, &sk, seen).map_err(|e| format!("A1 supertrait `{leaf}`: {e}"))?;
        collect_super_methods(c, &sk, &mut super_methods);
    }
    for m in &t.methods {
        if super_methods.contains(&m.name) {
            return Err(format!("A1 `{}` also declared by a supertrait", m.name));
        }
    }
    Ok(())
}

fn collect_super_methods(c: &Census, key: &str, out: &mut BTreeSet<String>) {
    if let Some(t) = c.traits.get(key) {
        for m in &t.methods {
            out.insert(m.name.clone());
        }
        for s in &t.supertraits {
            if let Some(sk) = resolve_trait(c, s, &[]) && sk != key {
                collect_super_methods(c, &sk, out);
            }
        }
    }
}

fn self_type_key<'c>(c: &'c Census, imp: &ImplInfo) -> Result<&'c TypeInfo, String> {
    let syn::Type::Path(tp) = &imp.self_ty else {
        return Err(format!("A2 self type `{}` is not a declared struct/enum", imp.self_written));
    };
    if tp.qself.is_some() {
        return Err("A2 qualified self type".into());
    }
    let leaf = tp.path.segments.last().map(|s| s.ident.to_string()).unwrap_or_default();
    let ty = {
        let mut m = imp.module.clone();
        let mut found: Option<&TypeInfo> = None;
        loop {
            if let Some(t) = c.types.get(&scoped(&m, &leaf)) {
                found = Some(t);
                break;
            }
            if m.pop().is_none() {
                break;
            }
        }
        found.or_else(|| {
            let hits: Vec<&TypeInfo> = c.types.values().filter(|t| !t.key.starts_with("?inherent?") && t.key.rsplit("::").next() == Some(leaf.as_str())).collect();
            if hits.len() == 1 { Some(hits[0]) } else { None }
        })
    };
    let Some(ty) = ty else {
        return Err(format!("A2 self type `{}` not declared in this crate", imp.self_written));
    };
    // exactly its own parameter list
    let impl_params: Vec<String> = imp.generics.params.iter().filter_map(|p| match p { syn::GenericParam::Type(tp) => Some(tp.ident.to_string()), _ => None }).collect();
    let args: Vec<String> = match &tp.path.segments.last().unwrap().arguments {
        syn::PathArguments::AngleBracketed(a) => a.args.iter().filter_map(|g| match g { syn::GenericArgument::Type(syn::Type::Path(p)) if p.qself.is_none() && p.path.segments.len()==1 => Some(p.path.segments[0].ident.to_string()), syn::GenericArgument::Type(_) => Some("<concrete>".into()), _ => None }).collect(),
        _ => Vec::new(),
    };
    if args.len() != ty.params.len() {
        return Err(format!("A2 self type `{}` applied to {} args, declares {}", imp.self_written, args.len(), ty.params.len()));
    }
    let mut used = BTreeSet::new();
    for a in &args {
        if !impl_params.contains(a) || !used.insert(a.clone()) {
            return Err(format!("A2 self type `{}` is not the type applied to its own parameter list", imp.self_written));
        }
    }
    // extra bounds on the impl params beyond the struct's own: any bound on an impl type param
    let bounded = imp.generics.params.iter().any(|p| matches!(p, syn::GenericParam::Type(tp) if !tp.bounds.is_empty()))
        || imp.generics.where_clause.as_ref().is_some_and(|wc| !wc.predicates.is_empty());
    if bounded {
        return Err(format!("A2 extra bound on the impl's parameters (`{}`)", imp.self_written));
    }
    Ok(ty)
}

pub fn run(file: &syn::File, label: &str) {
    let mut c = Census::default();
    let mut module = Vec::new();
    collect(&file.items, &mut module, &mut c);
    // resolve impl traits to crate traits
    let keys: Vec<Option<String>> = c.impls.iter().map(|i| resolve_trait(&c, &i.trait_written, &i.module)).collect();
    for (i, k) in c.impls.iter_mut().zip(keys) {
        i.trait_key = k;
    }
    // inherent methods queued under ?inherent?::<leaf> → merge into declared types
    let queued: Vec<(String, BTreeSet<String>)> = c.types.iter().filter(|(k, _)| k.starts_with("?inherent?::")).map(|(k, v)| (k.trim_start_matches("?inherent?::").to_string(), v.inherent_methods.clone())).collect();
    for (leaf, names) in queued {
        for (k, ty) in c.types.iter_mut() {
            if !k.starts_with("?inherent?") && k.rsplit("::").next() == Some(leaf.as_str()) {
                ty.inherent_methods.extend(names.iter().cloned());
            }
        }
    }
    // axis 1 per trait
    let mut trait_verdict: HashMap<String, Result<(), String>> = HashMap::new();
    for key in c.traits.keys() {
        let mut seen = Vec::new();
        trait_verdict.insert(key.clone(), axis1(&c, key, &mut seen));
    }
    // per-type tier-1 impl bookkeeping for the same-name / instantiation tests
    let mut tier1_methods_by_type: HashMap<String, Vec<(String, String)>> = HashMap::new(); // type key → (method, trait)
    let mut generic_trait_insts: HashMap<(String, String), usize> = HashMap::new(); // (type key, trait key) → count
    let mut pairs = 0usize;
    let mut tier1 = 0usize;
    let mut lines: Vec<String> = Vec::new();
    let mut tier1_pairs: Vec<(usize, String, String)> = Vec::new();
    for (idx, imp) in c.impls.iter().enumerate() {
        let Some(tk) = &imp.trait_key else { continue }; // foreign trait: not a crate-trait pair
        pairs += 1;
        let verdict: Result<(), String> = (|| {
            trait_verdict.get(tk).cloned().unwrap_or_else(|| Err("unknown".into()))?;
            let ty = self_type_key(&c, imp)?;
            if ty.repr_c_or_transparent {
                return Err("A2 repr(C)/repr(transparent)".into());
            }
            let t = &c.traits[tk];
            for m in &t.methods {
                if ty.inherent_methods.contains(&m.name) {
                    return Err(format!("A2 inherent method `{}` shadows the trait method", m.name));
                }
            }
            if t.has_generics {
                let e = generic_trait_insts.entry((ty.key.clone(), tk.clone())).or_insert(0);
                *e += 1;
                if *e > 1 {
                    return Err("A2 second instantiation of a generic trait on one type".into());
                }
            }
            // supertraits concretely implemented in this crate at tier 1
            for s in &t.supertraits {
                let leaf = s.rsplit("::").next().unwrap_or(s);
                if MARKER_SUPERTRAITS.contains(&leaf) { continue; }
                let Some(sk) = resolve_trait(&c, s, &[]) else { return Err(format!("A2 foreign supertrait `{s}`")); };
                let has = c.impls.iter().any(|o| o.trait_key.as_deref() == Some(sk.as_str()) && o.self_written == imp.self_written);
                if !has {
                    return Err(format!("A2 supertrait `{leaf}` not concretely implemented on `{}` in this crate", imp.self_written));
                }
            }
            Ok(ty.key.clone())
        })().map(|ty_key| {
            for m in &c.traits[tk].methods {
                tier1_methods_by_type.entry(ty_key.clone()).or_default().push((m.name.clone(), tk.clone()));
            }
            tier1_pairs.push((idx, ty_key, tk.clone()));
        });
        match verdict {
            Ok(()) => {
                tier1 += 1;
                lines.push(format!("census\t{label}\t{tk}\t{}\tTIER1\t-", imp.self_written));
            }
            Err(reason) => lines.push(format!("census\t{label}\t{tk}\t{}\tTIER2\t{reason}", imp.self_written)),
        }
    }
    // same-name test across the type's tier-1 impls (post-pass)
    let mut demoted = 0usize;
    for (ty_key, entries) in &tier1_methods_by_type {
        let mut by_name: BTreeMap<&String, BTreeSet<&String>> = BTreeMap::new();
        for (m, tr) in entries {
            by_name.entry(m).or_default().insert(tr);
        }
        for (m, traits) in by_name {
            if traits.len() > 1 {
                demoted += 1;
                lines.push(format!("census\t{label}\t{}\t{ty_key}\tTIER2\tA2 `{m}` declared by two tier-1 traits on one type", traits.iter().map(|s| s.as_str()).collect::<Vec<_>>().join("+")));
            }
        }
    }
    for l in &lines {
        eprintln!("{l}");
    }
    eprintln!("census-summary\t{label}\tpairs={pairs}\ttier1={}\tsame-name-demotions={demoted}", tier1);
}
