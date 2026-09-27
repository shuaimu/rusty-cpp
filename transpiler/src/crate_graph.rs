//! `--crate-graph`: transpile a crate together with its Cargo-selected local
//! path dependencies as one build.
//!
//! Plain crate mode walks path dependencies recursively and transpiles each one
//! per file, nested under its parent's output directory and with every module
//! exporting into the global namespace. That shape cannot carry a real crate
//! graph: a crate reached twice is emitted twice (two providers of one module
//! name), sibling modules of a multi-module dependency cannot name each other
//! (`vec_map::VecMap` names a namespace no module declares), and two crates
//! that both own a `types` module collide.
//!
//! Under `--crate-graph`:
//!
//! - Every dependency crate of Cargo's target-normal local graph is emitted
//!   exactly once, leaves first, into `<output-dir>/<package>/`, as ONE named
//!   C++ module per crate. The module is named after the crate's extern root
//!   (`lion_reactor`), and its purview is wrapped in `namespace lion_reactor`
//!   with the crate's own qualified references requalified (the wrap every
//!   parity-matrix crate already takes). Its Rust module tree becomes nested
//!   namespaces (`lion_reactor::types::Waker`); `pub use` re-exports become
//!   using-declarations in the owning namespace. A consumer names the crate
//!   through `import lion_reactor;` and `::lion_reactor::...`, so its names
//!   can never collide with the consumer's own namespace.
//! - The root crate keeps its layout (per-file modules; SRPC's `srpc.*`).
//! - `#[cfg(feature = "...")]` / `cfg_attr` / `cfg!` predicates are evaluated
//!   per crate against the feature set Cargo resolved for that crate in the
//!   selected graph ([`apply_feature_cfgs`]), exactly as rustc sees them.
//!   A module file whose declaration a false predicate removed is not part of
//!   the crate and is not read at all.
//! - Under `--verus-exec`, a module left without executable content after
//!   Verus erasure and ghost lowering emits nothing, and neither does a crate
//!   whose every module is such a module ([`prune_ghost_modules`]). Dependents
//!   see each dependency's [`CrateSurface`], so their imports and re-exports of
//!   the removed modules, of pruned spec-only datatypes and of erased spec
//!   functions vanish instead of becoming unresolved-import comments.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use proc_macro2::TokenStream;
use quote::ToTokens;
use syn::punctuated::Punctuated;
use syn::visit_mut::{self, VisitMut};
use syn::{Attribute, Expr, Item, Meta, Token};

// ---------------------------------------------------------------------------
// feature cfgs

/// Tri-state value of a cfg predicate once `feature = "..."` is known.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Truth {
    True,
    False,
    Unknown,
}

/// Evaluate every `feature = "x"` predicate in `source` against `features`
/// (Cargo's resolved feature set for this crate):
///
/// - `#[cfg(p)]`: `p` true removes the attribute, false removes the item
///   (item, impl/trait/foreign item, field, variant, statement, match arm,
///   struct-expression field, function parameter) it is attached to;
/// - `#[cfg_attr(p, a, b)]`: true replaces it by `#[a] #[b]`, false removes it;
/// - `cfg!(p)` becomes `true` / `false`;
/// - a predicate that still depends on another cfg keeps its other parts, with
///   each feature test replaced by `all()` (true) or `any()` (false).
///
/// A predicate is only touched when it tests a feature, so every other cfg
/// (`test`, `target_os`, SRPC's `cfg_attr(any(), ...)` directives) is left
/// exactly as written. Returns `Ok(None)` when the source contains no feature
/// predicate, `Ok(Some((text, disabled)))` otherwise, where `disabled` reports
/// a false inner `#![cfg(...)]` (the whole module file is compiled out).
/// A feature predicate in any other position is an error.
pub fn apply_feature_cfgs(
    source: &str,
    features: &BTreeSet<String>,
) -> Result<Option<(String, bool)>, String> {
    if !source.contains("feature") {
        return Ok(None);
    }
    let mut file = syn::parse_file(source).map_err(|error| format!("does not parse: {error}"))?;
    let mut evaluator = FeatureEvaluator {
        features,
        changed: false,
        errors: Vec::new(),
    };
    let disabled = match evaluator.filter_attrs(&mut file.attrs) {
        Truth::False => true,
        _ => false,
    };
    evaluator.visit_file_mut(&mut file);
    let mut residue = FeatureResidue { found: Vec::new() };
    syn::visit::Visit::visit_file(&mut residue, &file);
    evaluator.errors.extend(
        residue
            .found
            .into_iter()
            .map(|attr| format!("a `feature` cfg in a position --crate-graph does not evaluate: `{attr}`")),
    );
    if !evaluator.errors.is_empty() {
        return Err(evaluator.errors.join("; "));
    }
    if !evaluator.changed && !disabled {
        return Ok(None);
    }
    Ok(Some((prettyplease::unparse(&file), disabled)))
}

struct FeatureEvaluator<'a> {
    features: &'a BTreeSet<String>,
    changed: bool,
    errors: Vec<String>,
}

fn parse_meta_list(tokens: TokenStream) -> Option<Punctuated<Meta, Token![,]>> {
    syn::parse::Parser::parse2(Punctuated::<Meta, Token![,]>::parse_terminated, tokens).ok()
}

fn meta_mentions_feature(meta: &Meta) -> bool {
    match meta {
        Meta::NameValue(name_value) => name_value.path.is_ident("feature"),
        Meta::Path(_) => false,
        Meta::List(list) => {
            (list.path.is_ident("all") || list.path.is_ident("any") || list.path.is_ident("not"))
                && parse_meta_list(list.tokens.clone())
                    .is_some_and(|args| args.iter().any(meta_mentions_feature))
        }
    }
}

