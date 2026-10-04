//! Differential erasure: `--verus-exec` stage 1 against what rustc compiles.
//!
//! This is the evidence that the vendored `EraseAll` pass (the
//! `rusty-cpp-verus-erase` helper) and its splicing into the source are the
//! erasure plain rustc runs. The input is a snapshot of every Lion crate
//! SRPC compiles (`tests/fixtures/lion/tree`, Lion at the revision in
//! `MANIFEST.toml`) and, for each, `cargo expand --lib` output produced by
//! `tests/fixtures/lion/regen.sh` with the `verus_builtin_macros` that a
//! Cargo.lock identical to SRPC's resolves (Verus git `db81a74`). The test
//! never runs `cargo expand`; it runs the transpiler's stage 1
//! ([`super::erase_source`], the function `prepare_source` calls) on every
//! source file of the snapshot, i.e. the real "post-T1" output, not the
//! `--dump-verus-erasure` dump (which is taken after stage 2).
//!
//! # How the two sides are compared
//!
//! Both sides are parsed with `syn` and every item is printed alone with
//! `prettyplease`. One file of a crate is one module of the expansion
//! (`mod a;` and `mod b { .. }` nest the same way on both sides). Within a
//! module, every item stage 1 produced must be paired with an item of the
//! expansion, and every item of the expansion must be paired with one of
//! stage 1's or be one of rustc's own injections (the 2021 prelude import,
//! `extern crate std`, `#![feature(prelude_import)]`). Unpaired items on
//! either side are mismatches. So are differing inner attributes.
//!
//! Each item of a `verus!` block's output falls in exactly one class:
//!
//! - **exact**: no macro invocation anywhere in the item. Its print must
//!   equal the expansion's item's print byte for byte.
//! - **exact modulo derive**: the only macro is `#[derive(..)]`. rustc
//!   replaces the attribute with `#[automatically_derived]` impls that it
//!   generates from the item *after* erasure, so the derive is not part of
//!   what Verus erased. The item is compared with the attribute removed, and
//!   the expansion must contain an `#[automatically_derived]` impl of every
//!   derived trait for that type (plus the marker impls rustc adds:
//!   `StructuralPartialEq` for `PartialEq`, `TrivialClone` for `Clone`), and
//!   no other derived impl for it.
//! - **partial**: an `impl` or `trait` some of whose members invoke a macro
//!   (`matches!`, `unreachable!`, `write!`, ...) that rustc expands. The
//!   header (attributes, generics, trait, self type) and every member
//!   without a macro are compared exactly; the members with one must exist
//!   in the expansion under the same kind and name, with the same signature
//!   and attributes for a function (only its body differs by the expansion).
//! - **presence**: any other item with a macro invocation (a `fn` whose body
//!   expands one is compared on its signature and attributes; the rest on
//!   their key). Never compared byte for byte.
//!
//! Items a cfg removes are checked rather than skipped: `verus_keep_ghost`
//! and `verus_keep_ghost_body` are false for plain rustc, so the items Verus's
//! output gates on them must be absent from both stage 1's output and the
//! expansion. Stage 1 leaves every other cfg (`feature = ..`) to later passes;
//! here those are evaluated against the features Cargo resolved for the
//! crate in the snapshot's resolution (recorded in `MANIFEST.toml`, none
//! today), with `test` false, which is what `cargo expand --lib` compiled.
//! An item one of them removes must be absent from the expansion too. Any
//! other cfg predicate fails the test until it is given a value here.
//!
//! The measured counts per crate are pinned ([`EXPECTED`]), so a coverage
//! drop fails as loudly as a mismatch. `cargo test differential -- --nocapture`
//! prints the table.
//!
//! `RUSTY_CPP_LION_FIXTURE=<dir>` points the test at another snapshot of the
//! same layout (`regen.sh --out <dir>` writes one), for checking a new Lion
//! revision before committing it; the pinned counts then apply only if they
//! still match.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use quote::ToTokens;
use syn::visit::Visit;
use syn::visit_mut::VisitMut;
use syn::{Attribute, ImplItem, Item, Meta, TraitItem};

use super::{AttrsVerdict, VERUS_GIT_REV, VerusExecConfig, erase_source, is_verus_items_macro, process_attrs, run_helper, test_helper};

/// The measured outcome per crate, pinned: `(crate, blocks, verus items,
/// exact, exact modulo derive, partial, presence, removed by a Verus driver
/// cfg, removed by a feature cfg)`.
#[rustfmt::skip]
const EXPECTED: &[(&str, usize, usize, usize, usize, usize, usize, usize, usize)] = &[
    // crate               blocks items exact derive partial presence ghost-cfg feature-cfg
    ("lion-framework-spec",    4,    4,    4,     0,      0,       0,        0,          0),
    ("lion-utility-spec",      8,   15,   13,     2,      0,       0,        0,          0),
    ("lion-reactor-spec",     22,    4,    1,     3,      0,       0,       22,          0),
    ("lion-executor-spec",    19,   10,    8,     1,      1,       0,        0,          0),
    ("lion-slab",              1,    5,    5,     0,      0,       0,        0,          0),
    ("lion-timer-wheel",       3,   15,   14,     1,      0,       0,        0,          0),
    ("lion-reactor",          33,   64,   53,     7,      4,       0,        1,          0),
    ("lion-executor",         26,   44,   38,     5,      1,       0,        0,          0),
];

fn fixture_root() -> PathBuf {
    std::env::var_os("RUSTY_CPP_LION_FIXTURE")
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/lion"))
}

fn config() -> VerusExecConfig {
    VerusExecConfig {
        enabled: true,
        dump_dir: None,
        helper: Some(test_helper::path()),
    }
}

// ---------------------------------------------------------------------------
// printing, keys, macro classification

/// One item printed alone, after [`ClosureBodies`].
fn render(item: &Item) -> String {
    let mut item = item.clone();
    ClosureBodies.visit_item_mut(&mut item);
    render_raw(&item)
}

fn render_raw(item: &Item) -> String {
    prettyplease::unparse(&syn::File {
        shebang: None,
        attrs: Vec::new(),
        items: vec![item.clone()],
    })
}

