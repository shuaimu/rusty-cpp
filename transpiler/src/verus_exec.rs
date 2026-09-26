//! `--verus-exec`: transpile the executable code inside Verus `verus! { }`
//! blocks exactly as plain rustc compiles it.
//!
//! Plain rustc never sees Verus syntax. The `verus!` proc macro runs Verus's
//! own `EraseGhost::EraseAll` rewrite and hands rustc the executable items
//! only: spec and proof functions disappear, `requires`/`ensures`, `proof { }`
//! blocks and ghost `let`s vanish, and what remains is ordinary Rust. This
//! module reproduces that expansion at the source level, before any other
//! pass reads the file, by running the *same* erasure code
//! (`verus_erase`, vendored from `verus_builtin_macros`) on every item-level
//! `verus!` invocation and splicing the resulting items in its place.
//!
//! It then evaluates the two cfgs the Verus driver sets and plain rustc never
//! does, `verus_keep_ghost` and `verus_keep_ghost_body`, as false everywhere
//! in the file (Lion uses `#![cfg_attr(verus_keep_ghost, verus::trusted)]`
//! outside `verus!`, and `#[cfg(verus_keep_ghost)]` items inside it).
//!
//! What it does not do is lower the ghost residue that survives erasure
//! (`use vstd::prelude::*`, `View` impls, `Ghost<T>` values, vstd executable
//! methods). That is the next stage's job; this stage only guarantees that
//! downstream passes see the Rust rustc sees.
//!
//! Fail-closed rules:
//! - a file that cannot be parsed, or a `verus!` block Verus's own rewrite
//!   rejects (`compile_error!` in its output), is an error;
//! - any other Verus macro anywhere in the file (`verus_impl!`,
//!   `verus_keep_ghost!`, `proof!`, `struct_with_invariants!`, a `verus!` in
//!   statement position, ...) is an error rather than a TODO slot;
//! - a `verus_keep_ghost` cfg in a position this pass does not evaluate
//!   (inside a macro invocation's tokens, on an expression, ...) is an error.
//!
//! A file without any of these constructs is returned byte for byte.

use std::borrow::Cow;
use std::path::{Path, PathBuf};

use proc_macro2::{TokenStream, TokenTree};
use syn::punctuated::Punctuated;
use syn::visit::Visit;
use syn::visit_mut::VisitMut;
use syn::{Attribute, Item, Meta, Token};

/// The Verus `builtin_macros` revision whose erasure this build runs.
#[allow(dead_code)]
pub const VERUS_BUILTIN_MACROS_VERSION: &str = verus_erase::VERUS_BUILTIN_MACROS_VERSION;
/// The verus-lang/verus commit that revision was vendored from.
#[allow(dead_code)]
pub const VERUS_GIT_REV: &str = verus_erase::VERUS_GIT_REV;

/// cfg names the Verus driver sets and plain rustc never does.
const VERUS_DRIVER_CFGS: &[&str] = &["verus_keep_ghost", "verus_keep_ghost_body"];

/// `--verus-exec` configuration carried on `TranspileOptions`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct VerusExecConfig {
    /// Apply the pre-pass to every crate source before any other pass reads it.
    pub enabled: bool,
    /// `--dump-verus-erasure`: write each source unit, as handed downstream,
    /// to `<dir>/<crate name>/<source identity>`.
    pub dump_dir: Option<PathBuf>,
}

/// Apply `--verus-exec` to one source text (identity when disabled).
pub fn prepare_source(config: &VerusExecConfig, source: String) -> Result<String, String> {
    if !config.enabled {
        return Ok(source);
    }
    match erase_source(&source)? {
        Cow::Borrowed(_) => Ok(source),
        Cow::Owned(erased) => Ok(erased),
    }
}

/// Write one prepared source unit to the `--dump-verus-erasure` directory.
pub fn dump_source(
    config: &VerusExecConfig,
    crate_name: &str,
    identity: &Path,
    prepared: &str,
) -> Result<(), String> {
    let Some(dir) = config.dump_dir.as_ref() else {
        return Ok(());
    };
    let path = dir.join(crate_name).join(identity);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|error| format!("could not create {}: {error}", parent.display()))?;
    }
    std::fs::write(&path, prepared)
        .map_err(|error| format!("could not write {}: {error}", path.display()))
}