impl FeatureEvaluator<'_> {
    /// Replace each feature test in `meta` by `all()` / `any()` and evaluate.
    fn substitute(&mut self, meta: &Meta) -> (Meta, Truth) {
        match meta {
            Meta::NameValue(name_value) if name_value.path.is_ident("feature") => {
                let Expr::Lit(syn::ExprLit {
                    lit: syn::Lit::Str(name),
                    ..
                }) = &name_value.value
                else {
                    self.errors.push(format!(
                        "malformed feature predicate `{}`",
                        meta.to_token_stream()
                    ));
                    return (meta.clone(), Truth::Unknown);
                };
                if self.features.contains(&name.value()) {
                    (syn::parse_quote!(all()), Truth::True)
                } else {
                    (syn::parse_quote!(any()), Truth::False)
                }
            }
            Meta::List(list)
                if list.path.is_ident("all") || list.path.is_ident("any") || list.path.is_ident("not") =>
            {
                let Some(args) = parse_meta_list(list.tokens.clone()) else {
                    return (meta.clone(), Truth::Unknown);
                };
                let mut rewritten = Punctuated::<Meta, Token![,]>::new();
                let mut truths = Vec::new();
                for arg in &args {
                    let (arg, truth) = self.substitute(arg);
                    rewritten.push(arg);
                    truths.push(truth);
                }
                let truth = if list.path.is_ident("not") {
                    match truths.as_slice() {
                        [Truth::True] => Truth::False,
                        [Truth::False] => Truth::True,
                        _ => Truth::Unknown,
                    }
                } else if list.path.is_ident("all") {
                    if truths.contains(&Truth::False) {
                        Truth::False
                    } else if truths.contains(&Truth::Unknown) {
                        Truth::Unknown
                    } else {
                        Truth::True
                    }
                } else if truths.contains(&Truth::True) {
                    Truth::True
                } else if truths.contains(&Truth::Unknown) {
                    Truth::Unknown
                } else {
                    Truth::False
                };
                let path = &list.path;
                (syn::parse_quote!(#path(#rewritten)), truth)
            }
            other => (other.clone(), Truth::Unknown),
        }
    }

    /// Evaluate the feature predicates among `attrs`. Returns `False` when a
    /// `cfg` is false (the node must be removed); otherwise rewrites the
    /// attributes in place and returns `True` (or `Unknown` if an
    /// unevaluated `cfg` remains).
    fn filter_attrs(&mut self, attrs: &mut Vec<Attribute>) -> Truth {
        if !attrs.iter().any(|attr| {
            (attr.path().is_ident("cfg") || attr.path().is_ident("cfg_attr"))
                && attr.to_token_stream().to_string().contains("feature")
        }) {
            return Truth::True;
        }
        let mut result = Truth::True;
        let mut kept = Vec::with_capacity(attrs.len());
        for attr in std::mem::take(attrs) {
            let is_cfg = attr.path().is_ident("cfg");
            let is_cfg_attr = attr.path().is_ident("cfg_attr");
            let Meta::List(list) = &attr.meta else {
                kept.push(attr);
                continue;
            };
            if !(is_cfg || is_cfg_attr) {
                kept.push(attr);
                continue;
            }
            let Some(args) = parse_meta_list(list.tokens.clone()) else {
                kept.push(attr);
                continue;
            };
            let Some(predicate) = args.first() else {
                kept.push(attr);
                continue;
            };
            if !meta_mentions_feature(predicate) {
                kept.push(attr);
                continue;
            }
            self.changed = true;
            let (rewritten, truth) = self.substitute(predicate);
            if is_cfg {
                match truth {
                    Truth::True => {}
                    Truth::False => result = Truth::False,
                    Truth::Unknown => {
                        if result == Truth::True {
                            result = Truth::Unknown;
                        }
                        kept.push(syn::parse_quote!(#[cfg(#rewritten)]));
                    }
                }
            } else {
                let rest = args.iter().skip(1).cloned().collect::<Vec<_>>();
                match truth {
                    Truth::True => {
                        for meta in rest {
                            let mut new_attr: Attribute = syn::parse_quote!(#[#meta]);
                            new_attr.style = attr.style;
                            kept.push(new_attr);
                        }
                    }
                    Truth::False => {}
                    Truth::Unknown => {
                        let rest = rest.into_iter().collect::<Punctuated<Meta, Token![,]>>();
                        let mut new_attr: Attribute =
                            syn::parse_quote!(#[cfg_attr(#rewritten, #rest)]);
                        new_attr.style = attr.style;
                        kept.push(new_attr);
                    }
                }
            }
        }
        *attrs = kept;
        result
    }

    fn keep<T>(&mut self, node: &mut T, attrs: fn(&mut T) -> Option<&mut Vec<Attribute>>) -> bool {
        match attrs(node) {
            Some(attrs) => self.filter_attrs(attrs) != Truth::False,
            None => true,
        }
    }
}

fn item_attrs_mut(item: &mut Item) -> Option<&mut Vec<Attribute>> {
    Some(match item {
        Item::Const(item) => &mut item.attrs,
        Item::Enum(item) => &mut item.attrs,
        Item::ExternCrate(item) => &mut item.attrs,
        Item::Fn(item) => &mut item.attrs,
        Item::ForeignMod(item) => &mut item.attrs,
        Item::Impl(item) => &mut item.attrs,
        Item::Macro(item) => &mut item.attrs,
        Item::Mod(item) => &mut item.attrs,
        Item::Static(item) => &mut item.attrs,
        Item::Struct(item) => &mut item.attrs,
        Item::Trait(item) => &mut item.attrs,
        Item::TraitAlias(item) => &mut item.attrs,
        Item::Type(item) => &mut item.attrs,
        Item::Union(item) => &mut item.attrs,
        Item::Use(item) => &mut item.attrs,
        _ => return None,
    })
}

fn impl_item_attrs_mut(item: &mut syn::ImplItem) -> Option<&mut Vec<Attribute>> {
    Some(match item {
        syn::ImplItem::Const(item) => &mut item.attrs,
        syn::ImplItem::Fn(item) => &mut item.attrs,
        syn::ImplItem::Type(item) => &mut item.attrs,
        syn::ImplItem::Macro(item) => &mut item.attrs,
        _ => return None,
    })
}

fn trait_item_attrs_mut(item: &mut syn::TraitItem) -> Option<&mut Vec<Attribute>> {
    Some(match item {
        syn::TraitItem::Const(item) => &mut item.attrs,
        syn::TraitItem::Fn(item) => &mut item.attrs,
        syn::TraitItem::Type(item) => &mut item.attrs,
        syn::TraitItem::Macro(item) => &mut item.attrs,
        _ => return None,
    })
}

fn foreign_item_attrs_mut(item: &mut syn::ForeignItem) -> Option<&mut Vec<Attribute>> {
    Some(match item {
        syn::ForeignItem::Fn(item) => &mut item.attrs,
        syn::ForeignItem::Static(item) => &mut item.attrs,
        syn::ForeignItem::Type(item) => &mut item.attrs,
        syn::ForeignItem::Macro(item) => &mut item.attrs,
        _ => return None,
    })
}

fn stmt_attrs_mut(stmt: &mut syn::Stmt) -> Option<&mut Vec<Attribute>> {
    match stmt {
        syn::Stmt::Local(local) => Some(&mut local.attrs),
        syn::Stmt::Item(item) => item_attrs_mut(item),
        syn::Stmt::Expr(expr, _) => expr_attrs_mut(expr),
        syn::Stmt::Macro(mac) => Some(&mut mac.attrs),
    }
}

fn expr_attrs_mut(expr: &mut Expr) -> Option<&mut Vec<Attribute>> {
    Some(match expr {
        Expr::Array(e) => &mut e.attrs,
        Expr::Assign(e) => &mut e.attrs,
        Expr::Async(e) => &mut e.attrs,
        Expr::Await(e) => &mut e.attrs,
        Expr::Binary(e) => &mut e.attrs,
        Expr::Block(e) => &mut e.attrs,
        Expr::Break(e) => &mut e.attrs,
        Expr::Call(e) => &mut e.attrs,
        Expr::Cast(e) => &mut e.attrs,
        Expr::Closure(e) => &mut e.attrs,
        Expr::Const(e) => &mut e.attrs,
        Expr::Continue(e) => &mut e.attrs,
        Expr::Field(e) => &mut e.attrs,
        Expr::ForLoop(e) => &mut e.attrs,
        Expr::Group(e) => &mut e.attrs,
        Expr::If(e) => &mut e.attrs,
        Expr::Index(e) => &mut e.attrs,
        Expr::Infer(e) => &mut e.attrs,
        Expr::Let(e) => &mut e.attrs,
        Expr::Lit(e) => &mut e.attrs,
        Expr::Loop(e) => &mut e.attrs,
        Expr::Macro(e) => &mut e.attrs,
        Expr::Match(e) => &mut e.attrs,
        Expr::MethodCall(e) => &mut e.attrs,
        Expr::Paren(e) => &mut e.attrs,
        Expr::Path(e) => &mut e.attrs,
        Expr::Range(e) => &mut e.attrs,
        Expr::RawAddr(e) => &mut e.attrs,
        Expr::Reference(e) => &mut e.attrs,
        Expr::Repeat(e) => &mut e.attrs,
        Expr::Return(e) => &mut e.attrs,
        Expr::Struct(e) => &mut e.attrs,
        Expr::Try(e) => &mut e.attrs,
        Expr::TryBlock(e) => &mut e.attrs,
        Expr::Tuple(e) => &mut e.attrs,
        Expr::Unary(e) => &mut e.attrs,
        Expr::Unsafe(e) => &mut e.attrs,
        Expr::While(e) => &mut e.attrs,
        Expr::Yield(e) => &mut e.attrs,
        _ => return None,
    })
}

fn fn_arg_attrs_mut(arg: &mut syn::FnArg) -> Option<&mut Vec<Attribute>> {
    Some(match arg {
        syn::FnArg::Receiver(receiver) => &mut receiver.attrs,
        syn::FnArg::Typed(typed) => &mut typed.attrs,
    })
}

fn retain_punctuated<T, P: Default>(
    list: &mut Punctuated<T, P>,
    mut keep: impl FnMut(&mut T) -> bool,
) {
    let old = std::mem::take(list);
    let mut new = Punctuated::new();
    for mut value in old.into_iter() {
        if keep(&mut value) {
            new.push(value);
        }
    }
    *list = new;
}

impl VisitMut for FeatureEvaluator<'_> {
    fn visit_file_mut(&mut self, file: &mut syn::File) {
        file.items.retain_mut(|item| self.keep(item, item_attrs_mut));
        visit_mut::visit_file_mut(self, file);
    }

    fn visit_item_mod_mut(&mut self, module: &mut syn::ItemMod) {
        if let Some((_, items)) = &mut module.content {
            items.retain_mut(|item| self.keep(item, item_attrs_mut));
        }
        visit_mut::visit_item_mod_mut(self, module);
    }

    fn visit_item_impl_mut(&mut self, item: &mut syn::ItemImpl) {
        item.items.retain_mut(|item| self.keep(item, impl_item_attrs_mut));
        visit_mut::visit_item_impl_mut(self, item);
    }

    fn visit_item_trait_mut(&mut self, item: &mut syn::ItemTrait) {
        item.items.retain_mut(|item| self.keep(item, trait_item_attrs_mut));
        visit_mut::visit_item_trait_mut(self, item);
    }

    fn visit_item_foreign_mod_mut(&mut self, item: &mut syn::ItemForeignMod) {
        item.items.retain_mut(|item| self.keep(item, foreign_item_attrs_mut));
        visit_mut::visit_item_foreign_mod_mut(self, item);
    }

    fn visit_fields_named_mut(&mut self, fields: &mut syn::FieldsNamed) {
        retain_punctuated(&mut fields.named, |field| self.filter_attrs(&mut field.attrs) != Truth::False);
        visit_mut::visit_fields_named_mut(self, fields);
    }

    fn visit_fields_unnamed_mut(&mut self, fields: &mut syn::FieldsUnnamed) {
        retain_punctuated(&mut fields.unnamed, |field| {
            self.filter_attrs(&mut field.attrs) != Truth::False
        });
        visit_mut::visit_fields_unnamed_mut(self, fields);
    }

    fn visit_item_enum_mut(&mut self, item: &mut syn::ItemEnum) {
        retain_punctuated(&mut item.variants, |variant| {
            self.filter_attrs(&mut variant.attrs) != Truth::False
        });
        visit_mut::visit_item_enum_mut(self, item);
    }

    fn visit_signature_mut(&mut self, sig: &mut syn::Signature) {
        retain_punctuated(&mut sig.inputs, |arg| self.keep(arg, fn_arg_attrs_mut));
        visit_mut::visit_signature_mut(self, sig);
    }

    fn visit_block_mut(&mut self, block: &mut syn::Block) {
        block.stmts.retain_mut(|stmt| self.keep(stmt, stmt_attrs_mut));
        visit_mut::visit_block_mut(self, block);
    }

    fn visit_expr_match_mut(&mut self, expr: &mut syn::ExprMatch) {
        expr.arms.retain_mut(|arm| self.filter_attrs(&mut arm.attrs) != Truth::False);
        visit_mut::visit_expr_match_mut(self, expr);
    }

    fn visit_expr_struct_mut(&mut self, expr: &mut syn::ExprStruct) {
        retain_punctuated(&mut expr.fields, |field| {
            self.filter_attrs(&mut field.attrs) != Truth::False
        });
        visit_mut::visit_expr_struct_mut(self, expr);
    }

    fn visit_expr_mut(&mut self, expr: &mut Expr) {
        if let Expr::Macro(mac) = expr
            && mac.mac.path.is_ident("cfg")
            && let Some(args) = parse_meta_list(mac.mac.tokens.clone())
            && args.len() == 1
            && meta_mentions_feature(&args[0])
        {
            let (_, truth) = self.substitute(&args[0]);
            let value = match truth {
                Truth::True => Some(true),
                Truth::False => Some(false),
                Truth::Unknown => None,
            };
            if let Some(value) = value {
                self.changed = true;
                *expr = syn::parse_quote!(#value);
                return;
            }
        }
        visit_mut::visit_expr_mut(self, expr);
    }
}

/// Any `feature` cfg left after evaluation sits in a position the evaluator
/// does not walk.
struct FeatureResidue {
    found: Vec<String>,
}

impl<'ast> syn::visit::Visit<'ast> for FeatureResidue {
    fn visit_attribute(&mut self, attr: &'ast Attribute) {
        if (attr.path().is_ident("cfg") || attr.path().is_ident("cfg_attr"))
            && let Meta::List(list) = &attr.meta
            && let Some(args) = parse_meta_list(list.tokens.clone())
            && args.first().is_some_and(|predicate| {
                // A substituted predicate names `all()` / `any()` only.
                has_feature_test(predicate)
            })
        {
            self.found.push(attr.to_token_stream().to_string());
        }
    }
}

fn has_feature_test(meta: &Meta) -> bool {
    match meta {
        Meta::NameValue(name_value) => name_value.path.is_ident("feature"),
        Meta::Path(_) => false,
        Meta::List(list) => parse_meta_list(list.tokens.clone())
            .is_some_and(|args| args.iter().any(has_feature_test)),
    }
}

// ---------------------------------------------------------------------------
// module tree, ghost-only modules and the cross-crate surface

/// Per-crate inputs of `--crate-graph` that ride on
/// [`crate::transpile::TranspileOptions`] into the crate's source read and
/// output write.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct GraphCrateContext {
    /// Cargo's resolved feature set for this crate in the selected graph.
    pub features: BTreeSet<String>,
    /// The crate's direct graph dependencies, keyed by the extern root this
    /// crate spells them with.
    pub dependencies: BTreeMap<String, CrateSurface>,
    /// Extern root -> named C++ module to import, for every dependency that
    /// emits one.
    pub root_to_module_import: BTreeMap<String, String>,
    /// This crate's C++ module / namespace name.
    pub crate_ident: String,
    /// A dependency crate of the graph (not the root): its traits may be
    /// implemented by a consumer it never sees.
    pub dependency: bool,
}

impl GraphCrateContext {
    /// Every spec-only datatype name pruned in a dependency.
    pub fn external_pruned(&self) -> BTreeSet<String> {
        self.dependencies
            .values()
            .flat_map(|surface| surface.pruned.iter().cloned())
            .collect()
    }
}

/// What a dependency crate looks like to its dependents after `--crate-graph`
/// has processed it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CrateSurface {
    /// The crate's own extern root and C++ module / namespace name.
    pub crate_ident: String,
    /// No provider is emitted: every module was ghost-only.
    pub ghost: bool,
    /// Stage 1 erased a `verus!` block in this crate.
    pub erased: bool,
    /// Module paths (below the crate root) that are not part of the output:
    /// compiled out by a feature cfg, or ghost-only.
    pub removed_modules: BTreeSet<Vec<String>>,
    /// Spec-only datatype names pruned by stage 2, here or in a dependency.
    pub pruned: BTreeSet<String>,
    /// Every item name the crate defines or re-exports after lowering (and,
    /// through a glob re-export of a dependency, that dependency's names).
    pub names: BTreeSet<String>,
    /// `names` is exhaustive: no item-position macro could have produced a
    /// name it does not list.
    pub names_complete: bool,
    /// `names` without module names: what a consumer's code can name to
    /// need this crate (see [`live_dependencies`]).
    pub provides: BTreeSet<String>,
}

/// The module path of a crate source identity (`src/a/b.rs`, `src/a/mod.rs`
/// -> `[a, b]`, `[a]`; the crate root -> `[]`).
pub fn module_path_of_identity(identity: &Path) -> Vec<String> {
    let text = identity.to_string_lossy().replace('\\', "/");
    let relative = text.strip_prefix("src/").unwrap_or(&text);
    let without_ext = relative.strip_suffix(".rs").unwrap_or(relative);
    if matches!(without_ext, "lib" | "main") {
        return Vec::new();
    }
    let normalized = without_ext.strip_suffix("/mod").unwrap_or(without_ext);
    normalized.split('/').map(str::to_string).collect()
}

/// One crate source unit while `--crate-graph` processes it.
struct Unit {
    identity: PathBuf,
    path: Vec<String>,
    file: syn::File,
    original_text: String,
    changed: bool,
}

/// The result of [`prune_crate_units`].
pub struct PrunedCrate {
    /// Surviving units, in input order; a unit's text is byte-identical to
    /// its input unless something in it was removed.
    pub units: Vec<(PathBuf, String)>,
    pub surface: CrateSurface,
}

/// Inputs for [`prune_crate_units`] beyond the units themselves.
pub struct PruneContext<'a> {
    /// Labels messages (the package name).
    pub crate_label: &'a str,
    /// The crate's extern root / C++ module name.
    pub crate_ident: &'a str,
    /// Stage 1 erased a `verus!` block in this crate: modules left without
    /// executable content are ghost-only and removed.
    pub erased: bool,
    /// Units whose inner `#![cfg(feature ...)]` evaluated false.
    pub disabled: &'a BTreeSet<PathBuf>,
    /// This crate's direct dependencies, by the extern root this crate uses.
    pub dependencies: &'a BTreeMap<String, CrateSurface>,
    /// Stage 2's pruned datatype names for this crate.
    pub pruned: &'a BTreeSet<String>,
}

/// Build the crate's module tree from its units, drop units no `mod`
/// declaration reaches (compiled out by a feature cfg), drop `use` leaves
/// that name removed modules, ghost-only dependencies, pruned datatypes or
/// erased dependency items, and (for an erased crate) remove every module
/// left without executable content. Returns the surviving units and the
/// crate's [`CrateSurface`].
pub fn prune_crate_units(
    units: Vec<(PathBuf, String)>,
    context: &PruneContext<'_>,
) -> Result<PrunedCrate, String> {
    let mut parsed = Vec::with_capacity(units.len());
    for (identity, text) in units {
        let file = syn::parse_file(&text).map_err(|error| {
            format!(
                "--crate-graph: {}/{}: does not parse: {error}",
                context.crate_label,
                identity.display()
            )
        })?;
        parsed.push(Unit {
            path: module_path_of_identity(&identity),
            identity,
            file,
            original_text: text,
            changed: false,
        });
    }
    let by_path = parsed
        .iter()
        .enumerate()
        .map(|(index, unit)| (unit.path.clone(), index))
        .collect::<BTreeMap<_, _>>();
    let root = by_path.get(&Vec::new()).copied().ok_or_else(|| {
        format!("--crate-graph: {}: no crate root (src/lib.rs)", context.crate_label)
    })?;

    // Reachability through out-of-line `mod` declarations. A declaration of a
    // unit whose own inner cfg is false is removed with it.
    let mut removed_modules = BTreeSet::new();
    let mut reachable = BTreeSet::new();
    let mut pending = vec![root];
    while let Some(index) = pending.pop() {
        if !reachable.insert(index) {
            continue;
        }
        let unit_path = parsed[index].path.clone();
        let mut declared = Vec::new();
        collect_out_of_line_mods(&parsed[index].file.items, &unit_path, &mut declared);
        for child in declared {
            match by_path.get(&child) {
                Some(&child_index) if context.disabled.contains(&parsed[child_index].identity) => {
                    removed_modules.insert(child.clone());
                }
                Some(&child_index) => pending.push(child_index),
                None => {
                    return Err(format!(
                        "--crate-graph: {}: `mod {}` has no source file under src/",
                        context.crate_label,
                        child.join("::")
                    ));
                }
            }
        }
    }
    for unit in &parsed {
        let index = by_path[&unit.path];
        if !reachable.contains(&index) && index != root {
            removed_modules.insert(unit.path.clone());
        }
    }
    let mut module_paths = BTreeSet::new();
    for &index in &reachable {
        module_paths.insert(parsed[index].path.clone());
        collect_inline_module_paths(&parsed[index].file.items, &parsed[index].path, &mut module_paths);
    }

    // Fixpoint: filter imports against what is removed so far, then remove
    // modules left without content (erased crates only).
    let mut errors = Vec::new();
    loop {
        let mut progress = false;
        for &index in &reachable {
            let unit = &mut parsed[index];
            if removed_modules.contains(&unit.path) {
                continue;
            }
            let unit_path = unit.path.clone();
            let filter = UseFilter {
                module_paths: &module_paths,
                removed_modules: &removed_modules,
                dependencies: context.dependencies,
            };
            let mut dropped = Vec::new();
            if filter_module_items(&mut unit.file.items, &unit_path, &filter, &mut dropped) {
                unit.changed = true;
                progress = true;
            }
            audit_dropped_names(&unit.file, &dropped, context, &unit.identity, &mut errors);
            if remove_out_of_line_mods(&mut unit.file.items, &unit_path, &removed_modules) {
                unit.changed = true;
                progress = true;
            }
            if context.erased {
                let mut newly_removed = Vec::new();
                if prune_empty_inline_mods(&mut unit.file.items, &unit_path, &mut newly_removed) {
                    unit.changed = true;
                    progress = true;
                }
                removed_modules.extend(newly_removed);
                if index != root && !items_have_content(&unit.file.items) {
                    removed_modules.insert(unit_path.clone());
                    progress = true;
                }
            }
        }
        if !progress {
            break;
        }
    }
    if !errors.is_empty() {
        return Err(errors.join("; "));
    }

    let ghost = context.erased && !items_have_content(&parsed[root].file.items);
    let mut names = BTreeSet::new();
    let mut module_names = BTreeSet::new();
    let mut names_complete = true;
    let mut out = Vec::new();
    for (index, unit) in parsed.into_iter().enumerate() {
        if !reachable.contains(&index) || removed_modules.contains(&unit.path) {
            continue;
        }
        if let Some(last) = unit.path.last() {
            module_names.insert(last.clone());
        }
        collect_surface_names(
            &unit.file.items,
            context.dependencies,
            &mut names,
            &mut module_names,
            &mut names_complete,
        );
        let text = if unit.changed {
            prettyplease::unparse(&unit.file)
        } else {
            unit.original_text
        };
        out.push((unit.identity, text));
    }
    let mut pruned = context.pruned.clone();
    for dependency in context.dependencies.values() {
        pruned.extend(dependency.pruned.iter().cloned());
    }
    Ok(PrunedCrate {
        units: if ghost { Vec::new() } else { out },
        surface: CrateSurface {
            crate_ident: context.crate_ident.to_string(),
            ghost,
            erased: context.erased,
            removed_modules,
            pruned,
            provides: names.difference(&module_names).cloned().collect(),
            names,
            names_complete,
        },
    })
}

/// What one crate's surviving source needs from its dependencies.
#[derive(Clone, Debug, Default)]
pub struct CrateUsage {
    /// Identifiers anywhere in executable code (every item but `use`).
    pub exec_idents: BTreeSet<String>,
    /// Extern root -> names of surviving private, explicit (non-glob)
    /// imports from that dependency: a trait imported for its methods, or a
    /// type the code names, is needed even when the use site does not spell
    /// the dependency's name.
    pub imports_from: BTreeMap<String, BTreeSet<String>>,
}

/// [`CrateUsage`] of a crate's surviving units.
pub fn crate_usage(
    units: &[(PathBuf, String)],
    dependencies: &BTreeMap<String, CrateSurface>,
) -> Result<CrateUsage, String> {
    let mut usage = CrateUsage::default();
    for (identity, text) in units {
        let file = syn::parse_file(text)
            .map_err(|error| format!("--crate-graph: {}: does not parse: {error}", identity.display()))?;
        collect_usage(&file.items, dependencies, &mut usage);
    }
    Ok(usage)
}

fn collect_usage(items: &[Item], dependencies: &BTreeMap<String, CrateSurface>, usage: &mut CrateUsage) {
    for item in items {
        match item {
            Item::Use(item_use) => {
                if !matches!(item_use.vis, syn::Visibility::Inherited) {
                    continue;
                }
                let mut prefix = Vec::new();
                collect_import_usage(&item_use.tree, &mut prefix, dependencies, usage);
            }
            Item::Mod(module) => {
                if let Some((_, nested)) = &module.content {
                    collect_usage(nested, dependencies, usage);
                }
            }
            other => collect_idents(other.to_token_stream(), &mut usage.exec_idents),
        }
    }
}

fn collect_import_usage(
    tree: &syn::UseTree,
    prefix: &mut Vec<String>,
    dependencies: &BTreeMap<String, CrateSurface>,
    usage: &mut CrateUsage,
) {
    let mut record = |prefix: &[String], name: &str| {
        if let Some(root) = prefix.first()
            && dependencies.contains_key(root)
        {
            usage
                .imports_from
                .entry(root.clone())
                .or_default()
                .insert(name.to_string());
        }
    };
    match tree {
        syn::UseTree::Path(path) => {
            prefix.push(path.ident.to_string());
            collect_import_usage(&path.tree, prefix, dependencies, usage);
            prefix.pop();
        }
        syn::UseTree::Name(name) if name.ident != "self" => record(prefix, &name.ident.to_string()),
        syn::UseTree::Rename(rename) if rename.ident != "self" => {
            record(prefix, &rename.ident.to_string())
        }
        syn::UseTree::Group(group) => {
            for tree in &group.items {
                collect_import_usage(tree, prefix, dependencies, usage);
            }
        }
        _ => {}
    }
}

/// Dead-dependency elimination over the crate graph: which dependency crates
/// the root actually needs. A crate is needed when a needed crate's
/// executable code names something it provides (an item it defines or
/// re-exports), or imports something from it explicitly. Names a consumer
/// needs that a crate only re-exports flow on to that crate's dependencies,
/// so a re-export chain keeps its source alive. Identifier-level, so it
/// over-approximates (a name collision keeps a crate); a crate nothing names
/// has no C++ user, and Rust crates have no static initialisers, so dropping
/// it is unobservable.
///
/// `dependencies_of(key)` lists `(extern root, dependency key)` edges.
pub fn live_dependencies<K: Ord + Clone>(
    root: &K,
    dependencies_of: &dyn Fn(&K) -> Vec<(String, K)>,
    usage: &BTreeMap<K, CrateUsage>,
    provides: &BTreeMap<K, BTreeSet<String>>,
) -> BTreeSet<K> {
    let empty = CrateUsage::default();
    let need_of = |key: &K, demand: &BTreeSet<String>| -> BTreeSet<String> {
        let usage = usage.get(key).unwrap_or(&empty);
        let mut need = usage.exec_idents.clone();
        for names in usage.imports_from.values() {
            need.extend(names.iter().cloned());
        }
        need.extend(demand.iter().cloned());
        need
    };
    let mut live = BTreeSet::new();
    let mut demand = BTreeMap::<K, BTreeSet<String>>::new();
    let mut queue = vec![root.clone()];
    while let Some(consumer) = queue.pop() {
        let need = need_of(&consumer, demand.get(&consumer).unwrap_or(&BTreeSet::new()));
        let consumer_usage = usage.get(&consumer).unwrap_or(&empty);
        for (extern_root, dependency) in dependencies_of(&consumer) {
            let mut flow = provides
                .get(&dependency)
                .map(|provided| provided.intersection(&need).cloned().collect::<BTreeSet<_>>())
                .unwrap_or_default();
            if let Some(imported) = consumer_usage.imports_from.get(&extern_root) {
                flow.extend(imported.iter().cloned());
            }
            if flow.is_empty() {
                continue;
            }
            let known = demand.entry(dependency.clone()).or_default();
            let grew = !flow.is_subset(known);
            known.extend(flow);
            if live.insert(dependency.clone()) || grew {
                queue.push(dependency);
            }
        }
    }
    live
}

fn collect_out_of_line_mods(items: &[Item], prefix: &[String], out: &mut Vec<Vec<String>>) {
    for item in items {
        if let Item::Mod(module) = item {
            // Compiled out (`#[cfg(test)]`, ...): not part of the crate, and
            // its `#[path]` target is not one of the crate's sources.
            if crate::codegen::CodeGen::should_skip_cfg_attrs(&module.attrs) {
                continue;
            }
            let mut path = prefix.to_vec();
            path.push(module.ident.to_string());
            match &module.content {
                None => out.push(path),
                Some((_, nested)) => collect_out_of_line_mods(nested, &path, out),
            }
        }
    }
}

fn collect_inline_module_paths(items: &[Item], prefix: &[String], out: &mut BTreeSet<Vec<String>>) {
    for item in items {
        if let Item::Mod(module) = item {
            let mut path = prefix.to_vec();
            path.push(module.ident.to_string());
            out.insert(path.clone());
            if let Some((_, nested)) = &module.content {
                collect_inline_module_paths(nested, &path, out);
            }
        }
    }
}

/// Remove `mod x;` declarations (at any inline depth) whose module path was
/// removed.
fn remove_out_of_line_mods(
    items: &mut Vec<Item>,
    prefix: &[String],
    removed: &BTreeSet<Vec<String>>,
) -> bool {
    let before = items.len();
    items.retain(|item| {
        let Item::Mod(module) = item else {
            return true;
        };
        let mut path = prefix.to_vec();
        path.push(module.ident.to_string());
        !(module.content.is_none() && removed.contains(&path))
    });
    let mut changed = items.len() != before;
    for item in items.iter_mut() {
        if let Item::Mod(module) = item
            && let Some((_, nested)) = &mut module.content
        {
            let mut path = prefix.to_vec();
            path.push(module.ident.to_string());
            changed |= remove_out_of_line_mods(nested, &path, removed);
        }
    }
    changed
}

/// Post-order removal of inline modules without content.
fn prune_empty_inline_mods(
    items: &mut Vec<Item>,
    prefix: &[String],
    removed: &mut Vec<Vec<String>>,
) -> bool {
    let mut changed = false;
    for item in items.iter_mut() {
        if let Item::Mod(module) = item
            && let Some((_, nested)) = &mut module.content
        {
            let mut path = prefix.to_vec();
            path.push(module.ident.to_string());
            changed |= prune_empty_inline_mods(nested, &path, removed);
        }
    }
    let before = items.len();
    items.retain(|item| {
        let Item::Mod(module) = item else {
            return true;
        };
        let Some((_, nested)) = &module.content else {
            return true;
        };
        if items_have_content(nested) {
            return true;
        }
        let mut path = prefix.to_vec();
        path.push(module.ident.to_string());
        removed.push(path);
        false
    });
    changed | (items.len() != before)
}

/// Whether a module body has anything a C++ provider carries: a definition,
/// an item-position macro, a surviving non-private re-export, or a
/// surviving child module. Private imports, `const _: () = ();` and items
/// compiled out by a known-false cfg are not content.
fn items_have_content(items: &[Item]) -> bool {
    items.iter().any(|item| {
        if let Some(attrs) = item_attrs(item)
            && crate::codegen::CodeGen::should_skip_cfg_attrs(attrs)
        {
            return false;
        }
        match item {
            Item::Use(item_use) => !matches!(item_use.vis, syn::Visibility::Inherited),
            Item::Mod(_) => true,
            Item::Const(constant) => {
                !(constant.ident == "_"
                    && matches!(&*constant.ty, syn::Type::Tuple(tuple) if tuple.elems.is_empty()))
            }
            Item::Verbatim(tokens) => !tokens.is_empty(),
            _ => true,
        }
    })
}

fn item_attrs(item: &Item) -> Option<&Vec<Attribute>> {
    Some(match item {
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
        _ => return None,
    })
}

struct UseFilter<'a> {
    module_paths: &'a BTreeSet<Vec<String>>,
    removed_modules: &'a BTreeSet<Vec<String>>,
    dependencies: &'a BTreeMap<String, CrateSurface>,
}

/// Where a `use` path points.
enum Target<'a> {
    /// A path inside this crate, below the root.
    Local(Vec<String>),
    /// A path into a graph dependency (its surface, the path below its root).
    Dependency(&'a CrateSurface, Vec<String>),
    /// Anything else (std, core, a registry crate, an in-scope item).
    Other,
}

impl<'a> UseFilter<'a> {
    fn is_removed_prefix(removed: &BTreeSet<Vec<String>>, path: &[String]) -> bool {
        (1..=path.len()).any(|length| removed.contains(&path[..length]))
    }

    fn resolve(&self, module: &[String], path: &[String]) -> Target<'a> {
        let Some(first) = path.first() else {
            return Target::Other;
        };
        match first.as_str() {
            "crate" => Target::Local(path[1..].to_vec()),
            "self" => {
                let mut absolute = module.to_vec();
                absolute.extend(path[1..].iter().cloned());
                Target::Local(absolute)
            }
            "super" => {
                let mut absolute = module.to_vec();
                let mut rest = path;
                while rest.first().is_some_and(|segment| segment == "super") {
                    if absolute.pop().is_none() {
                        return Target::Other;
                    }
                    rest = &rest[1..];
                }
                absolute.extend(rest.iter().cloned());
                Target::Local(absolute)
            }
            _ => {
                let mut child = module.to_vec();
                child.push(first.clone());
                if self.module_paths.contains(&child) || self.removed_modules.contains(&child) {
                    let mut absolute = module.to_vec();
                    absolute.extend(path.iter().cloned());
                    Target::Local(absolute)
                } else if let Some(surface) = self.dependencies.get(first) {
                    Target::Dependency(surface, path[1..].to_vec())
                } else {
                    Target::Other
                }
            }
        }
    }

    /// Whether one use leaf survives. `path` is the leaf's full path; for a
    /// glob it is the module path, for `self` the module itself.
    fn keep(&self, module: &[String], path: &[String], leaf: Option<&str>) -> bool {
        match self.resolve(module, path) {
            Target::Local(absolute) => !Self::is_removed_prefix(self.removed_modules, &absolute),
            Target::Dependency(surface, below) => {
                if surface.ghost || Self::is_removed_prefix(&surface.removed_modules, &below) {
                    return false;
                }
                let Some(leaf) = leaf else {
                    return true;
                };
                if surface.pruned.contains(leaf) {
                    return false;
                }
                // A name the dependency neither defines nor re-exports after
                // erasure was a spec/proof item: rustc resolved it, so its
                // absence here means Verus erased it.
                !(surface.erased && surface.names_complete && !below.is_empty() && !surface.names.contains(leaf))
            }
            Target::Other => true,
        }
    }
}