/// rustc's `-Zunpretty=expanded` printer chooses the braces of a closure
/// body itself: `|q| { e }` in the source prints as `|q| e`, and a long
/// `|c| a.b().c()` prints as `|c| { a.b().c() }`. The two spellings are the
/// same closure. Both sides are therefore printed with a closure body that is
/// a block of exactly one tail expression (no statements, attributes, label
/// or `-> T`) unwrapped to that expression. Nothing else is normalized.
struct ClosureBodies;

impl VisitMut for ClosureBodies {
    fn visit_expr_closure_mut(&mut self, closure: &mut syn::ExprClosure) {
        syn::visit_mut::visit_expr_closure_mut(self, closure);
        if matches!(closure.output, syn::ReturnType::Default)
            && let syn::Expr::Block(block) = &*closure.body
            && block.attrs.is_empty()
            && block.label.is_none()
            && let [syn::Stmt::Expr(tail, None)] = block.block.stmts.as_slice()
        {
            closure.body = Box::new(tail.clone());
        }
    }
}

fn tokens(node: &impl ToTokens) -> String {
    node.to_token_stream().to_string()
}

/// [`tokens`] without trailing commas, which `prettyplease` adds or drops
/// with line breaks; for keys only (prints are compared as printed).
fn key_tokens(node: &impl ToTokens) -> String {
    let mut text = tokens(node);
    for close in ["}", ">", ")", "]"] {
        text = text.replace(&format!(" , {close}"), &format!(" {close}"));
    }
    text
}

/// What pairs an item with its counterpart: kind and name, and for an impl
/// its generics, trait and self type.
fn key(item: &Item) -> String {
    match item {
        Item::Const(item) => format!("const {}", item.ident),
        Item::Enum(item) => format!("enum {}", item.ident),
        Item::ExternCrate(item) => format!("extern crate {}", item.ident),
        Item::Fn(item) => format!("fn {}", item.sig.ident),
        Item::ForeignMod(item) => format!("extern {}", key_tokens(&item.abi)),
        Item::Impl(item) => format!(
            "impl{} {}for {}",
            key_tokens(&item.generics),
            item.trait_
                .as_ref()
                .map(|(bang, path, _)| format!("{}{} ", if bang.is_some() { "!" } else { "" }, key_tokens(path)))
                .unwrap_or_default(),
            key_tokens(&item.self_ty)
        ),
        Item::Macro(item) => match &item.ident {
            Some(ident) => format!("macro_rules {ident}"),
            None => format!("{}!", key_tokens(&item.mac.path)),
        },
        Item::Mod(item) => format!("mod {}", item.ident),
        Item::Static(item) => format!("static {}", item.ident),
        Item::Struct(item) => format!("struct {}", item.ident),
        Item::Trait(item) => format!("trait {}", item.ident),
        Item::TraitAlias(item) => format!("trait alias {}", item.ident),
        Item::Type(item) => format!("type {}", item.ident),
        Item::Union(item) => format!("union {}", item.ident),
        Item::Use(item) => format!("use {}", key_tokens(&item.tree)),
        other => format!("other {}", key_tokens(other)),
    }
}

/// Macro invocations in a node (`name!`), and `#derive` for a derive.
#[derive(Default)]
struct Macros(BTreeSet<String>);

impl<'ast> Visit<'ast> for Macros {
    fn visit_macro(&mut self, mac: &'ast syn::Macro) {
        self.0.insert(format!("{}!", tokens(&mac.path).replace(' ', "")));
        syn::visit::visit_macro(self, mac);
    }

    fn visit_attribute(&mut self, attribute: &'ast Attribute) {
        if attribute.path().is_ident("derive") {
            self.0.insert("#derive".to_string());
        }
    }
}

fn macros_in(visit: impl FnOnce(&mut Macros)) -> BTreeSet<String> {
    let mut macros = Macros::default();
    visit(&mut macros);
    macros.0
}

fn item_macros(item: &Item) -> BTreeSet<String> {
    macros_in(|m| m.visit_item(item))
}

fn attrs_of(item: &Item) -> &[Attribute] {
    match item {
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
        _ => &[],
    }
}