/// Erase every item-level `verus!` block in `source` and evaluate the Verus
/// driver cfgs as false. Returns the input unchanged (borrowed) when there is
/// nothing to do.
pub fn erase_source(source: &str) -> Result<Cow<'_, str>, String> {
    // Cheap screen: every construct this pass acts on or rejects spells one
    // of these names. A file that merely mentions one (a comment, a
    // same-named user item) is parsed and audited, then returned unchanged.
    if !VERUS_DRIVER_CFGS
        .iter()
        .chain(VERUS_MACROS)
        .chain(VERUS_MACRO_CRATES)
        .any(|name| source.contains(name))
    {
        return Ok(Cow::Borrowed(source));
    }
    let mut file = syn::parse_file(source)
        .map_err(|error| format!("--verus-exec could not parse the source: {error}"))?;
    let mut changed = false;

    changed |= erase_items(&mut file.items, "crate")?;

    let mut cfg = DriverCfgPass::default();
    cfg.visit_file_mut(&mut file);
    if let Some(error) = cfg.error {
        return Err(error);
    }
    changed |= cfg.changed;

    let mut audit = ResidueAudit::default();
    audit.visit_file(&file);
    if !audit.errors.is_empty() {
        return Err(audit.errors.join("; "));
    }

    if changed {
        Ok(Cow::Owned(prettyplease::unparse(&file)))
    } else {
        Ok(Cow::Borrowed(source))
    }
}

// ---------------------------------------------------------------------------
// verus! erasure

/// The item-level macro paths that name `verus_builtin_macros::verus`.
fn is_verus_items_macro(path: &syn::Path) -> bool {
    let segments = path
        .segments
        .iter()
        .map(|segment| segment.ident.to_string())
        .collect::<Vec<_>>();
    let segments = segments.iter().map(String::as_str).collect::<Vec<_>>();
    matches!(
        segments.as_slice(),
        ["verus"]
            | ["vstd", "prelude", "verus"]
            | ["vstd", "verus"]
            | ["verus_builtin_macros", "verus"]
            | ["builtin_macros", "verus"]
    ) && path
        .segments
        .iter()
        .all(|segment| segment.arguments.is_none())
}

/// Replace every `verus! { ... }` in `items` (and in inline `mod` bodies) with
/// the items Verus's EraseAll rewrite produces. Returns whether anything
/// changed.
fn erase_items(items: &mut Vec<Item>, scope: &str) -> Result<bool, String> {
    let mut changed = false;
    let mut index = 0;
    while index < items.len() {
        match &mut items[index] {
            Item::Macro(item_macro)
                if item_macro.ident.is_none() && is_verus_items_macro(&item_macro.mac.path) =>
            {
                if !item_macro.attrs.is_empty() {
                    return Err(format!(
                        "--verus-exec: attributes on a `verus!` invocation in `{scope}` are unsupported"
                    ));
                }
                let erased = verus_erase::erase_items(item_macro.mac.tokens.clone()).map_err(
                    |error| format!("--verus-exec: Verus rejected a `verus!` block in `{scope}`: {error}"),
                )?;
                let erased: syn::File = syn::parse2(erased).map_err(|error| {
                    format!(
                        "--verus-exec: the erasure of a `verus!` block in `{scope}` is not Rust syn can parse: {error}"
                    )
                })?;
                if !erased.attrs.is_empty() {
                    return Err(format!(
                        "--verus-exec: the erasure of a `verus!` block in `{scope}` produced inner attributes"
                    ));
                }
                let replacement = erased.items;
                let count = replacement.len();
                items.splice(index..=index, replacement);
                changed = true;
                // Verus's rewrite already recursed into inline modules inside
                // the block; revisit them only so a nested `verus!` there is
                // handled like any other. A `verus!` at the spliced level
                // itself is not re-expanded (rustc would); `ResidueAudit`
                // rejects it.
                let end = index + count;
                while index < end {
                    if let Item::Mod(module) = &mut items[index] {
                        let name = format!("{scope}::{}", module.ident);
                        if let Some((_, content)) = &mut module.content {
                            changed |= erase_items(content, &name)?;
                        }
                    }
                    index += 1;
                }
                continue;
            }
            Item::Mod(module) => {
                let name = format!("{scope}::{}", module.ident);
                if let Some((_, content)) = &mut module.content {
                    changed |= erase_items(content, &name)?;
                }
            }
            _ => {}
        }
        index += 1;
    }
    Ok(changed)
}