/// Filter every `use` item among `items` (recursing into inline modules);
/// returns whether anything changed. Local names of dropped named leaves are
/// pushed to `dropped`.
fn filter_module_items(
    items: &mut Vec<Item>,
    module: &[String],
    filter: &UseFilter<'_>,
    dropped: &mut Vec<String>,
) -> bool {
    let mut changed = false;
    let mut kept = Vec::with_capacity(items.len());
    for mut item in std::mem::take(items) {
        match &mut item {
            Item::Use(item_use) => {
                let mut prefix = Vec::new();
                match filter_use_tree(&item_use.tree, &mut prefix, module, filter, dropped) {
                    Some(tree) => {
                        if tree.to_token_stream().to_string() != item_use.tree.to_token_stream().to_string()
                        {
                            item_use.tree = tree;
                            changed = true;
                        }
                        kept.push(item);
                    }
                    None => changed = true,
                }
            }
            Item::Mod(inner) => {
                if let Some((_, nested)) = &mut inner.content {
                    let mut path = module.to_vec();
                    path.push(inner.ident.to_string());
                    changed |= filter_module_items(nested, &path, filter, dropped);
                }
                kept.push(item);
            }
            _ => kept.push(item),
        }
    }
    *items = kept;
    changed
}

fn filter_use_tree(
    tree: &syn::UseTree,
    prefix: &mut Vec<String>,
    module: &[String],
    filter: &UseFilter<'_>,
    dropped: &mut Vec<String>,
) -> Option<syn::UseTree> {
    match tree {
        syn::UseTree::Path(path) => {
            prefix.push(path.ident.to_string());
            let inner = filter_use_tree(&path.tree, prefix, module, filter, dropped);
            prefix.pop();
            let mut path = path.clone();
            *path.tree = inner?;
            Some(syn::UseTree::Path(path))
        }
        syn::UseTree::Name(name) => {
            let source = name.ident.to_string();
            let (full, leaf, local) = if source == "self" {
                (prefix.clone(), None, prefix.last().cloned().unwrap_or_default())
            } else {
                let mut full = prefix.clone();
                full.push(source.clone());
                (full, Some(source.clone()), source.clone())
            };
            if filter.keep(module, &full, leaf.as_deref()) {
                Some(tree.clone())
            } else {
                dropped.push(local);
                None
            }
        }
        syn::UseTree::Rename(rename) => {
            let source = rename.ident.to_string();
            let (full, leaf) = if source == "self" {
                (prefix.clone(), None)
            } else {
                let mut full = prefix.clone();
                full.push(source.clone());
                (full, Some(source.clone()))
            };
            if filter.keep(module, &full, leaf.as_deref()) {
                Some(tree.clone())
            } else {
                dropped.push(rename.rename.to_string());
                None
            }
        }
        syn::UseTree::Glob(_) => filter.keep(module, prefix, None).then(|| tree.clone()),
        syn::UseTree::Group(group) => {
            let items = group
                .items
                .iter()
                .filter_map(|tree| filter_use_tree(tree, prefix, module, filter, dropped))
                .collect::<Punctuated<syn::UseTree, Token![,]>>();
            if items.is_empty() {
                return None;
            }
            let mut group = group.clone();
            group.items = items;
            Some(syn::UseTree::Group(group))
        }
    }
}