fn attrs_of_mut(item: &mut Item) -> Option<&mut Vec<Attribute>> {
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

fn has_attr(item: &Item, name: &str) -> bool {
    attrs_of(item).iter().any(|attribute| attribute.path().is_ident(name))
}

/// The derived trait names of an item (last path segment of every
/// `#[derive(..)]` entry).
fn derived_traits(item: &Item) -> Vec<String> {
    let mut traits = Vec::new();
    for attribute in attrs_of(item).iter().filter(|a| a.path().is_ident("derive")) {
        let paths = attribute
            .parse_args_with(syn::punctuated::Punctuated::<syn::Path, syn::Token![,]>::parse_terminated)
            .expect("derive arguments are paths");
        for path in paths {
            traits.push(path.segments.last().unwrap().ident.to_string());
        }
    }
    traits
}

fn strip_derives(item: &Item) -> Item {
    let mut item = item.clone();
    if let Some(attrs) = attrs_of_mut(&mut item) {
        attrs.retain(|attribute| !attribute.path().is_ident("derive"));
    }
    item
}

fn type_name(ty: &syn::Type) -> Option<String> {
    match ty {
        syn::Type::Path(path) => path.path.segments.last().map(|segment| segment.ident.to_string()),
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// like-with-like cfg evaluation of what stage 1 leaves (feature, test)

struct CfgContext {
    features: BTreeSet<String>,
}

impl CfgContext {
    fn eval(&self, meta: &Meta) -> bool {
        match meta {
            Meta::Path(path) if path.is_ident("verus_keep_ghost") || path.is_ident("verus_keep_ghost_body") => false,
            Meta::Path(path) if path.is_ident("test") => false,
            Meta::NameValue(name_value) if name_value.path.is_ident("feature") => {
                let syn::Expr::Lit(syn::ExprLit { lit: syn::Lit::Str(value), .. }) = &name_value.value else {
                    panic!("malformed feature cfg `{}`", tokens(meta));
                };
                self.features.contains(&value.value())
            }
            Meta::List(list) if list.path.is_ident("all") || list.path.is_ident("any") || list.path.is_ident("not") => {
                let args = list
                    .parse_args_with(syn::punctuated::Punctuated::<Meta, syn::Token![,]>::parse_terminated)
                    .expect("cfg predicate arguments parse");
                let values = args.iter().map(|arg| self.eval(arg)).collect::<Vec<_>>();
                if list.path.is_ident("all") {
                    values.iter().all(|v| *v)
                } else if list.path.is_ident("any") {
                    values.iter().any(|v| *v)
                } else {
                    assert_eq!(values.len(), 1, "`not` takes one predicate");
                    !values[0]
                }
            }
            other => panic!(
                "cfg predicate `{}` has no value in this comparison; give it the value `cargo expand --lib` compiled with",
                tokens(other)
            ),
        }
    }

    /// Evaluate `cfg`/`cfg_attr` in one attribute list the way rustc's cfg
    /// stripping does; `false` means the owner is removed.
    fn attrs(&self, attrs: &mut Vec<Attribute>) -> bool {
        let mut keep = true;
        let mut out = Vec::with_capacity(attrs.len());
        for attribute in attrs.drain(..) {
            if attribute.path().is_ident("cfg") {
                let predicate = attribute.parse_args::<Meta>().expect("cfg predicate parses");
                keep &= self.eval(&predicate);
            } else if attribute.path().is_ident("cfg_attr") {
                let args = attribute
                    .parse_args_with(syn::punctuated::Punctuated::<Meta, syn::Token![,]>::parse_terminated)
                    .expect("cfg_attr parses");
                let mut args = args.into_iter();
                let predicate = args.next().expect("cfg_attr has a predicate");
                if self.eval(&predicate) {
                    let mut expanded = args
                        .map(|meta| Attribute { meta, ..attribute.clone() })
                        .collect::<Vec<_>>();
                    keep &= self.attrs(&mut expanded);
                    out.extend(expanded);
                }
            } else {
                out.push(attribute);
            }
        }
        *attrs = out;
        keep
    }
}

/// Applies [`CfgContext`] everywhere rustc's cfg stripping looks, below the
/// item level (items themselves are filtered by the caller, which counts them).
struct CfgStrip<'a> {
    cfg: &'a CfgContext,
}

impl CfgStrip<'_> {
    fn retain<T>(&self, nodes: &mut Vec<T>, attrs: impl Fn(&mut T) -> Option<&mut Vec<Attribute>>) {
        nodes.retain_mut(|node| attrs(node).is_none_or(|attrs| self.cfg.attrs(attrs)));
    }

    fn retain_punctuated<T, P: Default>(
        &self,
        nodes: &mut syn::punctuated::Punctuated<T, P>,
        attrs: impl Fn(&mut T) -> &mut Vec<Attribute>,
    ) {
        let kept = std::mem::take(nodes)
            .into_iter()
            .filter_map(|mut node| self.cfg.attrs(attrs(&mut node)).then_some(node))
            .collect();
        *nodes = kept;
    }
}

impl VisitMut for CfgStrip<'_> {
    fn visit_item_mut(&mut self, item: &mut Item) {
        if let Some(attrs) = attrs_of_mut(item) {
            assert!(self.cfg.attrs(attrs), "item-level cfgs are filtered by the caller");
        }
        if let Item::Mod(module) = item
            && let Some((_, items)) = &mut module.content
        {
            items.retain_mut(|item| attrs_of_mut(item).is_none_or(|attrs| self.cfg.attrs(attrs)));
        }
        syn::visit_mut::visit_item_mut(self, item);
    }

    fn visit_item_impl_mut(&mut self, item: &mut syn::ItemImpl) {
        self.retain(&mut item.items, |item| match item {
            ImplItem::Const(item) => Some(&mut item.attrs),
            ImplItem::Fn(item) => Some(&mut item.attrs),
            ImplItem::Type(item) => Some(&mut item.attrs),
            ImplItem::Macro(item) => Some(&mut item.attrs),
            _ => None,
        });
        syn::visit_mut::visit_item_impl_mut(self, item);
    }

    fn visit_item_trait_mut(&mut self, item: &mut syn::ItemTrait) {
        self.retain(&mut item.items, |item| match item {
            TraitItem::Const(item) => Some(&mut item.attrs),
            TraitItem::Fn(item) => Some(&mut item.attrs),
            TraitItem::Type(item) => Some(&mut item.attrs),
            TraitItem::Macro(item) => Some(&mut item.attrs),
            _ => None,
        });
        syn::visit_mut::visit_item_trait_mut(self, item);
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
        self.retain_punctuated(&mut signature.inputs, |arg| match arg {
            syn::FnArg::Receiver(receiver) => &mut receiver.attrs,
            syn::FnArg::Typed(typed) => &mut typed.attrs,
        });
        syn::visit_mut::visit_signature_mut(self, signature);
    }

    fn visit_block_mut(&mut self, block: &mut syn::Block) {
        self.retain(&mut block.stmts, |stmt| match stmt {
            syn::Stmt::Local(local) => Some(&mut local.attrs),
            syn::Stmt::Item(item) => attrs_of_mut(item),
            syn::Stmt::Macro(mac) => Some(&mut mac.attrs),
            syn::Stmt::Expr(..) => None,
        });
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
// stage 1 output, with each item attributed to a verus! block or not

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Origin {
    Outside,
    Block,
}

struct Tagged {
    item: Item,
    origin: Origin,
    /// An inline `mod`'s items, attributed the same way.
    children: Vec<Tagged>,
}

#[derive(Default, Debug, Clone, PartialEq, Eq)]
struct Counts {
    blocks: usize,
    items: usize,
    exact: usize,
    derive: usize,
    partial: usize,
    presence: usize,
    ghost_cfg_removed: usize,
    feature_cfg_removed: usize,
    /// Members of `partial` items compared exactly / only by presence.
    partial_members_exact: usize,
    partial_members_presence: usize,
    /// Items outside `verus!` blocks: compared the same way, but not part of
    /// the coverage figure.
    outside_items: usize,
    /// The macros that kept items out of the exact classes.
    presence_macros: BTreeSet<String>,
}

/// One item of the expansion and whether a stage 1 item claimed it.
struct Expanded<'e> {
    item: &'e Item,
    taken: bool,
}

struct Comparison<'a> {
    config: VerusExecConfig,
    cfg: CfgContext,
    crate_name: &'a str,
    counts: Counts,
    mismatches: Vec<String>,
}