// ---------------------------------------------------------------------------
// verus_keep_ghost / verus_keep_ghost_body evaluation

/// Result of partially evaluating a cfg predicate with the Verus driver cfgs
/// fixed to false.
enum Partial {
    True,
    False,
    /// The predicate still depends on other cfgs; this is its simplified form,
    /// free of Verus driver cfgs.
    Residual(Meta),
}

fn meta_mentions_driver_cfg(tokens: TokenStream) -> bool {
    tokens.into_iter().any(|tree| match tree {
        TokenTree::Ident(ident) => VERUS_DRIVER_CFGS.iter().any(|name| ident == name),
        TokenTree::Group(group) => meta_mentions_driver_cfg(group.stream()),
        _ => false,
    })
}

fn cfg_args(list: &syn::MetaList) -> Result<Vec<Meta>, String> {
    list.parse_args_with(Punctuated::<Meta, Token![,]>::parse_terminated)
        .map(|args| args.into_iter().collect())
        .map_err(|error| format!("malformed cfg predicate `{}`: {error}", quote::quote!(#list)))
}

fn partial_eval(meta: &Meta) -> Result<Partial, String> {
    match meta {
        Meta::Path(path)
            if VERUS_DRIVER_CFGS
                .iter()
                .any(|name| path.is_ident(name)) =>
        {
            Ok(Partial::False)
        }
        Meta::Path(_) | Meta::NameValue(_) => Ok(Partial::Residual(meta.clone())),
        Meta::List(list) if list.path.is_ident("all") || list.path.is_ident("any") => {
            let is_all = list.path.is_ident("all");
            let mut residuals = Vec::new();
            for arg in cfg_args(list)? {
                match partial_eval(&arg)? {
                    Partial::True if is_all => {}
                    Partial::False if !is_all => {}
                    Partial::True => return Ok(Partial::True),
                    Partial::False => return Ok(Partial::False),
                    Partial::Residual(residual) => residuals.push(residual),
                }
            }
            Ok(match residuals.len() {
                0 if is_all => Partial::True,
                0 => Partial::False,
                1 => Partial::Residual(residuals.pop().unwrap()),
                _ => {
                    let path = &list.path;
                    Partial::Residual(syn::parse_quote!(#path(#(#residuals),*)))
                }
            })
        }
        Meta::List(list) if list.path.is_ident("not") => {
            let mut args = cfg_args(list)?;
            if args.len() != 1 {
                return Err(format!(
                    "malformed cfg predicate `{}`: `not` takes one argument",
                    quote::quote!(#list)
                ));
            }
            Ok(match partial_eval(&args.remove(0))? {
                Partial::True => Partial::False,
                Partial::False => Partial::True,
                Partial::Residual(residual) => Partial::Residual(syn::parse_quote!(not(#residual))),
            })
        }
        Meta::List(_) => Ok(Partial::Residual(meta.clone())),
    }
}

/// What to do with the node that owns an attribute list.
enum AttrsVerdict {
    Keep,
    /// A `#[cfg]` on the node evaluated to false: drop the node.
    Remove,
}

/// Rewrite one attribute list: evaluate `cfg` / `cfg_attr` predicates that
/// mention a Verus driver cfg, leaving every other attribute untouched.
fn process_attrs(attrs: &mut Vec<Attribute>, changed: &mut bool) -> Result<AttrsVerdict, String> {
    let mut output = Vec::with_capacity(attrs.len());
    let mut verdict = AttrsVerdict::Keep;
    for attribute in attrs.drain(..) {
        let Meta::List(list) = &attribute.meta else {
            output.push(attribute);
            continue;
        };
        let is_cfg = list.path.is_ident("cfg");
        let is_cfg_attr = list.path.is_ident("cfg_attr");
        if !(is_cfg || is_cfg_attr) || !meta_mentions_driver_cfg(list.tokens.clone()) {
            output.push(attribute);
            continue;
        }
        *changed = true;
        if is_cfg {
            let predicate = list
                .parse_args::<Meta>()
                .map_err(|error| format!("malformed `#[cfg(..)]`: {error}"))?;
            match partial_eval(&predicate)? {
                Partial::True => {}
                Partial::False => verdict = AttrsVerdict::Remove,
                Partial::Residual(residual) => {
                    let mut kept = attribute.clone();
                    kept.meta = syn::parse_quote!(cfg(#residual));
                    output.push(kept);
                }
            }
            continue;
        }
        // cfg_attr(predicate, attr1, attr2, ...)
        let mut args = list
            .parse_args_with(Punctuated::<Meta, Token![,]>::parse_terminated)
            .map_err(|error| format!("malformed `#[cfg_attr(..)]`: {error}"))?
            .into_iter();
        let Some(predicate) = args.next() else {
            return Err("malformed `#[cfg_attr(..)]`: missing predicate".to_string());
        };
        let rest = args.collect::<Vec<_>>();
        match partial_eval(&predicate)? {
            Partial::False => {}
            Partial::True => {
                // Rustc expands a true cfg_attr into its attributes, which
                // may themselves be cfgs; re-process them in place.
                let mut expanded = rest
                    .into_iter()
                    .map(|meta| Attribute {
                        pound_token: attribute.pound_token,
                        style: attribute.style,
                        bracket_token: attribute.bracket_token,
                        meta,
                    })
                    .collect::<Vec<_>>();
                if let AttrsVerdict::Remove = process_attrs(&mut expanded, changed)? {
                    verdict = AttrsVerdict::Remove;
                }
                output.extend(expanded);
            }
            Partial::Residual(residual) => {
                let mut kept = attribute.clone();
                kept.meta = syn::parse_quote!(cfg_attr(#residual, #(#rest),*));
                output.push(kept);
            }
        }
    }
    *attrs = output;
    Ok(verdict)
}

/// Evaluates Verus driver cfgs on the attribute owners rustc's cfg-stripping
/// visits and this pass supports. Anything left over is caught by
/// `ResidueAudit`.
#[derive(Default)]
struct DriverCfgPass {
    changed: bool,
    error: Option<String>,
}

impl DriverCfgPass {
    fn attrs(&mut self, attrs: &mut Vec<Attribute>) -> bool {
        if self.error.is_some() {
            return true;
        }
        match process_attrs(attrs, &mut self.changed) {
            Ok(AttrsVerdict::Keep) => true,
            Ok(AttrsVerdict::Remove) => false,
            Err(error) => {
                self.error = Some(format!("--verus-exec: {error}"));
                true
            }
        }
    }

    fn retain<T>(&mut self, nodes: &mut Vec<T>, attrs_of: impl Fn(&mut T) -> Option<&mut Vec<Attribute>>) {
        let mut kept = Vec::with_capacity(nodes.len());
        for mut node in nodes.drain(..) {
            let keep = match attrs_of(&mut node) {
                Some(attrs) => self.attrs(attrs),
                None => true,
            };
            if keep {
                kept.push(node);
            }
        }
        *nodes = kept;
    }

    fn retain_punctuated<T, P: Default>(
        &mut self,
        nodes: &mut Punctuated<T, P>,
        attrs_of: impl Fn(&mut T) -> &mut Vec<Attribute>,
    ) {
        let mut kept = Punctuated::new();
        for mut node in std::mem::take(nodes).into_iter() {
            if self.attrs(attrs_of(&mut node)) {
                kept.push(node);
            }
        }
        *nodes = kept;
    }
}

fn item_attrs(item: &mut Item) -> Option<&mut Vec<Attribute>> {
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

fn impl_item_attrs(item: &mut syn::ImplItem) -> Option<&mut Vec<Attribute>> {
    Some(match item {
        syn::ImplItem::Const(item) => &mut item.attrs,
        syn::ImplItem::Fn(item) => &mut item.attrs,
        syn::ImplItem::Type(item) => &mut item.attrs,
        syn::ImplItem::Macro(item) => &mut item.attrs,
        _ => return None,
    })
}

fn trait_item_attrs(item: &mut syn::TraitItem) -> Option<&mut Vec<Attribute>> {
    Some(match item {
        syn::TraitItem::Const(item) => &mut item.attrs,
        syn::TraitItem::Fn(item) => &mut item.attrs,
        syn::TraitItem::Type(item) => &mut item.attrs,
        syn::TraitItem::Macro(item) => &mut item.attrs,
        _ => return None,
    })
}

fn foreign_item_attrs(item: &mut syn::ForeignItem) -> Option<&mut Vec<Attribute>> {
    Some(match item {
        syn::ForeignItem::Fn(item) => &mut item.attrs,
        syn::ForeignItem::Static(item) => &mut item.attrs,
        syn::ForeignItem::Type(item) => &mut item.attrs,
        syn::ForeignItem::Macro(item) => &mut item.attrs,
        _ => return None,
    })
}

fn stmt_attrs(stmt: &mut syn::Stmt) -> Option<&mut Vec<Attribute>> {
    match stmt {
        syn::Stmt::Local(local) => Some(&mut local.attrs),
        syn::Stmt::Item(item) => item_attrs(item),
        syn::Stmt::Macro(mac) => Some(&mut mac.attrs),
        syn::Stmt::Expr(..) => None,
    }
}

fn fn_arg_attrs(arg: &mut syn::FnArg) -> &mut Vec<Attribute> {
    match arg {
        syn::FnArg::Receiver(receiver) => &mut receiver.attrs,
        syn::FnArg::Typed(typed) => &mut typed.attrs,
    }
}

impl VisitMut for DriverCfgPass {
    fn visit_file_mut(&mut self, file: &mut syn::File) {
        if !self.attrs(&mut file.attrs) {
            self.error.get_or_insert_with(|| {
                "--verus-exec: a file-level `#![cfg(..)]` on a Verus driver cfg removes the whole module; unsupported"
                    .to_string()
            });
        }
        self.retain(&mut file.items, item_attrs);
        syn::visit_mut::visit_file_mut(self, file);
    }

    fn visit_item_mod_mut(&mut self, module: &mut syn::ItemMod) {
        if let Some((_, items)) = &mut module.content {
            self.retain(items, item_attrs);
        }
        syn::visit_mut::visit_item_mod_mut(self, module);
    }

    fn visit_item_impl_mut(&mut self, item: &mut syn::ItemImpl) {
        self.retain(&mut item.items, impl_item_attrs);
        syn::visit_mut::visit_item_impl_mut(self, item);
    }

    fn visit_item_trait_mut(&mut self, item: &mut syn::ItemTrait) {
        self.retain(&mut item.items, trait_item_attrs);
        syn::visit_mut::visit_item_trait_mut(self, item);
    }

    fn visit_item_foreign_mod_mut(&mut self, item: &mut syn::ItemForeignMod) {
        self.retain(&mut item.items, foreign_item_attrs);
        syn::visit_mut::visit_item_foreign_mod_mut(self, item);
    }

    fn visit_item_enum_mut(&mut self, item: &mut syn::ItemEnum) {
        self.retain_punctuated(&mut item.variants, |variant| &mut variant.attrs);
        syn::visit_mut::visit_item_enum_mut(self, item);
    }

    fn visit_fields_named_mut(&mut self, fields: &mut syn::FieldsNamed) {
        self.retain_punctuated(&mut fields.named, |field| &mut field.attrs);
        syn::visit_mut::visit_fields_named_mut(self, fields);
    }

    fn visit_fields_unnamed_mut(&mut self, fields: &mut syn::FieldsUnnamed) {
        self.retain_punctuated(&mut fields.unnamed, |field| &mut field.attrs);
        syn::visit_mut::visit_fields_unnamed_mut(self, fields);
    }

    fn visit_signature_mut(&mut self, signature: &mut syn::Signature) {
        self.retain_punctuated(&mut signature.inputs, fn_arg_attrs);
        syn::visit_mut::visit_signature_mut(self, signature);
    }

    fn visit_block_mut(&mut self, block: &mut syn::Block) {
        self.retain(&mut block.stmts, stmt_attrs);
        syn::visit_mut::visit_block_mut(self, block);
    }

    fn visit_expr_match_mut(&mut self, expr: &mut syn::ExprMatch) {
        self.retain(&mut expr.arms, |arm| Some(&mut arm.attrs));
        syn::visit_mut::visit_expr_match_mut(self, expr);
    }

    fn visit_expr_struct_mut(&mut self, expr: &mut syn::ExprStruct) {
        self.retain_punctuated(&mut expr.fields, |field| &mut field.attrs);
        syn::visit_mut::visit_expr_struct_mut(self, expr);
    }
}

// ---------------------------------------------------------------------------
// fail-closed audit of what survives

/// Macro names exported by `verus_builtin_macros` and
/// `verus_state_machines_macros` (and vstd's `macro_rules!` front ends for
/// them). None of them may survive into the transpiled program.
const VERUS_MACROS: &[&str] = &[
    "verus",
    "verus_keep_ghost",
    "verus_erase_ghost",
    "verus_impl",
    "verus_trait_impl",
    "verus_proof_expr",
    "verus_exec_expr",
    "verus_exec_expr_keep_ghost",
    "verus_exec_expr_erase_ghost",
    "verus_proof_macro_exprs",
    "verus_exec_macro_exprs",
    "verus_exec_inv_macro_exprs",
    "verus_ghost_inv_macro_exprs",
    "verus_proof_macro_explicit_exprs",
    "struct_with_invariants",
    "atomic_with_ghost",
    "atomic_with_ghost_helper",
    "calc",
    "calc_proc_macro",
    "proof",
    "proof_decl",
    "proof_with",
    "fndecl",
    "exec_spec",
    "tokenized_state_machine",
    "state_machine",
    "case_on_next",
    "case_on_next_strong",
    "case_on_init",
    "open_atomic_invariant",
    "open_local_invariant",
    "open_atomic_invariant_in_proof",
    "open_local_invariant_in_proof",
];

/// Crates whose macros are Verus-only, whatever their name.
const VERUS_MACRO_CRATES: &[&str] = &[
    "vstd",
    "verus_builtin",
    "verus_builtin_macros",
    "builtin_macros",
    "verus_state_machines_macros",
    "state_machines_macros",
];

fn verus_macro_name(path: &syn::Path) -> Option<String> {
    let first = path.segments.first()?.ident.to_string();
    let last = path.segments.last()?.ident.to_string();
    if path.segments.len() > 1 && VERUS_MACRO_CRATES.contains(&first.as_str()) {
        return Some(quote::quote!(#path).to_string().replace(' ', ""));
    }
    (path.segments.len() == 1 && VERUS_MACROS.contains(&last.as_str())).then_some(last)
}

/// The first `<verus macro>!` invocation inside a token stream, if any.
fn tokens_invoke_verus_macro(tokens: TokenStream) -> Option<String> {
    let trees = tokens.into_iter().collect::<Vec<_>>();
    for (index, tree) in trees.iter().enumerate() {
        match tree {
            TokenTree::Ident(ident)
                if VERUS_MACROS.iter().any(|name| ident == name)
                    && matches!(trees.get(index + 1), Some(TokenTree::Punct(p)) if p.as_char() == '!') =>
            {
                return Some(ident.to_string());
            }
            TokenTree::Group(group) => {
                if let Some(found) = tokens_invoke_verus_macro(group.stream()) {
                    return Some(found);
                }
            }
            _ => {}
        }
    }
    None
}

#[derive(Default)]
struct ResidueAudit {
    errors: Vec<String>,
}

impl<'ast> Visit<'ast> for ResidueAudit {
    fn visit_macro(&mut self, mac: &'ast syn::Macro) {
        if let Some(name) = verus_macro_name(&mac.path) {
            self.errors.push(format!(
                "--verus-exec: `{name}!` is a Verus macro this pass does not erase (only item-level `verus! {{ }}` is supported)"
            ));
        } else if let Some(inner) = tokens_invoke_verus_macro(mac.tokens.clone()) {
            // e.g. Lion's `macro_rules! reactor_log_action { .. => { verus! {
            // impl Reactor { .. } } } }`: rustc erases what the expansion
            // produces, but this pass only sees the unexpanded tokens.
            let path = &mac.path;
            self.errors.push(format!(
                "--verus-exec: `{}!` carries a `{inner}!` invocation in its tokens; macro-generated Verus code is not erased by this pass",
                quote::quote!(#path).to_string().replace(' ', "")
            ));
        } else if meta_mentions_driver_cfg(mac.tokens.clone()) {
            let path = &mac.path;
            self.errors.push(format!(
                "--verus-exec: `{}!` mentions a Verus driver cfg inside its tokens, which this pass cannot evaluate",
                quote::quote!(#path).to_string().replace(' ', "")
            ));
        }
        syn::visit::visit_macro(self, mac);
    }

    fn visit_attribute(&mut self, attribute: &'ast Attribute) {
        if let Meta::List(list) = &attribute.meta
            && (list.path.is_ident("cfg") || list.path.is_ident("cfg_attr"))
            && meta_mentions_driver_cfg(list.tokens.clone())
        {
            self.errors.push(format!(
                "--verus-exec: `{}` survives in a position this pass does not evaluate",
                quote::quote!(#attribute)
            ));
        }
        syn::visit::visit_attribute(self, attribute);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn erase(source: &str) -> String {
        erase_source(source).expect("erasure succeeds").into_owned()
    }

    fn pretty(source: &str) -> String {
        prettyplease::unparse(&syn::parse_file(source).expect("expected source parses"))
    }

    #[test]
    fn source_without_verus_constructs_is_returned_byte_for_byte() {
        let source = "// a comment the pre-pass must not touch\nfn  main( ) { }\n";
        assert!(matches!(erase_source(source).unwrap(), Cow::Borrowed(_)));
        let mentions = "// verus is only mentioned here\nfn  main( ) { }\n";
        assert!(matches!(erase_source(mentions).unwrap(), Cow::Borrowed(_)));
        let lookalike = "mod proof_utils { pub fn calc() {} }\n// vstd\n";
        assert!(matches!(erase_source(lookalike).unwrap(), Cow::Borrowed(_)));
        assert_eq!(
            prepare_source(&VerusExecConfig::default(), "verus! { spec fn f() {} }".to_string())
                .unwrap(),
            "verus! { spec fn f() {} }",
            "disabled mode is the identity"
        );
    }

    #[test]
    fn verus_block_erases_to_executable_code_only() {
        let erased = erase(
            r#"
use vstd::prelude::*;
verus! {
pub struct Counter { pub value: u64 }

spec fn limit() -> nat { 100 }

proof fn limit_is_positive() ensures limit() > 0 {}

impl Counter {
    pub open spec fn wf(&self) -> bool { self.value <= limit() }

    pub fn bump(&mut self) -> (r: u64)
        requires old(self).wf(), old(self).value < 100,
        ensures self.wf(), r == self.value,
    {
        let ghost before = self.value;
        proof { assert(before < 100); }
        self.value = self.value + 1;
        assert(self.value == before + 1);
        self.value
    }
}
}
"#,
        );
        assert_eq!(
            erased,
            pretty(
                r#"
use vstd::prelude::*;
pub struct Counter { pub value: u64 }
impl Counter {
    pub fn bump(&mut self) -> u64 {
        {}
        self.value = self.value + 1;
        {};
        self.value
    }
}
"#
            )
        );
    }

    #[test]
    fn verus_blocks_inside_inline_modules_are_erased() {
        let erased = erase(
            r#"
pub mod outer {
    pub mod inner {
        verus! {
            pub fn f(x: u8) -> u8 ensures x == x { x }
            spec fn g() -> int { 0 }
        }
    }
    vstd::prelude::verus! { pub const K: u8 = 1; }
}
"#,
        );
        assert_eq!(
            erased,
            pretty("pub mod outer { pub mod inner { pub fn f(x: u8) -> u8 { x } } pub const K: u8 = 1; }")
        );
    }

    #[test]
    fn verus_keep_ghost_cfgs_are_false() {
        let erased = erase(
            r#"
#![cfg_attr(verus_keep_ghost, verus::trusted)]
#![allow(dead_code)]
#[cfg(verus_keep_ghost)]
use vstd::invariant::*;
#[cfg(not(verus_keep_ghost))]
pub fn plain() {}
#[cfg(all(feature = "x", not(verus_keep_ghost_body)))]
pub fn featured() {}
#[cfg(any(verus_keep_ghost, test))]
pub fn test_only() {}
#[cfg_attr(not(verus_keep_ghost), inline)]
pub fn inlined() {}
pub struct S {
    #[cfg(verus_keep_ghost)]
    ghost: u8,
    kept: u8,
}
"#,
        );
        assert_eq!(
            erased,
            pretty(
                r#"
#![allow(dead_code)]
pub fn plain() {}
#[cfg(feature = "x")]
pub fn featured() {}
#[cfg(test)]
pub fn test_only() {}
#[inline]
pub fn inlined() {}
pub struct S {
    kept: u8,
}
"#
            )
        );
    }

    #[test]
    fn verus_keep_ghost_items_inside_verus_blocks_are_removed() {
        let erased = erase(
            "verus! {\n#[cfg(verus_keep_ghost)]\npub fn ghost_only() {}\npub fn kept() {}\n}\n",
        );
        assert_eq!(erased, pretty("pub fn kept() {}"));
    }

    #[test]
    fn other_verus_macros_fail_closed() {
        for source in [
            "verus_impl! { fn f() {} }",
            "verus_keep_ghost! { fn f() {} }",
            "fn f() { proof! { assert(true); } }",
            "fn f() { verus! { fn g() {} } }",
            "struct_with_invariants! { struct S {} }",
            "vstd::atomic_with_ghost!(x => load(); ghost g => {});",
        ] {
            let error = erase_source(source).expect_err(source);
            assert!(error.contains("Verus macro"), "{source}: {error}");
        }
    }

    #[test]
    fn macro_generated_verus_blocks_fail_closed() {
        // Lion's reactor/executor `*_log_action` shape.
        let error = erase_source(
            "macro_rules! log_action { ($name:ident) => { verus! { impl R { fn $name(&self) {} } } }; }",
        )
        .unwrap_err();
        assert!(error.contains("macro_rules") && error.contains("verus!"), "{error}");
    }

    #[test]
    fn driver_cfg_inside_macro_tokens_fails_closed() {
        let error = erase_source(
            "thread_local! { #[cfg(verus_keep_ghost)] static X: u8 = 0; }",
        )
        .unwrap_err();
        assert!(error.contains("thread_local"), "{error}");
    }

    #[test]
    fn verus_syntax_errors_fail_closed() {
        let error = erase_source("verus! { fn f() -> {} }").unwrap_err();
        assert!(error.contains("Verus rejected"), "{error}");
    }

    #[test]
    fn dump_writes_the_prepared_unit_under_the_crate_name() {
        let dir = tempfile::tempdir().unwrap();
        let config = VerusExecConfig {
            enabled: true,
            dump_dir: Some(dir.path().to_path_buf()),
        };
        let prepared = prepare_source(&config, "verus! { fn f() ensures true {} }".to_string())
            .unwrap();
        dump_source(&config, "demo", Path::new("src/lib.rs"), &prepared).unwrap();
        let dumped = std::fs::read_to_string(dir.path().join("demo/src/lib.rs")).unwrap();
        assert_eq!(dumped, pretty("fn f() {}"));
    }
}