/// A dropped import whose name executable code in the same file still uses
/// would leave an unresolved name in the C++; fail instead.
fn audit_dropped_names(
    file: &syn::File,
    dropped: &[String],
    context: &PruneContext<'_>,
    identity: &Path,
    errors: &mut Vec<String>,
) {
    if dropped.is_empty() {
        return;
    }
    let mut used = BTreeSet::new();
    for item in &file.items {
        if !matches!(item, Item::Use(_)) {
            collect_idents(item.to_token_stream(), &mut used);
        }
    }
    for name in dropped {
        if used.contains(name) {
            errors.push(format!(
                "--crate-graph: {}/{}: executable code uses `{name}`, which is imported from a module, crate or item that has no executable content after Verus erasure",
                context.crate_label,
                identity.display()
            ));
        }
    }
}

fn collect_idents(tokens: TokenStream, out: &mut BTreeSet<String>) {
    for token in tokens {
        match token {
            proc_macro2::TokenTree::Ident(ident) => {
                out.insert(ident.to_string());
            }
            proc_macro2::TokenTree::Group(group) => collect_idents(group.stream(), out),
            _ => {}
        }
    }
}

fn collect_surface_names(
    items: &[Item],
    dependencies: &BTreeMap<String, CrateSurface>,
    names: &mut BTreeSet<String>,
    module_names: &mut BTreeSet<String>,
    complete: &mut bool,
) {
    for item in items {
        match item {
            Item::Const(item) => {
                names.insert(item.ident.to_string());
            }
            Item::Enum(item) => {
                names.insert(item.ident.to_string());
            }
            Item::Fn(item) => {
                names.insert(item.sig.ident.to_string());
            }
            Item::Static(item) => {
                names.insert(item.ident.to_string());
            }
            Item::Struct(item) => {
                names.insert(item.ident.to_string());
            }
            Item::Trait(item) => {
                names.insert(item.ident.to_string());
            }
            Item::TraitAlias(item) => {
                names.insert(item.ident.to_string());
            }
            Item::Type(item) => {
                names.insert(item.ident.to_string());
            }
            Item::Union(item) => {
                names.insert(item.ident.to_string());
            }
            Item::ExternCrate(item) => {
                names.insert(
                    item.rename
                        .as_ref()
                        .map(|(_, rename)| rename.to_string())
                        .unwrap_or_else(|| item.ident.to_string()),
                );
            }
            Item::Mod(module) => {
                names.insert(module.ident.to_string());
                module_names.insert(module.ident.to_string());
                if let Some((_, nested)) = &module.content {
                    collect_surface_names(nested, dependencies, names, module_names, complete);
                }
            }
            Item::Macro(mac) => {
                if let Some(ident) = &mac.ident {
                    names.insert(ident.to_string());
                } else if mac.mac.path.is_ident("thread_local") {
                    // `thread_local! { static NAME: T = ...; }`
                    let mut after_static = false;
                    for token in mac.mac.tokens.clone() {
                        if let proc_macro2::TokenTree::Ident(ident) = token {
                            if after_static && ident != "mut" {
                                names.insert(ident.to_string());
                                after_static = false;
                            } else {
                                after_static = ident == "static";
                            }
                        }
                    }
                } else {
                    *complete = false;
                }
            }
            Item::Use(item_use) => {
                let mut prefix = Vec::new();
                collect_use_names(&item_use.tree, &mut prefix, dependencies, names);
            }
            _ => {}
        }
    }
}