fn module_label(crate_name: &str, module: &[String]) -> String {
    let mut label = crate_name.replace('-', "_");
    for segment in module {
        label.push_str("::");
        label.push_str(segment);
    }
    label
}

/// rustc's own additions to an expanded crate.
fn is_injection(item: &Item) -> bool {
    match item {
        Item::ExternCrate(item) => item.ident == "std" && item.rename.is_none(),
        Item::Use(_) => has_attr(item, "prelude_import"),
        _ => false,
    }
}

impl Comparison<'_> {
    fn mismatch(&mut self, module: &[String], what: String) {
        let label = module_label(self.crate_name, module);
        self.mismatches.push(format!("{label}: {what}"));
    }

    /// Stage 1 of one `verus!` block alone (the transpiler's own function on
    /// a one-block source), and the helper's raw erasure of it, before
    /// stage 1 evaluates the driver cfgs.
    fn erase_block(&self, mac: &syn::Macro) -> (Vec<Item>, Vec<Item>) {
        let text = mac.tokens.to_string();
        let stage1 = erase_source(&self.config, &format!("verus! {{ {text} }}"))
            .unwrap_or_else(|error| panic!("{}: stage 1 rejects a block: {error}", self.crate_name))
            .into_owned();
        let stage1 = syn::parse_file(&stage1).expect("stage 1 output parses").items;
        let raw = run_helper(&self.config, &[text])
            .expect("the helper runs")
            .remove(0)
            .expect("the helper erases the block");
        let raw = syn::parse_file(&raw).expect("helper output parses").items;
        (stage1, raw)
    }

    /// Pair the original items of one item list with stage 1's output for
    /// it, so each output item knows whether a `verus!` block produced it.
    /// Items of a block that a driver cfg removes are returned separately.
    fn attribute(
        &mut self,
        module: &[String],
        original: &[Item],
        prepared: Vec<Item>,
        ghost_removed: &mut Vec<Item>,
    ) -> Vec<Tagged> {
        let mut prepared = prepared.into_iter();
        let mut out = Vec::new();
        for item in original {
            if let Item::Macro(mac) = item
                && mac.ident.is_none()
                && is_verus_items_macro(&mac.mac.path)
            {
                self.counts.blocks += 1;
                let (block, raw) = self.erase_block(&mac.mac);
                for raw_item in raw {
                    let mut attrs = attrs_of(&raw_item).to_vec();
                    if let Ok(AttrsVerdict::Remove) = process_attrs(&mut attrs, &mut false) {
                        self.counts.ghost_cfg_removed += 1;
                        ghost_removed.push(raw_item);
                    }
                }
                for expected in block {
                    let Some(actual) = prepared.next() else {
                        self.mismatch(module, format!("stage 1 lost `{}` of a verus! block", key(&expected)));
                        continue;
                    };
                    if render(&actual) != render(&expected) {
                        self.mismatch(
                            module,
                            format!(
                                "a verus! block erases differently in its file than alone:\n{}\nvs\n{}",
                                render(&actual),
                                render(&expected)
                            ),
                        );
                    }
                    out.push(Tagged {
                        children: block_children(&actual),
                        item: actual,
                        origin: Origin::Block,
                    });
                }
                continue;
            }
            let mut attrs = attrs_of(item).to_vec();
            if let Ok(AttrsVerdict::Remove) = process_attrs(&mut attrs, &mut false) {
                // A driver cfg outside any verus! block: stage 1 removes the
                // item, and so does rustc (checked as an expansion leftover).
                continue;
            }
            let Some(actual) = prepared.next() else {
                self.mismatch(module, format!("stage 1 lost `{}`", key(item)));
                continue;
            };
            assert_eq!(key(&actual), key(item), "stage 1 output is out of step with its input");
            let children = match (item, &actual) {
                (Item::Mod(original), Item::Mod(prepared)) if original.content.is_some() => {
                    let mut inner = module.to_vec();
                    inner.push(original.ident.to_string());
                    let mut nested_removed = Vec::new();
                    let children = self.attribute(
                        &inner,
                        &original.content.as_ref().unwrap().1,
                        prepared.content.as_ref().map(|(_, items)| items.clone()).unwrap_or_default(),
                        &mut nested_removed,
                    );
                    assert!(nested_removed.is_empty(), "driver-cfg items in a nested inline module are not handled");
                    children
                }
                _ => Vec::new(),
            };
            out.push(Tagged {
                item: actual,
                origin: Origin::Outside,
                children,
            });
        }
        for extra in prepared {
            self.mismatch(module, format!("stage 1 produced `{}`, which no input item accounts for", key(&extra)));
        }
        out
    }

    /// Compare one source file (one module) of the crate with its module in
    /// the expansion.
    fn compare_file(&mut self, file: &Path, module: &[String], expanded: &[Item], expanded_attrs: &[Attribute]) {
        let text = std::fs::read_to_string(file).unwrap_or_else(|error| panic!("{}: {error}", file.display()));
        let original = syn::parse_file(&text).unwrap_or_else(|error| panic!("{}: {error}", file.display()));
        let prepared = erase_source(&self.config, &text)
            .unwrap_or_else(|error| panic!("{}: stage 1 failed: {error}", file.display()))
            .into_owned();
        let prepared = syn::parse_file(&prepared).expect("stage 1 output parses");
        let mut ghost_removed = Vec::new();
        let tagged = self.attribute(module, &original.items, prepared.items, &mut ghost_removed);

        // Inner attributes: stage 1's (cfg-evaluated) against the module's in
        // the expansion, without rustc's `#![feature(prelude_import)]`.
        let mut attrs = prepared.attrs.clone();
        assert!(self.cfg.attrs(&mut attrs), "a file-level cfg removes {}", file.display());
        let expected_attrs = expanded_attrs
            .iter()
            .filter(|attribute| tokens(*attribute).replace(' ', "") != "#![feature(prelude_import)]")
            .map(tokens)
            .collect::<Vec<_>>();
        let actual_attrs = attrs.iter().map(tokens).collect::<Vec<_>>();
        if actual_attrs != expected_attrs {
            self.mismatch(module, format!("inner attributes differ: {actual_attrs:?} vs {expected_attrs:?}"));
        }

        let child_dir = if matches!(file.file_name().and_then(|n| n.to_str()), Some("lib.rs" | "mod.rs" | "main.rs")) {
            file.parent().unwrap().to_path_buf()
        } else {
            file.with_extension("")
        };
        self.compare_items(module, tagged, expanded, &child_dir);

        // What Verus's output gates on a driver cfg is not in the expansion.
        let expanded_prints = expanded.iter().map(render).collect::<BTreeSet<_>>();
        for item in ghost_removed {
            let mut item = item;
            if let Some(attrs) = attrs_of_mut(&mut item) {
                attrs.retain(|attribute| !attribute.path().is_ident("cfg"));
            }
            if expanded_prints.contains(&render(&item)) {
                self.mismatch(module, format!("`{}` is cfg(verus_keep_ghost)-only but rustc compiled it", key(&item)));
            }
        }
    }

    fn compare_items(&mut self, module: &[String], tagged: Vec<Tagged>, expanded: &[Item], child_dir: &Path) {
        let mut pool = expanded
            .iter()
            .filter(|item| !is_injection(item))
            .map(|item| Expanded { item, taken: false })
            .collect::<Vec<_>>();
        let mut derived: Vec<(String, Vec<String>)> = Vec::new();

        // Like-with-like cfg evaluation, then the exact classes first, so the
        // looser matches below cannot claim an item an exact match needs.
        let mut kept = Vec::new();
        for mut entry in tagged {
            let keep = attrs_of_mut(&mut entry.item).is_none_or(|attrs| self.cfg.attrs(attrs));
            if !keep {
                if entry.origin == Origin::Block {
                    self.counts.feature_cfg_removed += 1;
                }
                continue;
            }
            CfgStrip { cfg: &self.cfg }.visit_item_mut(&mut entry.item);
            match entry.origin {
                Origin::Block => self.counts.items += 1,
                Origin::Outside => self.counts.outside_items += 1,
            }
            kept.push(entry);
        }

        let mut loose = Vec::new();
        for entry in kept {
            let macros = item_macros(&entry.item);
            let is_mod = matches!(entry.item, Item::Mod(_));
            if is_mod || !(macros.is_empty() || macros.iter().all(|m| m == "#derive")) {
                loose.push((entry, macros));
                continue;
            }
            let derive = !macros.is_empty();
            let candidate = if derive { strip_derives(&entry.item) } else { entry.item.clone() };
            let wanted_key = key(&candidate);
            let wanted = render(&candidate);
            let found = pool
                .iter_mut()
                .find(|e| !e.taken && !has_attr(e.item, "automatically_derived") && key(e.item) == wanted_key && render(e.item) == wanted);
            match found {
                Some(e) => {
                    e.taken = true;
                    if entry.origin == Origin::Block {
                        if derive {
                            self.counts.derive += 1;
                        } else {
                            self.counts.exact += 1;
                        }
                    }
                    if derive {
                        let name = match &entry.item {
                            Item::Struct(item) => item.ident.to_string(),
                            Item::Enum(item) => item.ident.to_string(),
                            Item::Union(item) => item.ident.to_string(),
                            other => panic!("derive on `{}`", key(other)),
                        };
                        derived.push((name, derived_traits(&entry.item)));
                    }
                }
                None => {
                    let same_key = pool
                        .iter()
                        .filter(|e| !e.taken && key(e.item) == wanted_key)
                        .map(|e| first_difference(&wanted, &render(e.item)))
                        .collect::<Vec<_>>();
                    self.mismatch(
                        module,
                        format!(
                            "{:?} item `{wanted_key}` differs from the expansion ({} candidate(s) with that key)\n{}",
                            entry.origin,
                            same_key.len(),
                            same_key.join("\n")
                        ),
                    );
                }
            }
        }

        for (entry, macros) in loose {
            self.compare_loose(module, entry, macros, &mut pool, child_dir);
        }

        // Derived impls: exactly the derives stage 1 kept, for their types.
        for (name, traits) in derived {
            let mut allowed = traits.iter().cloned().collect::<BTreeSet<_>>();
            if allowed.contains("PartialEq") {
                allowed.insert("StructuralPartialEq".to_string());
            }
            if allowed.contains("Clone") {
                allowed.insert("TrivialClone".to_string());
            }
            let mut seen = BTreeSet::new();
            for e in pool.iter_mut().filter(|e| !e.taken && has_attr(e.item, "automatically_derived")) {
                let Item::Impl(item) = e.item else { continue };
                let Some((_, path, _)) = &item.trait_ else { continue };
                let trait_name = path.segments.last().unwrap().ident.to_string();
                if type_name(&item.self_ty).as_deref() == Some(name.as_str()) && allowed.contains(&trait_name) {
                    e.taken = true;
                    seen.insert(trait_name);
                }
            }
            for wanted in &traits {
                if !seen.contains(wanted) {
                    self.mismatch(module, format!("`{name}` derives `{wanted}` but the expansion has no such impl"));
                }
            }
        }

        let leftovers = pool
            .iter()
            .filter(|e| !e.taken)
            .map(|e| format!("`{}`", key(e.item)))
            .collect::<Vec<_>>();
        for leftover in leftovers {
            self.mismatch(module, format!("the expansion has an item stage 1 does not: {leftover}"));
        }
    }

    /// Items with a macro other than a derive, and modules.
    fn compare_loose(
        &mut self,
        module: &[String],
        entry: Tagged,
        macros: BTreeSet<String>,
        pool: &mut [Expanded<'_>],
        child_dir: &Path,
    ) {
        let block = entry.origin == Origin::Block;
        let wanted_key = key(&entry.item);
        // `thread_local! { static X: T = ..; }` expands to `const X: LocalKey<T>`.
        if let Item::Macro(mac) = &entry.item
            && mac.ident.is_none()
            && mac.mac.path.is_ident("thread_local")
        {
            let names = thread_local_names(&mac.mac);
            for name in names {
                let wanted = format!("const {name}");
                match pool.iter_mut().find(|e| !e.taken && key(e.item) == wanted) {
                    Some(e) => e.taken = true,
                    None => self.mismatch(module, format!("thread_local! `{name}` is not in the expansion")),
                }
            }
            if block {
                self.counts.presence += 1;
                self.counts.presence_macros.extend(macros);
            }
            return;
        }
        let Some(position) = pool.iter().position(|e| !e.taken && key(e.item) == wanted_key && same_shape(&entry.item, e.item)) else {
            self.mismatch(module, format!("`{wanted_key}` ({:?}, macros {macros:?}) is not in the expansion", entry.origin));
            return;
        };
        pool[position].taken = true;
        let counterpart = pool[position].item;
        match (&entry.item, counterpart) {
            (Item::Mod(mine), Item::Mod(theirs)) => {
                let mut inner = module.to_vec();
                inner.push(mine.ident.to_string());
                if render(&mod_header(&entry.item)) != render(&mod_header(counterpart)) {
                    self.mismatch(module, format!("`mod {}` has different attributes", mine.ident));
                }
                let Some((_, their_items)) = &theirs.content else {
                    self.mismatch(module, format!("`mod {}` has no body in the expansion", mine.ident));
                    return;
                };
                match &mine.content {
                    Some(_) => {
                        // An inline module: its items, attributed already.
                        let children = if block { block_children(&entry.item) } else { entry.children };
                        let nested_dir = child_dir.join(mine.ident.to_string());
                        self.compare_items(&inner, children, their_items, &nested_dir);
                    }
                    None => {
                        assert!(
                            !mine.attrs.iter().any(|attribute| attribute.path().is_ident("path")),
                            "#[path] modules are not handled"
                        );
                        let name = mine.ident.to_string();
                        let flat = child_dir.join(format!("{name}.rs"));
                        let file = if flat.is_file() { flat } else { child_dir.join(&name).join("mod.rs") };
                        self.compare_file(&file, &inner, their_items, &theirs.attrs);
                    }
                }
                if block {
                    self.counts.partial += 1;
                }
            }
            (Item::Impl(mine), Item::Impl(theirs)) => {
                let header_mine = Item::Impl(syn::ItemImpl { items: Vec::new(), ..mine.clone() });
                let header_theirs = Item::Impl(syn::ItemImpl { items: Vec::new(), ..theirs.clone() });
                if render(&header_mine) != render(&header_theirs) {
                    self.mismatch(module, format!("`{wanted_key}` header differs"));
                }
                let mine = mine.items.iter().map(Member::Impl).collect::<Vec<_>>();
                let theirs = theirs.items.iter().map(Member::Impl).collect::<Vec<_>>();
                self.compare_members(module, &wanted_key, block, &mine, &theirs);
                if block {
                    self.counts.partial += 1;
                }
            }
            (Item::Trait(mine), Item::Trait(theirs)) => {
                let header_mine = Item::Trait(syn::ItemTrait { items: Vec::new(), ..mine.clone() });
                let header_theirs = Item::Trait(syn::ItemTrait { items: Vec::new(), ..theirs.clone() });
                if render(&header_mine) != render(&header_theirs) {
                    self.mismatch(module, format!("`{wanted_key}` header differs"));
                }
                let mine = mine.items.iter().map(Member::Trait).collect::<Vec<_>>();
                let theirs = theirs.items.iter().map(Member::Trait).collect::<Vec<_>>();
                self.compare_members(module, &wanted_key, block, &mine, &theirs);
                if block {
                    self.counts.partial += 1;
                }
            }
            (Item::Fn(mine), Item::Fn(theirs)) => {
                let (header_mine, header_theirs) =
                    (fn_header(&mine.attrs, &mine.vis, &mine.sig), fn_header(&theirs.attrs, &theirs.vis, &theirs.sig));
                if header_mine != header_theirs {
                    self.mismatch(module, format!("`{wanted_key}` signature differs:\n{header_mine}\nvs\n{header_theirs}"));
                }
                if block {
                    self.counts.presence += 1;
                    self.counts.presence_macros.extend(macros);
                }
            }
            _ => {
                if block {
                    self.counts.presence += 1;
                    self.counts.presence_macros.extend(macros);
                }
            }
        }
    }

    fn compare_members(&mut self, module: &[String], owner: &str, block: bool, mine: &[Member<'_>], theirs: &[Member<'_>]) {
        let mut taken = vec![false; theirs.len()];
        for member in mine {
            let name = member.name();
            let Some(index) = (0..theirs.len()).find(|&i| !taken[i] && theirs[i].name() == name) else {
                self.mismatch(module, format!("`{owner}`: member `{name}` is not in the expansion"));
                continue;
            };
            taken[index] = true;
            let other = &theirs[index];
            let macros = member.macros();
            if macros.is_empty() {
                if member.render() != other.render() {
                    self.mismatch(
                        module,
                        format!("`{owner}`: member `{name}` differs\n-- stage 1:\n{}\n-- expansion:\n{}", member.render(), other.render()),
                    );
                }
                if block {
                    self.counts.partial_members_exact += 1;
                }
            } else {
                if member.header() != other.header() {
                    self.mismatch(
                        module,
                        format!("`{owner}`: member `{name}` signature differs:\n{}\nvs\n{}", member.header(), other.header()),
                    );
                }
                if block {
                    self.counts.partial_members_presence += 1;
                    self.counts.presence_macros.extend(macros);
                }
            }
        }
        for (index, other) in theirs.iter().enumerate() {
            if !taken[index] {
                self.mismatch(module, format!("`{owner}`: the expansion has a member stage 1 does not: `{}`", other.name()));
            }
        }
    }
}

