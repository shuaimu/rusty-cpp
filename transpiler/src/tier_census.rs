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
    /// A generic associated type (`type Item<'a>;` / `type T<U>;`): no C++
    /// member of a class can be a template alias overridden per implementor.
    has_gat: bool,
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
    /// A data-carrying enum: lowered to variant structs over `std::variant`,
    /// no single class to carry an interface base (`emit_enum` adds none).
    is_enum: bool,
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
                let has_gat = t.items.iter().any(|ti| {
                    matches!(ti, syn::TraitItem::Type(at) if !at.generics.params.is_empty())
                });
                c.traits.insert(
                    key.clone(),
                    TraitInfo {
                        key,
                        supertraits,
                        methods,
                        has_generics: t.generics.params.iter().any(|p| matches!(p, syn::GenericParam::Type(_))),
                        has_gat,
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
                    is_enum: false,
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
                        is_enum: true,
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
                            c.types.entry(format!("?inherent?::{}", leaf)).or_insert(TypeInfo { key: leaf.clone(), params: vec![], repr_c_or_transparent: false, is_enum: false, inherent_methods: BTreeSet::new() }).inherent_methods.extend(names);
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
    if t.has_gat {
        return Err("A1 generic associated type".to_string());
    }
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

/// Book §3.2.1 axis 1 for every crate trait of `file`, keyed by the
/// module-scoped trait name (`m::Tr`, or `Tr` at the root): `Ok(())` when the
/// trait can be a C++ interface class, else the excluding test. Phase 0
/// (§3.2.16) consults it to make a `cpp_trait_member_dispatch` marker on an
/// ineligible trait a diagnosed no-op; phase 1 decides every trait's tier by it.
pub fn axis1_verdicts(file: &syn::File) -> std::collections::HashMap<String, Result<(), String>> {
    let mut c = Census::default();
    let mut module = Vec::new();
    collect(&file.items, &mut module, &mut c);
    let mut out = std::collections::HashMap::new();
    for key in c.traits.keys() {
        let mut seen = Vec::new();
        out.insert(key.clone(), axis1(&c, key, &mut seen));
    }
    out
}

/// Book §3.2.1 applied to one crate-trait `(trait, impl)` pair: `Ok(())` when
/// the pair may be emitted tier 1, else the excluding test (`A1 …` / `A2 …`).
#[derive(Debug, Clone)]
pub struct PairVerdict {
    /// The trait's module-scoped key (`m::Tr`, or `Tr` at the root).
    pub trait_key: String,
    /// The impl's self type as written.
    pub self_written: String,
    /// The self type's module-scoped key when it is a declared struct/enum.
    pub self_type_key: Option<String>,
    /// The impl's module path.
    pub module: Vec<String>,
    pub verdict: Result<(), String>,
}

/// The census of one file: every crate-trait pair with its verdict, and the
/// same-name demotions of the post-pass (a method declared by two tier-1
/// traits on one type demotes BOTH pairs — the member name would collide).
pub struct CensusOutcome {
    pub pairs: Vec<PairVerdict>,
    /// (type key, method, the traits declaring it)
    pub demotions: Vec<(String, String, Vec<String>)>,
}

/// Phase 1 (§3.2.16): the per-pair tier decision, ONE source of truth for the
/// census log and the emitter.
pub fn pair_verdicts(file: &syn::File) -> CensusOutcome {
    pair_verdicts_for_units(&[(Vec::new(), file)])
}

/// The crate-wide tier verdicts of phase 1 (book §3.2.16, the program-wide
/// pre-pass): the census over EVERY file of a crate, each file's items under
/// its module path, so a trait declared in one file and implemented in
/// another forms a pair. Keys are crate-scoped (`de::Expected`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CrateTierVerdicts {
    /// `(crate-scoped trait key, self type as written)` → verdict.
    pub pairs: std::collections::HashMap<(String, String), Result<(), String>>,
    /// Trait leaf name → the crate-scoped keys declaring it (one = unambiguous).
    pub leaf_keys: std::collections::HashMap<String, Vec<String>>,
}

impl CrateTierVerdicts {
    pub fn from_outcome(outcome: &CensusOutcome) -> Self {
        let mut pairs = std::collections::HashMap::new();
        let mut leaf_keys: std::collections::HashMap<String, Vec<String>> = std::collections::HashMap::new();
        for p in &outcome.pairs {
            pairs.insert((p.trait_key.clone(), p.self_written.clone()), p.verdict.clone());
            let leaf = p.trait_key.rsplit("::").next().unwrap_or(&p.trait_key).to_string();
            let keys = leaf_keys.entry(leaf).or_default();
            if !keys.contains(&p.trait_key) {
                keys.push(p.trait_key.clone());
            }
        }
        CrateTierVerdicts { pairs, leaf_keys }
    }
}

pub fn pair_verdicts_for_units(units: &[(Vec<String>, &syn::File)]) -> CensusOutcome {
    let mut c = Census::default();
    for (module_path, file) in units {
        let mut module = module_path.clone();
        collect(&file.items, &mut module, &mut c);
    }
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
    let mut pairs: Vec<PairVerdict> = Vec::new();
    for imp in c.impls.iter() {
        let Some(tk) = &imp.trait_key else { continue }; // foreign trait: not a crate-trait pair
        let mut self_type_key_out: Option<String> = None;
        let verdict: Result<(), String> = (|| {
            trait_verdict.get(tk).cloned().unwrap_or_else(|| Err("unknown".into()))?;
            let ty = self_type_key(&c, imp)?;
            self_type_key_out = Some(ty.key.clone());
            if ty.repr_c_or_transparent {
                return Err("A2 repr(C)/repr(transparent)".into());
            }
            if ty.is_enum {
                return Err("A2 enum self type: variant-lowered, no base".into());
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
            // book §3.2.14: `impl Tr for &T` beside `impl Tr for T` is the keyed
            // (`self_tag`) twin of the lane — references are never tier-1 self
            // types, and a tier-1 `T` beside its twin would make every `&T`
            // coercion to `dyn Tr` pick the inherited value body (measured on
            // trait_probes_collapse under the phase-1 switch): the pair stays
            // tier 2 so both bodies live in the lane.
            {
                // …and the same for `Box<T>` / `Rc<T>` / `Arc<T>` / `&T` twins:
                // any OTHER impl of this trait whose self type mentions `T` as a
                // path segment is reached through the lane's keyed/forwarded
                // body, which a tier-1 `T` would shadow on a deref-coerced call.
                let leaf = ty.key.rsplit("::").next().unwrap_or(&ty.key).to_string();
                let mentions_leaf = |written: &str| -> bool {
                    written
                        .split(|ch: char| !(ch.is_alphanumeric() || ch == '_'))
                        .any(|tok| tok == leaf)
                };
                let twin = c.impls.iter().find(|o| {
                    o.trait_key.as_deref() == Some(tk.as_str())
                        && o.self_written != imp.self_written
                        && mentions_leaf(&o.self_written)
                });
                if let Some(o) = twin {
                    return Err(format!("A2 `impl {} for {}` twin: the pair stays keyed in the lane", t.key.rsplit("::").next().unwrap_or(&t.key), o.self_written));
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
        });
        pairs.push(PairVerdict {
            trait_key: tk.clone(),
            self_written: imp.self_written.clone(),
            self_type_key: self_type_key_out,
            module: imp.module.clone(),
            verdict,
        });
    }
    // same-name test across the type's tier-1 impls (post-pass): demote both pairs
    let mut demotions: Vec<(String, String, Vec<String>)> = Vec::new();
    for (ty_key, entries) in &tier1_methods_by_type {
        let mut by_name: BTreeMap<&String, BTreeSet<&String>> = BTreeMap::new();
        for (m, tr) in entries {
            by_name.entry(m).or_default().insert(tr);
        }
        for (m, traits) in by_name {
            if traits.len() > 1 {
                let traits: Vec<String> = traits.iter().map(|s| s.to_string()).collect();
                for p in pairs.iter_mut() {
                    if p.verdict.is_ok()
                        && p.self_type_key.as_deref() == Some(ty_key.as_str())
                        && traits.contains(&p.trait_key)
                    {
                        p.verdict = Err(format!("A2 `{m}` declared by two tier-1 traits on one type"));
                    }
                }
                demotions.push((ty_key.clone(), m.clone(), traits));
            }
        }
    }
    CensusOutcome { pairs, demotions }
}

pub fn run(file: &syn::File, label: &str) {
    let outcome = pair_verdicts(file);
    let mut tier1 = 0usize;
    for p in &outcome.pairs {
        match &p.verdict {
            Ok(()) => {
                tier1 += 1;
                eprintln!("census\t{label}\t{}\t{}\tTIER1\t-", p.trait_key, p.self_written);
            }
            Err(reason) => eprintln!("census\t{label}\t{}\t{}\tTIER2\t{reason}", p.trait_key, p.self_written),
        }
    }
    for (ty_key, m, traits) in &outcome.demotions {
        eprintln!(
            "census\t{label}\t{}\t{ty_key}\tTIER2\tA2 `{m}` declared by two tier-1 traits on one type",
            traits.join("+")
        );
    }
    eprintln!(
        "census-summary\t{label}\tpairs={}\ttier1={}\tsame-name-demotions={}",
        outcome.pairs.len(),
        tier1,
        outcome.demotions.len()
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pair_verdicts_apply_both_axes_and_the_same_name_demotion() {
        let file: syn::File = syn::parse_str(
            r#"
            pub trait Same { fn same(&self, other: &Self) -> bool; }
            pub trait Gen { fn show<T: core::fmt::Debug>(&self, t: T) -> String; }
            pub trait Fine { fn v(&self) -> i32; }
            pub trait Gat { type Item<'a>; fn first(&self) -> i32; }
            pub struct P { pub val: i32 }
            impl Same for P { fn same(&self, other: &Self) -> bool { self.val == other.val } }
            impl Gen for P { fn show<T: core::fmt::Debug>(&self, t: T) -> String { format!("{:?}", t) } }
            impl Fine for P { fn v(&self) -> i32 { self.val } }
            impl Gat for P { type Item<'a> = &'a i32; fn first(&self) -> i32 { 1 } }
            pub trait A { fn k(&self) -> i32; }
            pub trait B { fn k(&self) -> i32; }
            pub struct Q;
            impl A for Q { fn k(&self) -> i32 { 1 } }
            impl B for Q { fn k(&self) -> i32 { 2 } }
            pub struct R { pub x: i32 }
            impl R { pub fn v(&self) -> i32 { self.x } }
            impl Fine for R { fn v(&self) -> i32 { self.x } }
            impl Fine for i32 { fn v(&self) -> i32 { *self } }
            pub trait RefTr { fn m(&self) -> i32; }
            pub struct Tw { pub v: i32 }
            impl RefTr for Tw { fn m(&self) -> i32 { self.v } }
            impl RefTr for &Tw { fn m(&self) -> i32 { self.v + 1000 } }
            pub trait BoxTr { fn b(&self) -> i32; }
            pub struct Bw { pub v: i32 }
            impl BoxTr for Bw { fn b(&self) -> i32 { self.v } }
            impl BoxTr for Box<Bw> { fn b(&self) -> i32 { self.v + 1 } }
            pub enum Ev { A(i32), B }
            impl Fine for Ev { fn v(&self) -> i32 { 0 } }
            "#,
        )
        .unwrap();
        let outcome = pair_verdicts(&file);
        let find = |t: &str, s: &str| -> Result<(), String> {
            outcome
                .pairs
                .iter()
                .find(|p| p.trait_key == t && p.self_written == s)
                .unwrap_or_else(|| panic!("no pair {t} for {s}"))
                .verdict
                .clone()
        };
        assert!(find("Same", "P").unwrap_err().contains("A1 `Self` outside receiver"));
        assert!(find("Gen", "P").unwrap_err().contains("A1 generic required method"));
        assert!(find("Gat", "P").unwrap_err().contains("A1 generic associated type"));
        assert_eq!(find("Fine", "P"), Ok(()));
        assert!(find("Fine", "R").unwrap_err().contains("A2 inherent method `v` shadows"));
        assert!(find("Fine", "i32").unwrap_err().contains("A2 self type `i32` not declared"));
        // a reference twin keeps the value pair in the lane (keyed by self_tag)
        assert!(find("RefTr", "Tw").unwrap_err().contains("twin"));
        assert!(find("RefTr", "& Tw").unwrap_err().contains("A2 self type"));
        assert!(find("BoxTr", "Bw").unwrap_err().contains("twin"));
        assert!(find("Fine", "Ev").unwrap_err().contains("A2 enum self type"));
        // the same-name post-pass demotes BOTH tier-1 pairs on one type
        assert!(find("A", "Q").unwrap_err().contains("declared by two tier-1 traits"));
        assert!(find("B", "Q").unwrap_err().contains("declared by two tier-1 traits"));
        assert_eq!(outcome.demotions.len(), 1);
    }

    #[test]
    fn pair_verdicts_for_units_pairs_a_trait_of_one_file_with_an_impl_in_another() {
        let root: syn::File = syn::parse_str("pub trait Tr { fn v(&self) -> i32; } pub mod m;").unwrap();
        let m: syn::File = syn::parse_str(
            "pub struct X { pub k: i32 } impl crate::Tr for X { fn v(&self) -> i32 { self.k } } pub struct Y; impl super::Tr for &Y { fn v(&self) -> i32 { 0 } }",
        )
        .unwrap();
        let units = vec![(Vec::new(), &root), (vec!["m".to_string()], &m)];
        let outcome = pair_verdicts_for_units(&units);
        let cv = CrateTierVerdicts::from_outcome(&outcome);
        assert_eq!(cv.pairs.get(&("Tr".to_string(), "X".to_string())), Some(&Ok(())));
        assert!(cv.pairs.get(&("Tr".to_string(), "& Y".to_string())).unwrap().is_err());
        assert_eq!(cv.leaf_keys.get("Tr"), Some(&vec!["Tr".to_string()]));
        // the per-file census of `m` alone has no pair at all
        assert!(pair_verdicts(&m).pairs.is_empty());
    }
}