fn collect_use_names(
    tree: &syn::UseTree,
    prefix: &mut Vec<String>,
    dependencies: &BTreeMap<String, CrateSurface>,
    names: &mut BTreeSet<String>,
) {
    match tree {
        syn::UseTree::Path(path) => {
            prefix.push(path.ident.to_string());
            collect_use_names(&path.tree, prefix, dependencies, names);
            prefix.pop();
        }
        syn::UseTree::Name(name) => {
            if name.ident == "self" {
                if let Some(last) = prefix.last() {
                    names.insert(last.clone());
                }
            } else {
                names.insert(name.ident.to_string());
            }
        }
        syn::UseTree::Rename(rename) => {
            names.insert(rename.rename.to_string());
        }
        syn::UseTree::Glob(_) => {
            if let Some(surface) = prefix.first().and_then(|root| dependencies.get(root)) {
                names.extend(surface.provides.iter().cloned());
            }
        }
        syn::UseTree::Group(group) => {
            for tree in &group.items {
                collect_use_names(tree, prefix, dependencies, names);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// one module per dependency crate

/// Inline every out-of-line `mod x;` of the crate's surviving units into one
/// source file rooted at the crate root, so the crate is emitted as ONE named
/// module whose Rust modules become nested namespaces. A child file's inner
/// attributes move onto its inline module.
pub fn merge_crate_units(crate_label: &str, units: &[(PathBuf, String)]) -> Result<String, String> {
    let mut files = BTreeMap::new();
    for (identity, text) in units {
        let file = syn::parse_file(text).map_err(|error| {
            format!("--crate-graph: {crate_label}/{}: does not parse: {error}", identity.display())
        })?;
        files.insert(module_path_of_identity(identity), file);
    }
    let mut root = files
        .remove(&Vec::new())
        .ok_or_else(|| format!("--crate-graph: {crate_label}: no crate root to merge"))?;
    inline_modules(&mut root.items, &[], &mut files, crate_label)?;
    if let Some(stray) = files.keys().next() {
        return Err(format!(
            "--crate-graph: {crate_label}: module `{}` is not declared by its parent",
            stray.join("::")
        ));
    }
    Ok(prettyplease::unparse(&root))
}

fn inline_modules(
    items: &mut Vec<Item>,
    prefix: &[String],
    files: &mut BTreeMap<Vec<String>, syn::File>,
    crate_label: &str,
) -> Result<(), String> {
    // A compiled-out `mod x;` (`#[cfg(test)]`) has no source among the units.
    items.retain(|item| {
        !matches!(item, Item::Mod(module)
            if module.content.is_none()
                && crate::codegen::CodeGen::should_skip_cfg_attrs(&module.attrs))
    });
    for item in items.iter_mut() {
        let Item::Mod(module) = item else {
            continue;
        };
        let mut path = prefix.to_vec();
        path.push(module.ident.to_string());
        if module.content.is_none() {
            let child = files.remove(&path).ok_or_else(|| {
                format!(
                    "--crate-graph: {crate_label}: no source for `mod {}`",
                    path.join("::")
                )
            })?;
            module.attrs.extend(child.attrs);
            module.content = Some((syn::token::Brace::default(), child.items));
            module.semi = None;
        }
        if let Some((_, nested)) = &mut module.content {
            inline_modules(nested, &path, files, crate_label)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn features(names: &[&str]) -> BTreeSet<String> {
        names.iter().map(|name| name.to_string()).collect()
    }

    fn evaluate(source: &str, enabled: &[&str]) -> String {
        match apply_feature_cfgs(source, &features(enabled)).unwrap() {
            Some((text, _)) => text,
            None => source.to_string(),
        }
    }

    fn normalized(source: &str) -> String {
        prettyplease::unparse(&syn::parse_file(source).unwrap())
    }

    #[test]
    fn feature_cfgs_follow_the_resolved_feature_set() {
        let source = r#"
#[cfg(feature = "mio")]
mod mio_backend;
#[cfg(not(feature = "mio"))]
fn backend() -> u8 { 0 }
#[cfg(feature = "mio")]
fn backend() -> u8 { 1 }
pub struct S {
    #[cfg(feature = "mio")]
    pub poll: u8,
    pub fd: i32,
}
pub enum E {
    #[cfg(feature = "mio")]
    Mio,
    Plain,
}
impl S {
    #[cfg(feature = "mio")]
    pub fn new() -> Self { todo!() }
    pub fn fd(&self) -> i32 {
        #[cfg(feature = "mio")]
        let x = 1;
        let on = cfg!(feature = "mio");
        match self.fd {
            #[cfg(feature = "mio")]
            1 => 2,
            _ => 3,
        }
    }
}
#[cfg_attr(feature = "mio", derive(Debug))]
#[cfg_attr(not(feature = "mio"), derive(Clone))]
pub struct T;
#[cfg(all(feature = "mio", unix))]
fn both() {}
#[cfg(any(feature = "mio", unix))]
fn either() {}
"#;
        let off = evaluate(source, &[]);
        let expected_off = r#"
#[cfg(not(feature = "unused"))]
fn backend() -> u8 { 0 }
pub struct S {
    pub fd: i32,
}
pub enum E {
    Plain,
}
impl S {
    pub fn fd(&self) -> i32 {
        let on = false;
        match self.fd {
            _ => 3,
        }
    }
}
#[derive(Clone)]
pub struct T;
#[cfg(any(any(), unix))]
fn either() {}
"#
        .replace("#[cfg(not(feature = \"unused\"))]\n", "");
        assert_eq!(off, normalized(&expected_off));

        let on = evaluate(source, &["mio"]);
        assert!(on.contains("mod mio_backend;"), "{on}");
        assert!(on.contains("fn backend() -> u8 {\n    1\n}"), "{on}");
        assert!(!on.contains("fn backend() -> u8 {\n    0\n}"), "{on}");
        assert!(on.contains("pub poll: u8"), "{on}");
        assert!(on.contains("let on = true;"), "{on}");
        assert!(on.contains("#[derive(Debug)]\npub struct T;"), "{on}");
        assert!(on.contains("#[cfg(all(all(), unix))]"), "{on}");
        assert!(on.contains("fn either() {}") && !on.contains("fn either() {}\n#[cfg"), "{on}");
        assert!(!on.contains("feature"), "{on}");
    }

    #[test]
    fn sources_without_feature_cfgs_are_untouched() {
        // Emitter directives and other cfgs are not feature predicates.
        for source in [
            "#[cfg_attr(any(), cpp_abi(\"x\"))] pub fn f() {}",
            "#[cfg(test)] mod tests {}",
            "// a comment that says feature\npub fn g() {}",
        ] {
            assert!(apply_feature_cfgs(source, &features(&["mio"])).unwrap().is_none(), "{source}");
        }
    }

    #[test]
    fn a_false_inner_cfg_disables_the_module_file() {
        let (_, disabled) = apply_feature_cfgs("#![cfg(feature = \"x\")]\npub fn f() {}", &features(&[]))
            .unwrap()
            .unwrap();
        assert!(disabled);
        let (text, disabled) =
            apply_feature_cfgs("#![cfg(feature = \"x\")]\npub fn f() {}", &features(&["x"]))
                .unwrap()
                .unwrap();
        assert!(!disabled && !text.contains("cfg"), "{text}");
    }

    #[test]
    fn a_feature_cfg_in_an_unevaluated_position_fails_closed() {
        let error = apply_feature_cfgs("fn f<#[cfg(feature = \"x\")] T>() {}", &features(&[]))
            .unwrap_err();
        assert!(error.contains("does not evaluate"), "{error}");
    }

    fn unit(path: &str, text: &str) -> (PathBuf, String) {
        (PathBuf::from(path), text.to_string())
    }

    fn surface(ident: &str, ghost: bool) -> CrateSurface {
        CrateSurface {
            crate_ident: ident.to_string(),
            ghost,
            erased: true,
            names_complete: true,
            ..CrateSurface::default()
        }
    }

    #[test]
    fn ghost_modules_and_their_imports_vanish() {
        let mut spec = surface("spec_dep", false);
        spec.removed_modules.insert(vec!["invariants".to_string()]);
        spec.pruned.insert("Log".to_string());
        spec.names.extend(["events".to_string(), "Event".to_string()]);
        spec.provides.insert("Event".to_string());
        let dependencies = BTreeMap::from([
            ("spec_dep".to_string(), spec),
            ("ghost_dep".to_string(), surface("ghost_dep", true)),
        ]);
        let units = vec![
            unit(
                "src/lib.rs",
                "pub mod proof; pub mod types; mod unreached; pub use types::Waker; pub use proof::lemma; pub use spec_dep::events::Event;",
            ),
            unit(
                "src/proof/mod.rs",
                "pub mod helpers; use crate::types::*; pub use spec_dep::invariants::*; pub use spec_dep::log::Log; pub use spec_dep::events::open_spec_fn; pub use ghost_dep::*; const _: () = ();",
            ),
            unit("src/proof/helpers.rs", "use super::*;"),
            unit(
                "src/types.rs",
                "use crate::proof::helpers::*; pub struct Waker; impl Waker { pub fn wake(&self) {} }",
            ),
            unit("src/unused_file.rs", "pub fn stray() {}"),
            unit("src/unreached.rs", "pub fn gated() {}"),
        ];
        let disabled = BTreeSet::from([PathBuf::from("src/unreached.rs")]);
        let pruned_names = BTreeSet::new();
        let context = PruneContext {
            crate_label: "demo",
            crate_ident: "demo",
            erased: true,
            disabled: &disabled,
            dependencies: &dependencies,
            pruned: &pruned_names,
        };
        let pruned = prune_crate_units(units, &context).unwrap();
        let identities = pruned
            .units
            .iter()
            .map(|(identity, _)| identity.display().to_string())
            .collect::<Vec<_>>();
        assert_eq!(identities, ["src/lib.rs", "src/types.rs"]);
        let lib = &pruned.units[0].1;
        assert!(!lib.contains("proof") && !lib.contains("unreached"), "{lib}");
        assert!(lib.contains("pub use types::Waker;"), "{lib}");
        assert!(lib.contains("pub use spec_dep::events::Event;"), "{lib}");
        assert!(!pruned.units[1].1.contains("helpers"), "{}", pruned.units[1].1);
        assert!(!pruned.surface.ghost);
        for removed in [vec!["proof"], vec!["proof", "helpers"], vec!["unreached"], vec!["unused_file"]] {
            let removed = removed.into_iter().map(str::to_string).collect::<Vec<_>>();
            assert!(pruned.surface.removed_modules.contains(&removed), "{removed:?}");
        }
        assert!(pruned.surface.provides.contains("Waker") && !pruned.surface.provides.contains("types"));
    }

    #[test]
    fn a_crate_without_executable_content_is_ghost() {
        let units = vec![
            unit("src/lib.rs", "pub mod spec; pub use spec::*;"),
            unit("src/spec.rs", "use crate::*;"),
        ];
        let no_units = BTreeSet::new();
        let no_names = BTreeSet::new();
        let dependencies = BTreeMap::new();
        let context = PruneContext {
            crate_label: "spec",
            crate_ident: "spec",
            erased: true,
            disabled: &no_units,
            dependencies: &dependencies,
            pruned: &no_names,
        };
        let pruned = prune_crate_units(units, &context).unwrap();
        assert!(pruned.surface.ghost && pruned.units.is_empty());

        // The same shape in a crate without Verus erasure is left alone.
        let units = vec![
            unit("src/lib.rs", "pub mod spec; pub use spec::*;"),
            unit("src/spec.rs", "use crate::*;"),
        ];
        let context = PruneContext { erased: false, ..context };
        let pruned = prune_crate_units(units, &context).unwrap();
        assert!(!pruned.surface.ghost && pruned.units.len() == 2);
    }

    #[test]
    fn executable_use_of_a_dropped_import_fails_closed() {
        let dependencies = BTreeMap::from([("ghost_dep".to_string(), surface("ghost_dep", true))]);
        let units = vec![unit(
            "src/lib.rs",
            "use ghost_dep::Model; pub fn f() -> usize { std::mem::size_of::<Model>() }",
        )];
        let no_units = BTreeSet::new();
        let no_names = BTreeSet::new();
        let context = PruneContext {
            crate_label: "demo",
            crate_ident: "demo",
            erased: false,
            disabled: &no_units,
            dependencies: &dependencies,
            pruned: &no_names,
        };
        let error = match prune_crate_units(units, &context) {
            Ok(_) => panic!("a use of a ghost crate's item survived"),
            Err(error) => error,
        };
        assert!(error.contains("executable code uses `Model`"), "{error}");
    }

    #[test]
    fn merge_inlines_module_files_with_their_inner_attributes() {
        let units = vec![
            unit("src/lib.rs", "pub mod a; pub use a::X;"),
            unit(
                "src/a/mod.rs",
                "#![allow(dead_code)]\npub mod b; pub struct X; #[cfg(test)] #[path = \"t.rs\"] mod t;",
            ),
            unit("src/a/b.rs", "pub fn f() {}"),
        ];
        let merged = merge_crate_units("demo", &units).unwrap();
        assert_eq!(
            merged,
            normalized(
                "pub mod a { #![allow(dead_code)] pub mod b { pub fn f() {} } pub struct X; } pub use a::X;"
            )
        );
        let error = merge_crate_units("demo", &[unit("src/lib.rs", "pub mod missing;")]).unwrap_err();
        assert!(error.contains("no source for `mod missing`"), "{error}");
    }

    #[test]
    fn liveness_follows_names_imports_and_reexport_chains() {
        // root -> mid -> leaf; root names `Thing`, which mid only re-exports
        // from leaf; `unused` is named by nobody; `trait_dep` is only imported.
        let edges = BTreeMap::from([
            ("root", vec![("mid", "mid"), ("unused", "unused"), ("trait_dep", "trait_dep")]),
            ("mid", vec![("leaf", "leaf")]),
        ]);
        let dependencies_of = |key: &&str| -> Vec<(String, &str)> {
            edges
                .get(key)
                .map(|edges| edges.iter().map(|(root, key)| (root.to_string(), *key)).collect())
                .unwrap_or_default()
        };
        let mut usage = BTreeMap::new();
        usage.insert(
            "root",
            CrateUsage {
                exec_idents: ["Thing", "run"].iter().map(|s| s.to_string()).collect(),
                imports_from: BTreeMap::from([(
                    "trait_dep".to_string(),
                    BTreeSet::from(["Ext".to_string()]),
                )]),
            },
        );
        let provides = BTreeMap::from([
            ("mid", BTreeSet::from(["Thing".to_string()])),
            ("leaf", BTreeSet::from(["Thing".to_string(), "Other".to_string()])),
            ("unused", BTreeSet::from(["Nothing".to_string()])),
            ("trait_dep", BTreeSet::from(["Ext".to_string()])),
        ]);
        let live = live_dependencies(&"root", &dependencies_of, &usage, &provides);
        assert_eq!(live, BTreeSet::from(["leaf", "mid", "trait_dep"]));
    }
}