/// The items of an inline module produced by a `verus!` block.
fn block_children(item: &Item) -> Vec<Tagged> {
    match item {
        Item::Mod(module) => module
            .content
            .as_ref()
            .map(|(_, items)| {
                items
                    .iter()
                    .map(|item| Tagged {
                        children: block_children(item),
                        item: item.clone(),
                        origin: Origin::Block,
                    })
                    .collect()
            })
            .unwrap_or_default(),
        _ => Vec::new(),
    }
}

/// Same kind of item, and for `mod`, both with bodies on the expansion side.
fn same_shape(mine: &Item, theirs: &Item) -> bool {
    std::mem::discriminant(mine) == std::mem::discriminant(theirs)
}

fn mod_header(item: &Item) -> Item {
    let Item::Mod(module) = item else { unreachable!() };
    let mut module = module.clone();
    module.content = None;
    module.semi = Some(Default::default());
    // Inner attributes are compared with the module's file, not here.
    Item::Mod(module)
}

fn fn_header(attrs: &[Attribute], vis: &syn::Visibility, sig: &syn::Signature) -> String {
    render(&Item::Fn(syn::ItemFn {
        attrs: attrs.to_vec(),
        vis: vis.clone(),
        sig: sig.clone(),
        block: Box::new(syn::parse_quote!({})),
    }))
}

/// The first differing line of two prints, with a little context.
fn first_difference(mine: &str, theirs: &str) -> String {
    let (a, b) = (mine.lines().collect::<Vec<_>>(), theirs.lines().collect::<Vec<_>>());
    let at = (0..a.len().max(b.len())).find(|&i| a.get(i) != b.get(i)).unwrap_or(0);
    let from = at.saturating_sub(2);
    let show = |lines: &[&str]| lines.iter().skip(from).take(5).copied().collect::<Vec<_>>().join("\n");
    format!("first difference at line {}:\n-- stage 1:\n{}\n-- expansion:\n{}", at + 1, show(&a), show(&b))
}

fn thread_local_names(mac: &syn::Macro) -> Vec<String> {
    let trees = mac.tokens.clone().into_iter().collect::<Vec<_>>();
    let mut names = Vec::new();
    for window in trees.windows(2) {
        if let (proc_macro2::TokenTree::Ident(keyword), proc_macro2::TokenTree::Ident(name)) = (&window[0], &window[1])
            && keyword == "static"
        {
            names.push(name.to_string());
        }
    }
    assert!(!names.is_empty(), "a thread_local! without statics");
    names
}

/// A member of an `impl` or `trait`.
enum Member<'a> {
    Impl(&'a ImplItem),
    Trait(&'a TraitItem),
}

impl Member<'_> {
    fn name(&self) -> String {
        match self {
            Member::Impl(ImplItem::Const(item)) => format!("const {}", item.ident),
            Member::Impl(ImplItem::Fn(item)) => format!("fn {}", item.sig.ident),
            Member::Impl(ImplItem::Type(item)) => format!("type {}", item.ident),
            Member::Trait(TraitItem::Const(item)) => format!("const {}", item.ident),
            Member::Trait(TraitItem::Fn(item)) => format!("fn {}", item.sig.ident),
            Member::Trait(TraitItem::Type(item)) => format!("type {}", item.ident),
            Member::Impl(other) => format!("other {}", tokens(*other)),
            Member::Trait(other) => format!("other {}", tokens(*other)),
        }
    }

    fn macros(&self) -> BTreeSet<String> {
        match self {
            Member::Impl(item) => macros_in(|m| m.visit_impl_item(item)),
            Member::Trait(item) => macros_in(|m| m.visit_trait_item(item)),
        }
    }

    fn render(&self) -> String {
        let item = match self {
            Member::Impl(item) => Item::Impl(syn::parse_quote!(impl X { #item })),
            Member::Trait(item) => Item::Trait(syn::parse_quote!(trait X { #item })),
        };
        render(&item)
    }

    fn header(&self) -> String {
        match self {
            Member::Impl(ImplItem::Fn(item)) => fn_header(&item.attrs, &item.vis, &item.sig),
            Member::Trait(TraitItem::Fn(item)) => fn_header(&item.attrs, &syn::Visibility::Inherited, &item.sig),
            other => other.name(),
        }
    }
}

// ---------------------------------------------------------------------------
// the test

fn read_manifest(root: &Path) -> toml::Value {
    let text = std::fs::read_to_string(root.join("MANIFEST.toml")).expect("MANIFEST.toml");
    toml::from_str(&text).expect("MANIFEST.toml parses")
}

fn measure_crate(root: &Path, entry: &toml::Value) -> (String, Counts, Vec<String>) {
    let name = entry["name"].as_str().unwrap().to_string();
    let dir = root.join(entry["dir"].as_str().unwrap());
    let features = entry["features"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f.as_str().unwrap().to_string())
        .collect();
    let expanded_text = std::fs::read_to_string(root.join(entry["expanded"].as_str().unwrap())).unwrap();
    // std's `pin!` expands to `super let`, which rustc accepts and syn does
    // not parse. It only ever appears inside an item that invokes `pin!` on
    // the stage 1 side, which is never compared byte for byte.
    let expanded_text = expanded_text.replace("super let ", "let ");
    let expanded = syn::parse_file(&expanded_text).unwrap_or_else(|error| panic!("{name}: the expansion does not parse: {error}"));
    let mut comparison = Comparison {
        config: config(),
        cfg: CfgContext { features },
        crate_name: &name,
        counts: Counts::default(),
        mismatches: Vec::new(),
    };
    comparison.compare_file(&dir.join("src/lib.rs"), &[], &expanded.items, &expanded.attrs);
    let Comparison { counts, mismatches, .. } = comparison;
    (name, counts, mismatches)
}

#[test]
fn differential_erasure_matches_cargo_expand_on_every_lion_crate() {
    let root = fixture_root();
    let manifest = read_manifest(&root);
    // The snapshot's expansions were made with the Verus revision this
    // transpiler vendors; after a Verus bump they must be regenerated.
    assert_eq!(
        manifest["verus_git_rev"].as_str(),
        Some(VERUS_GIT_REV),
        "{}: regenerate the snapshot (regen.sh) for the vendored Verus revision",
        root.display()
    );
    let mut table = Vec::new();
    let mut mismatches = Vec::new();
    for entry in manifest["crates"].as_array().expect("[[crates]]") {
        let (name, counts, crate_mismatches) = measure_crate(&root, entry);
        mismatches.extend(crate_mismatches);
        table.push((name, counts));
    }

    println!("differential erasure, Lion {} (Verus {})", manifest["lion_rev"].as_str().unwrap_or("?"), VERUS_GIT_REV);
    println!(
        "{:<22} {:>6} {:>6} {:>6} {:>7} {:>8} {:>9} {:>10} {:>12} {:>9}",
        "crate", "blocks", "items", "exact", "derive", "partial", "presence", "ghost-cfg", "feature-cfg", "outside"
    );
    for (name, counts) in &table {
        println!(
            "{:<22} {:>6} {:>6} {:>6} {:>7} {:>8} {:>9} {:>10} {:>12} {:>9}",
            name,
            counts.blocks,
            counts.items,
            counts.exact,
            counts.derive,
            counts.partial,
            counts.presence,
            counts.ghost_cfg_removed,
            counts.feature_cfg_removed,
            counts.outside_items
        );
        println!(
            "    partial members: {} exact, {} presence; macros outside the exact classes: {:?}",
            counts.partial_members_exact, counts.partial_members_presence, counts.presence_macros
        );
    }
    assert!(mismatches.is_empty(), "{} mismatch(es):\n{}", mismatches.len(), mismatches.join("\n\n"));

    let measured = table
        .iter()
        .map(|(name, c)| {
            (
                name.as_str(),
                c.blocks,
                c.items,
                c.exact,
                c.derive,
                c.partial,
                c.presence,
                c.ghost_cfg_removed,
                c.feature_cfg_removed,
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(measured, EXPECTED, "the measured coverage changed; update EXPECTED deliberately");
}

// ---------------------------------------------------------------------------
// negative controls: the comparison goes red on each kind of difference

fn copy_tree(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for entry in std::fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let target = to.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_tree(&entry.path(), &target);
        } else {
            std::fs::copy(entry.path(), &target).unwrap();
        }
    }
}

/// One crate of the snapshot in a scratch copy, perturbed, then compared.
fn perturbed(crate_name: &str, perturb: impl FnOnce(&Path, &mut toml::Value)) -> Vec<String> {
    let root = fixture_root();
    let manifest = read_manifest(&root);
    let mut entry = manifest["crates"]
        .as_array()
        .unwrap()
        .iter()
        .find(|entry| entry["name"].as_str() == Some(crate_name))
        .unwrap()
        .clone();
    let scratch = tempfile::tempdir().unwrap();
    let dir = entry["dir"].as_str().unwrap();
    copy_tree(&root.join(dir), &scratch.path().join(dir));
    let expanded = entry["expanded"].as_str().unwrap();
    std::fs::create_dir_all(scratch.path().join("expanded")).unwrap();
    std::fs::copy(root.join(expanded), scratch.path().join(expanded)).unwrap();
    perturb(scratch.path(), &mut entry);
    measure_crate(scratch.path(), &entry).2
}

fn edit(path: &Path, from: &str, to: &str) {
    let text = std::fs::read_to_string(path).unwrap();
    assert!(text.contains(from), "{} does not contain `{from}`", path.display());
    std::fs::write(path, text.replacen(from, to, 1)).unwrap();
}

fn assert_red(mismatches: &[String], needle: &str) {
    assert!(
        mismatches.iter().any(|mismatch| mismatch.contains(needle)),
        "expected a mismatch containing `{needle}`, got {mismatches:#?}"
    );
}

#[test]
fn differential_unperturbed_crate_is_green() {
    assert_eq!(perturbed("lion-slab", |_, _| {}), Vec::<String>::new());
}

#[test]
fn differential_detects_a_changed_erased_item() {
    // rustc's side differs from what the erasure produced...
    let mismatches = perturbed("lion-slab", |root, _| {
        edit(&root.join("expanded/lion-slab.rs"), "self.inner.get(&key)", "self.inner.get(&(key))")
    });
    assert_red(&mismatches, "Block item `impl< V : View > for Slab < V >` differs from the expansion");
    // ...or the source inside the verus! block changed after the expansion was taken.
    let mismatches = perturbed("lion-slab", |root, _| {
        edit(&root.join("tree/lion-slab/src/slab.rs"), "SLAB_CAPACITY: usize = 65536", "SLAB_CAPACITY: usize = 65537")
    });
    assert_red(&mismatches, "Block item `const SLAB_CAPACITY` differs from the expansion");
}

#[test]
fn differential_detects_an_item_only_one_side_has() {
    let mismatches = perturbed("lion-slab", |root, _| {
        edit(&root.join("expanded/lion-slab.rs"), "pub mod slab {", "pub mod slab {\n    pub fn kept_by_rustc() {}")
    });
    assert_red(&mismatches, "the expansion has an item stage 1 does not: `fn kept_by_rustc`");
    let mismatches = perturbed("lion-slab", |root, _| {
        edit(&root.join("expanded/lion-slab.rs"), "pub const SLAB_CAPACITY: usize = 65536;", "")
    });
    assert_red(&mismatches, "`const SLAB_CAPACITY` differs from the expansion (0 candidate(s)");
}

#[test]
fn differential_detects_a_missing_derived_impl() {
    let mismatches = perturbed("lion-timer-wheel", |root, _| {
        edit(
            &root.join("expanded/lion-timer-wheel.rs"),
            "    #[automatically_derived]\n    impl ::core::marker::Copy for WheelPos {}\n",
            "",
        )
    });
    assert_red(&mismatches, "`WheelPos` derives `Copy` but the expansion has no such impl");
}

#[test]
fn differential_checks_driver_and_feature_cfgs() {
    // A verus_keep_ghost item that rustc nevertheless compiled.
    let mismatches = perturbed("lion-slab", |root, _| {
        edit(
            &root.join("tree/lion-slab/src/slab.rs"),
            "pub const SLAB_CAPACITY",
            "#[cfg(verus_keep_ghost)]\npub fn ghost_only() {}\npub const SLAB_CAPACITY",
        );
        edit(&root.join("expanded/lion-slab.rs"), "pub mod slab {", "pub mod slab {\n    pub fn ghost_only() {}");
    });
    assert_red(&mismatches, "`fn ghost_only` is cfg(verus_keep_ghost)-only but rustc compiled it");
    // A feature the crate was expanded without, claimed as resolved.
    let mismatches = perturbed("lion-slab", |root, entry| {
        edit(&root.join("tree/lion-slab/src/slab.rs"), "pub const SLAB_CAPACITY", "#[cfg(feature = \"extra\")]\npub fn featured() {}\npub const SLAB_CAPACITY");
        entry["features"] = toml::Value::Array(vec![toml::Value::String("extra".to_string())]);
    });
    assert_red(&mismatches, "Block item `fn featured` differs from the expansion (0 candidate(s)");
}

#[test]
#[should_panic(expected = "has no value in this comparison")]
fn differential_rejects_a_cfg_it_cannot_evaluate() {
    perturbed("lion-slab", |root, _| {
        edit(&root.join("tree/lion-slab/src/slab.rs"), "pub const SLAB_CAPACITY", "#[cfg(target_os = \"linux\")]\npub const SLAB_CAPACITY");
    });
}
