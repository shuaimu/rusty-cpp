//! `--verus-exec`, stage 2: lower the ghost residue in Verus-erased code.
//!
//! Stage 1 (`verus_exec`) replaces every `verus! { }` block with exactly the
//! items plain rustc compiles. rustc accepts that code, but it still names
//! ghost-only vocabulary that has no C++ meaning:
//!
//! - `Ghost<T>` / `Tracked<T>` fields, tuple elements and locals, built with
//!   `Ghost::assume_new_fallback(|| unreachable!())`. Under plain rustc these
//!   are vstd's `PhantomData` wrappers and `T` is a spec type (`int`, a ghost
//!   log);
//! - `impl View for X { type V = Map<nat, _>; }` (and `DeepView`) plus
//!   `V: View` bounds, which only feed spec code;
//! - `use vstd::...` imports;
//! - spec-only datatypes (`pub type Log = Seq<Event>;`,
//!   `pub type InstantView = nat;`) whose every user was erased;
//! - vstd's executable helpers, e.g. `VecAdditionalExecFns::set`;
//! - empty `{}` / `{};` statements where proof blocks were.
//!
//! This pass rewrites the erased ASTs so codegen never sees any of it. It is
//! crate-wide (every source unit of one crate at once, because a spec-only
//! datatype and its last user often live in different files) and it runs only
//! for a crate in which stage 1 erased at least one `verus!` block; any other
//! crate, SRPC's included, is returned byte for byte. Within such a crate a
//! unit the rules do not touch is also returned byte for byte, and items
//! gated by `#[cfg(verus)]` (always false here) are left alone.
//!
//! The rules, in the order they run:
//! 1. `View`/`DeepView` impls are removed, and so are `View`/`DeepView`
//!    bounds on generic parameters, where-clauses, supertraits, associated
//!    types and `impl`/`dyn` types.
//! 2. `Ghost<T>` and `Tracked<T>` become the reserved marker type
//!    [`GHOST_MARKER`]; `Ghost::assume_new()`, `Ghost::assume_new_fallback(..)`
//!    and the `Tracked` twins become the marker value. Codegen lowers both to
//!    the empty tag `rusty::Ghost` (see `include/rusty/marker.hpp`), so tuple
//!    arity and every pattern stay as written, and `T` is never emitted.
//!    Rewriting here rather than in codegen is deliberate: this pass knows
//!    which `Ghost` is vstd's (it sees the file's vstd imports before they
//!    are dropped), and dropping `T` is what makes the spec datatypes it named
//!    unreferenced, which rule 4 depends on. Codegen only maps one reserved
//!    name.
//! 3. vstd's executable methods are lowered through [`EXEC_SURFACE`]:
//!    `v.set(i, x)` becomes `v[i] = x`, `v.set_and_swap(i, x)` becomes
//!    `core::mem::swap(&mut v[i], x)`. A table method whose receiver cannot be
//!    classified, or one without a lowering, is an error.
//! 4. Spec-only datatypes are pruned: every struct, enum, union and type
//!    alias from whose definition a vstd spec type (`nat`, `int`, `Seq`,
//!    `Set`, `Map`, `Multiset`, `FnSpec`, any `vstd::` path) is reachable,
//!    directly or through another such datatype, is removed together with
//!    its impls.
//! 5. `use` trees rooted at vstd are removed, and so are import leaves that
//!    name a pruned datatype or whose every use was erased or lowered: a name
//!    the original source used outside imports that the lowered source no
//!    longer uses (checked per file for private imports and crate-wide for
//!    `pub(crate)`/`pub(super)`; `pub` re-exports are only dropped for a
//!    pruned datatype, since other crates may use them).
//! 6. Empty `{}` / `{};` statements are removed.
//!
//! Fail-closed audit, after the rules: any remaining mention, in an
//! executable position, of a vstd spec type, `Ghost`/`Tracked`,
//! `View`/`DeepView`, a pruned datatype, or any path rooted at vstd is an
//! error; so is a ghost value (the marker value, or a local, parameter or
//! field of marker type) used as an operand, receiver, field base, index,
//! cast, condition, scrutinee or macro argument, or passed to a function that
//! does not take the marker in that position. Nothing here becomes a TODO.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use proc_macro2::{TokenStream, TokenTree};
use quote::ToTokens;
use syn::punctuated::Punctuated;
use syn::visit::{self, Visit};
use syn::visit_mut::{self, VisitMut};
use syn::{Expr, Item, Token, Type, TypeParamBound, UseTree};

/// Reserved spelling of the erased ghost tag in the source handed to codegen,
/// which lowers it to `rusty::Ghost` (type) and `rusty::Ghost{}` (value). A
/// crate that already uses the name is rejected.
pub const GHOST_MARKER: &str = "RustyVerusGhost";

/// vstd's ghost wrappers (plain-rustc `PhantomData` newtypes).
const GHOST_WRAPPERS: &[&str] = &["Ghost", "Tracked"];
/// Their only executable constructors under plain rustc
/// (`verus_builtin/src/lib.rs`: `assume_new`, `assume_new_fallback`).
const GHOST_CONSTRUCTORS: &[&str] = &["assume_new", "assume_new_fallback"];
/// vstd traits whose impls and bounds only feed spec code.
const VIEW_TRAITS: &[&str] = &["View", "DeepView"];
/// vstd types that exist only for specifications.
const SPEC_TYPES: &[&str] = &[
    "nat", "int", "Seq", "Set", "Map", "Multiset", "FnSpec", "FnProof",
];
/// Crates whose paths are Verus-only.
const VSTD_ROOTS: &[&str] = &[
    "vstd",
    "verus_builtin",
    "builtin",
    "verus_builtin_macros",
    "builtin_macros",
    "verus_state_machines_macros",
];
/// The names `use vstd::prelude::*` puts in scope under plain rustc that this
/// pass acts on (vstd/prelude.rs at the vendored revision, not(verus_keep_ghost)).
const PRELUDE_NAMES: &[&str] = &[
    "Ghost",
    "Tracked",
    "int",
    "nat",
    "Seq",
    "Set",
    "Map",
    "View",
    "DeepView",
    "FnSpec",
    "FnProof",
    "VecAdditionalExecFns",
    "ArrayAdditionalExecFns",
    "ArrayAdditionalSpecFns",
    "SliceAdditionalSpecFns",
    "StrSliceExecFns",
    "StringExecFns",
    "StringExecFnsIsAscii",
];

/// The receiver type an [`ExecItem`] method is implemented for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Receiver {
    Vec,
    Array,
    Str,
    String,
}

/// How an [`ExecItem`] lowers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Lowering {
    /// `recv.m(i, x)` → `recv[i] = x` (the vstd body is `self[i] = value`).
    IndexAssign,
    /// `recv.m(i, x)` → `core::mem::swap(&mut recv[i], x)` (the vstd body).
    IndexSwap,
    /// Same observable behaviour as the std method of the same name that the
    /// C++ runtime already provides; the call is left as written.
    SameAsStd,
    /// No lowering: using it is an error.
    Unsupported,
}

/// One vstd executable method reachable through `use vstd::prelude::*`.
struct ExecItem {
    /// The vstd trait providing it (must be in scope for the call to be vstd's).
    provider: &'static str,
    method: &'static str,
    receiver: Receiver,
    /// Argument count, excluding the receiver.
    args: usize,
    lowering: Lowering,
    /// No std method of this name and arity exists on common receivers, so a
    /// call whose receiver cannot be classified is an error rather than
    /// assumed to be someone else's method.
    vstd_only: bool,
}

/// vstd's executable surface for plain-rustc Verus code (vstd/pervasive.rs
/// `VecAdditionalExecFns`, vstd/array.rs `ArrayAdditionalExecFns`,
/// vstd/string.rs `StrSliceExecFns`/`StringExecFns`/`StringExecFnsIsAscii`).
/// The complete list of what `use vstd::prelude::*` brings into method scope;
/// every entry either lowers or is rejected.
const EXEC_SURFACE: &[ExecItem] = &[
    ExecItem {
        provider: "VecAdditionalExecFns",
        method: "set",
        receiver: Receiver::Vec,
        args: 2,
        lowering: Lowering::IndexAssign,
        vstd_only: true,
    },
    ExecItem {
        provider: "VecAdditionalExecFns",
        method: "set_and_swap",
        receiver: Receiver::Vec,
        args: 2,
        lowering: Lowering::IndexSwap,
        vstd_only: true,
    },
    ExecItem {
        provider: "ArrayAdditionalExecFns",
        method: "set",
        receiver: Receiver::Array,
        args: 2,
        lowering: Lowering::IndexAssign,
        vstd_only: true,
    },
    ExecItem {
        provider: "StrSliceExecFns",
        method: "unicode_len",
        receiver: Receiver::Str,
        args: 0,
        lowering: Lowering::Unsupported,
        vstd_only: true,
    },
    ExecItem {
        provider: "StrSliceExecFns",
        method: "get_char",
        receiver: Receiver::Str,
        args: 1,
        lowering: Lowering::Unsupported,
        vstd_only: true,
    },
    ExecItem {
        provider: "StrSliceExecFns",
        method: "substring_ascii",
        receiver: Receiver::Str,
        args: 2,
        lowering: Lowering::Unsupported,
        vstd_only: true,
    },
    ExecItem {
        provider: "StrSliceExecFns",
        method: "substring_char",
        receiver: Receiver::Str,
        args: 2,
        lowering: Lowering::Unsupported,
        vstd_only: true,
    },
    ExecItem {
        provider: "StrSliceExecFns",
        method: "get_ascii",
        receiver: Receiver::Str,
        args: 1,
        lowering: Lowering::Unsupported,
        vstd_only: true,
    },
    ExecItem {
        provider: "StrSliceExecFns",
        method: "as_bytes_vec",
        receiver: Receiver::Str,
        args: 0,
        lowering: Lowering::Unsupported,
        vstd_only: true,
    },
    ExecItem {
        provider: "StringExecFns",
        method: "append",
        receiver: Receiver::String,
        args: 1,
        lowering: Lowering::Unsupported,
        vstd_only: false,
    },
    ExecItem {
        provider: "StringExecFns",
        method: "concat",
        receiver: Receiver::String,
        args: 1,
        lowering: Lowering::Unsupported,
        vstd_only: false,
    },
    ExecItem {
        provider: "StringExecFnsIsAscii",
        method: "is_ascii",
        receiver: Receiver::String,
        args: 0,
        lowering: Lowering::SameAsStd,
        vstd_only: false,
    },
];

/// One source unit of the crate being lowered.
pub struct LowerUnit<'a> {
    /// The unit's identity (`src/...`), for messages.
    pub identity: &'a Path,
    /// The unit's text before Verus erasure.
    pub original: &'a str,
    /// Input: the erased text. Output: the lowered text (unchanged, byte for
    /// byte, when no rule applies to the unit).
    pub prepared: String,
}

/// Lower the ghost residue of one crate (see the module docs). `crate_name`
/// only labels messages.
pub fn lower_crate(crate_name: &str, units: &mut [LowerUnit<'_>]) -> Result<(), String> {
    lower_crate_with_external(crate_name, units, &BTreeSet::new()).map(|_| ())
}

/// [`lower_crate`] for a crate whose dependencies were lowered first
/// (`--crate-graph`): `external_pruned` holds the spec-only datatype names
/// pruned in those dependencies. A name this crate does not define itself is
/// treated as pruned here too (rule 4 taints the datatypes that reach it,
/// rule 5 drops its imports from a dependency, the audit rejects it in
/// executable code). Returns every pruned name, the external ones included.
pub fn lower_crate_with_external(
    crate_name: &str,
    units: &mut [LowerUnit<'_>],
    external_pruned: &BTreeSet<String>,
) -> Result<BTreeSet<String>, String> {
    let mut files = Vec::with_capacity(units.len());
    for unit in units.iter() {
        files.push(syn::parse_file(&unit.prepared).map_err(|error| {
            format!(
                "--verus-exec: {}: the erased source does not parse: {error}",
                unit.identity.display()
            )
        })?);
    }
    let mut originals = Vec::with_capacity(units.len());
    for unit in units.iter() {
        originals.push(syn::parse_file(unit.original).map_err(|error| {
            format!(
                "--verus-exec: {}: the source does not parse: {error}",
                unit.identity.display()
            )
        })?);
    }
    let labels = units
        .iter()
        .map(|unit| format!("{crate_name}/{}", unit.identity.display()))
        .collect::<Vec<_>>();

    let facts = CrateFacts::collect(&files, &originals, &labels)?;
    let external = external_pruned
        .iter()
        .filter(|name| !facts.defined.contains(*name))
        .cloned()
        .collect::<BTreeSet<_>>();
    let mut changed = vec![false; files.len()];
    let mut errors = Vec::new();

    // Rules 1, 2, 3 and 6.
    for (index, file) in files.iter_mut().enumerate() {
        let mut rewriter = Rewriter {
            scope: &facts.scopes[index],
            facts: &facts,
            label: &labels[index],
            changed: false,
            errors: Vec::new(),
            self_types: Vec::new(),
            envs: Vec::new(),
        };
        rewriter.visit_file_mut(file);
        changed[index] |= rewriter.changed;
        errors.extend(rewriter.errors);
    }
    if !errors.is_empty() {
        return Err(errors.join("; "));
    }

    // Rule 4.
    let pruned = prune_spec_datatypes(&mut files, &facts, &labels, &external, &mut changed)?;

    // Rule 5.
    let original_idents = originals.iter().map(idents_outside_uses).collect::<Vec<_>>();
    let lowered_idents = files.iter().map(idents_outside_uses).collect::<Vec<_>>();
    let crate_original = original_idents.iter().flatten().cloned().collect::<BTreeSet<_>>();
    let crate_lowered = lowered_idents.iter().flatten().cloned().collect::<BTreeSet<_>>();
    for (index, file) in files.iter_mut().enumerate() {
        let context = ImportContext {
            pruned: &pruned,
            external: &external,
            local_modules: &facts.local_modules,
            file_original: &original_idents[index],
            file_lowered: &lowered_idents[index],
            crate_original: &crate_original,
            crate_lowered: &crate_lowered,
        };
        changed[index] |= filter_imports(&mut file.items, &context);
    }

    // Fail-closed audit.
    for (index, file) in files.iter().enumerate() {
        let mut audit = Audit {
            scope: &facts.scopes[index],
            facts: &facts,
            pruned: &pruned,
            label: &labels[index],
            item: String::new(),
            errors: Vec::new(),
        };
        audit.visit_file(file);
        errors.extend(audit.errors);
        let mut flow = GhostFlow::new(&facts, &labels[index]);
        flow.visit_file(file);
        errors.extend(flow.errors);
    }
    if !errors.is_empty() {
        return Err(errors.join("; "));
    }

    for ((unit, file), changed) in units.iter_mut().zip(&files).zip(changed) {
        if changed {
            unit.prepared = prettyplease::unparse(file);
        }
    }
    Ok(pruned)
}

// ---------------------------------------------------------------------------
// crate facts and name scopes

/// What `use` items make visible from vstd in one file.
#[derive(Default)]
struct FileScope {
    /// Names imported from vstd by an explicit leaf (under their local name).
    vstd_names: BTreeSet<String>,
    /// Names made visible by a glob import of a vstd module.
    vstd_glob_names: BTreeSet<String>,
    /// Names the file defines or imports from somewhere other than vstd;
    /// these shadow vstd's.
    shadowing: BTreeSet<String>,
}

impl FileScope {
    fn is_vstd(&self, name: &str) -> bool {
        !self.shadowing.contains(name)
            && (self.vstd_names.contains(name) || self.vstd_glob_names.contains(name))
    }
}

/// Crate-wide facts gathered before any rewrite.
struct CrateFacts {
    scopes: Vec<FileScope>,
    /// Struct name → (field name or tuple index → declared type), one entry
    /// per definition of that name.
    structs: BTreeMap<String, Vec<BTreeMap<String, Type>>>,
    /// Names of every item the crate defines.
    defined: BTreeSet<String>,
    /// Names of the crate's modules (for crate-relative `use` paths).
    local_modules: BTreeSet<String>,
    /// Field names declared with a ghost wrapper type in some struct, and
    /// field names declared with another type in some struct.
    ghost_fields: BTreeSet<String>,
    plain_fields: BTreeSet<String>,
    /// Function / method name → ghost-wrapper flag per (non-self) parameter,
    /// one entry per definition.
    fn_params: BTreeMap<String, Vec<Vec<bool>>>,
}

fn vstd_glob_module_names(path: &[String]) -> &'static [&'static str] {
    match path.last().map(String::as_str) {
        Some("seq") => &["Seq"],
        Some("set") => &["Set"],
        Some("map") => &["Map"],
        Some("multiset") => &["Multiset"],
        Some("view") => &["View", "DeepView"],
        Some("pervasive") => &["VecAdditionalExecFns"],
        Some("array") => &["ArrayAdditionalExecFns", "ArrayAdditionalSpecFns"],
        Some("slice") => &["SliceAdditionalSpecFns"],
        Some("string") => &["StrSliceExecFns", "StringExecFns", "StringExecFnsIsAscii"],
        // `vstd::prelude::*`, `vstd::*`, `verus_builtin::*` and any module
        // this list does not know: assume the prelude's names.
        _ => PRELUDE_NAMES,
    }
}

/// Visit every `use` leaf: `(path prefix, imported source name, local name,
/// is_glob)`.
fn for_each_use_leaf(tree: &UseTree, prefix: &mut Vec<String>, f: &mut dyn FnMut(&[String], &str, &str, bool)) {
    match tree {
        UseTree::Path(path) => {
            prefix.push(path.ident.to_string());
            for_each_use_leaf(&path.tree, prefix, f);
            prefix.pop();
        }
        UseTree::Name(name) => {
            let ident = name.ident.to_string();
            let local = if ident == "self" {
                prefix.last().cloned().unwrap_or(ident.clone())
            } else {
                ident.clone()
            };
            f(prefix, &ident, &local, false);
        }
        UseTree::Rename(rename) => {
            f(prefix, &rename.ident.to_string(), &rename.rename.to_string(), false);
        }
        UseTree::Glob(_) => f(prefix, "*", "*", true),
        UseTree::Group(group) => {
            for tree in &group.items {
                for_each_use_leaf(tree, prefix, f);
            }
        }
    }
}

fn is_vstd_root(segment: &str) -> bool {
    VSTD_ROOTS.contains(&segment)
}

/// True for `#[cfg(verus)]`, which is always false in the transpiler.
fn is_cfg_verus(attrs: &[syn::Attribute]) -> bool {
    attrs.iter().any(|attr| {
        attr.path().is_ident("cfg")
            && attr
                .parse_args::<syn::Path>()
                .is_ok_and(|path| path.is_ident("verus"))
    })
}

fn item_attrs(item: &Item) -> &[syn::Attribute] {
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

fn item_name(item: &Item) -> Option<String> {
    Some(
        match item {
            Item::Const(item) => &item.ident,
            Item::Enum(item) => &item.ident,
            Item::Fn(item) => &item.sig.ident,
            Item::Macro(item) => item.ident.as_ref()?,
            Item::Mod(item) => &item.ident,
            Item::Static(item) => &item.ident,
            Item::Struct(item) => &item.ident,
            Item::Trait(item) => &item.ident,
            Item::TraitAlias(item) => &item.ident,
            Item::Type(item) => &item.ident,
            Item::Union(item) => &item.ident,
            _ => return None,
        }
        .to_string(),
    )
}

/// True when `ty` is a ghost wrapper after rule 2 (the marker type).
fn is_marker_type(ty: &Type) -> bool {
    matches!(ty, Type::Path(path) if path.qself.is_none() && path.path.is_ident(GHOST_MARKER))
}

fn is_marker_expr(expr: &Expr) -> bool {
    matches!(expr, Expr::Path(path) if path.qself.is_none() && path.path.is_ident(GHOST_MARKER))
}

impl CrateFacts {
    fn collect(files: &[syn::File], originals: &[syn::File], labels: &[String]) -> Result<Self, String> {
        let mut facts = CrateFacts {
            scopes: Vec::with_capacity(files.len()),
            structs: BTreeMap::new(),
            defined: BTreeSet::new(),
            local_modules: BTreeSet::new(),
            ghost_fields: BTreeSet::new(),
            plain_fields: BTreeSet::new(),
            fn_params: BTreeMap::new(),
        };
        let mut errors = Vec::new();
        for (index, (file, original)) in files.iter().zip(originals).enumerate() {
            if idents_in(&original.to_token_stream()).contains(GHOST_MARKER) {
                errors.push(format!(
                    "--verus-exec: {}: `{GHOST_MARKER}` is reserved for lowered Verus ghost state",
                    labels[index]
                ));
            }
            let mut scope = FileScope::default();
            collect_items(&file.items, &mut scope, &mut facts);
            facts.scopes.push(scope);
        }
        if errors.is_empty() {
            Ok(facts)
        } else {
            Err(errors.join("; "))
        }
    }
}

fn collect_items(items: &[Item], scope: &mut FileScope, facts: &mut CrateFacts) {
    for item in items {
        if is_cfg_verus(item_attrs(item)) {
            continue;
        }
        if let Some(name) = item_name(item) {
            facts.defined.insert(name.clone());
            scope.shadowing.insert(name);
        }
        match item {
            Item::Use(item_use) => {
                let mut prefix = Vec::new();
                for_each_use_leaf(&item_use.tree, &mut prefix, &mut |path, _source, local, glob| {
                    let vstd = path.first().is_some_and(|root| is_vstd_root(root));
                    match (vstd, glob) {
                        (true, true) => scope
                            .vstd_glob_names
                            .extend(vstd_glob_module_names(path).iter().map(|name| name.to_string())),
                        (true, false) => {
                            scope.vstd_names.insert(local.to_string());
                        }
                        (false, false) => {
                            scope.shadowing.insert(local.to_string());
                        }
                        (false, true) => {}
                    }
                });
            }
            Item::Mod(module) => {
                facts.local_modules.insert(module.ident.to_string());
                if let Some((_, content)) = &module.content {
                    collect_items(content, scope, facts);
                }
            }
            Item::Struct(item) => {
                let mut fields = BTreeMap::new();
                for (position, field) in item.fields.iter().enumerate() {
                    let name = field
                        .ident
                        .as_ref()
                        .map(ToString::to_string)
                        .unwrap_or_else(|| position.to_string());
                    if field.ident.is_some() {
                        if is_ghost_wrapper_type_syntax(&field.ty) {
                            facts.ghost_fields.insert(name.clone());
                        } else {
                            facts.plain_fields.insert(name.clone());
                        }
                    }
                    fields.insert(name, field.ty.clone());
                }
                facts.structs.entry(item.ident.to_string()).or_default().push(fields);
            }
            Item::Fn(item) => record_fn(&item.sig, facts),
            Item::Impl(item) => {
                for impl_item in &item.items {
                    if let syn::ImplItem::Fn(method) = impl_item {
                        record_fn(&method.sig, facts);
                    }
                }
            }
            Item::Trait(item) => {
                for trait_item in &item.items {
                    if let syn::TraitItem::Fn(method) = trait_item {
                        record_fn(&method.sig, facts);
                    }
                }
            }
            _ => {}
        }
    }
}

fn record_fn(sig: &syn::Signature, facts: &mut CrateFacts) {
    let flags = sig
        .inputs
        .iter()
        .filter_map(|input| match input {
            syn::FnArg::Typed(typed) => Some(is_ghost_wrapper_type_syntax(&typed.ty)),
            syn::FnArg::Receiver(_) => None,
        })
        .collect();
    facts.fn_params.entry(sig.ident.to_string()).or_default().push(flags);
}

/// `Ghost<..>`/`Tracked<..>` (any qualification) or the marker, before scope
/// resolution; used for the crate tables built before rewriting.
fn is_ghost_wrapper_type_syntax(ty: &Type) -> bool {
    match ty {
        Type::Path(path) if path.qself.is_none() => path.path.segments.last().is_some_and(|segment| {
            GHOST_WRAPPERS.contains(&segment.ident.to_string().as_str())
                || segment.ident == GHOST_MARKER
        }),
        _ => false,
    }
}

/// All identifiers in a token stream, including inside groups.
fn idents_in(tokens: &TokenStream) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    fn walk(tokens: TokenStream, out: &mut BTreeSet<String>) {
        for tree in tokens {
            match tree {
                TokenTree::Ident(ident) => {
                    out.insert(ident.to_string());
                }
                TokenTree::Group(group) => walk(group.stream(), out),
                _ => {}
            }
        }
    }
    walk(tokens.clone(), &mut out);
    out
}

/// Identifiers a file uses outside its `use` items (including inside macro
/// invocations, so a name used only inside a `verus!` block counts as used in
/// the original).
fn idents_outside_uses(file: &syn::File) -> BTreeSet<String> {
    fn walk(items: &[Item], out: &mut BTreeSet<String>) {
        for item in items {
            match item {
                Item::Use(_) => {}
                Item::Mod(module) if module.content.is_some() => {
                    out.insert(module.ident.to_string());
                    walk(&module.content.as_ref().unwrap().1, out);
                }
                _ => out.extend(idents_in(&item.to_token_stream())),
            }
        }
    }
    let mut out = BTreeSet::new();
    walk(&file.items, &mut out);
    out
}

// ---------------------------------------------------------------------------
// rules 1, 2, 3 and 6

/// Syntactic kind of a receiver expression, for [`EXEC_SURFACE`].
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Kind {
    Vec,
    Array,
    Str,
    String,
    /// A struct the crate defines.
    Named(String),
    /// Some other known type.
    Other,
}

impl Kind {
    fn receiver(&self) -> Option<Receiver> {
        match self {
            Kind::Vec => Some(Receiver::Vec),
            Kind::Array => Some(Receiver::Array),
            Kind::Str => Some(Receiver::Str),
            Kind::String => Some(Receiver::String),
            Kind::Named(_) | Kind::Other => None,
        }
    }
}

/// Locals of one function body whose kind is known (`None`: bound more than
/// once with different kinds).
#[derive(Default)]
struct LocalEnv {
    locals: BTreeMap<String, Option<Kind>>,
}

impl LocalEnv {
    fn bind(&mut self, name: String, kind: Option<Kind>) {
        match self.locals.get(&name) {
            Some(existing) if *existing != kind => {
                self.locals.insert(name, None);
            }
            _ => {
                self.locals.insert(name, kind);
            }
        }
    }
}

fn type_kind(ty: &Type, self_type: Option<&str>, facts: &CrateFacts) -> Option<Kind> {
    match ty {
        Type::Reference(reference) => type_kind(&reference.elem, self_type, facts),
        Type::Paren(paren) => type_kind(&paren.elem, self_type, facts),
        Type::Group(group) => type_kind(&group.elem, self_type, facts),
        Type::Array(_) => Some(Kind::Array),
        Type::Path(path) if path.qself.is_none() => {
            let last = path.path.segments.last()?.ident.to_string();
            let std_name = |name: &str| last == name && !facts.defined.contains(name);
            if std_name("Vec") {
                Some(Kind::Vec)
            } else if std_name("String") {
                Some(Kind::String)
            } else if last == "str" {
                Some(Kind::Str)
            } else if last == "Self" && path.path.segments.len() == 1 {
                self_type.map(|name| Kind::Named(name.to_string()))
            } else if facts.structs.contains_key(&last) {
                Some(Kind::Named(last))
            } else {
                Some(Kind::Other)
            }
        }
        _ => Some(Kind::Other),
    }
}

fn peel(expr: &Expr) -> &Expr {
    match expr {
        Expr::Paren(paren) => peel(&paren.expr),
        Expr::Group(group) => peel(&group.expr),
        Expr::Reference(reference) => peel(&reference.expr),
        Expr::Unary(unary) if matches!(unary.op, syn::UnOp::Deref(_)) => peel(&unary.expr),
        _ => expr,
    }
}

fn expr_kind(
    expr: &Expr,
    env: Option<&LocalEnv>,
    self_type: Option<&str>,
    facts: &CrateFacts,
) -> Option<Kind> {
    match peel(expr) {
        Expr::Path(path) if path.qself.is_none() && path.path.segments.len() == 1 => {
            let name = path.path.segments[0].ident.to_string();
            if name == "self" {
                return self_type.map(|name| Kind::Named(name.to_string()));
            }
            env?.locals.get(&name).cloned().flatten()
        }
        Expr::Field(field) => {
            let Kind::Named(owner) = expr_kind(&field.base, env, self_type, facts)? else {
                return None;
            };
            let member = match &field.member {
                syn::Member::Named(ident) => ident.to_string(),
                syn::Member::Unnamed(index) => index.index.to_string(),
            };
            let definitions = facts.structs.get(&owner)?;
            let kinds = definitions
                .iter()
                .map(|fields| {
                    fields
                        .get(&member)
                        .and_then(|ty| type_kind(ty, Some(&owner), facts))
                })
                .collect::<BTreeSet<_>>();
            if kinds.len() == 1 {
                kinds.into_iter().next().flatten()
            } else {
                None
            }
        }
        Expr::Lit(lit) if matches!(lit.lit, syn::Lit::Str(_)) => Some(Kind::Str),
        Expr::Call(call) => {
            let Expr::Path(path) = &*call.func else {
                return None;
            };
            let segments = path
                .path
                .segments
                .iter()
                .map(|segment| segment.ident.to_string())
                .collect::<Vec<_>>();
            match segments.iter().map(String::as_str).collect::<Vec<_>>().as_slice() {
                [owner, "new" | "with_capacity" | "from"] if *owner == "Vec" && !facts.defined.contains("Vec") => {
                    Some(Kind::Vec)
                }
                [owner, "new" | "from" | "with_capacity"]
                    if *owner == "String" && !facts.defined.contains("String") =>
                {
                    Some(Kind::String)
                }
                _ => None,
            }
        }
        Expr::Struct(literal) if literal.qself.is_none() => {
            let name = literal.path.segments.last()?.ident.to_string();
            if name == "Self" {
                self_type.map(|name| Kind::Named(name.to_string()))
            } else if facts.structs.contains_key(&name) {
                Some(Kind::Named(name))
            } else {
                Some(Kind::Other)
            }
        }
        Expr::Macro(mac) if mac.mac.path.is_ident("vec") => Some(Kind::Vec),
        Expr::Macro(mac) if mac.mac.path.is_ident("format") => Some(Kind::String),
        Expr::Array(_) | Expr::Repeat(_) => Some(Kind::Array),
        _ => None,
    }
}

/// Collect the locals of one function body with a known kind.
fn build_env(
    sig: &syn::Signature,
    block: Option<&syn::Block>,
    self_type: Option<&str>,
    facts: &CrateFacts,
) -> LocalEnv {
    struct Lets<'a> {
        env: LocalEnv,
        self_type: Option<&'a str>,
        facts: &'a CrateFacts,
    }
    impl<'ast> Visit<'ast> for Lets<'_> {
        fn visit_local(&mut self, local: &'ast syn::Local) {
            let (pat, ty) = match &local.pat {
                syn::Pat::Type(typed) => (&*typed.pat, Some(&*typed.ty)),
                pat => (pat, None),
            };
            if let syn::Pat::Ident(ident) = pat {
                let kind = match ty {
                    Some(ty) => type_kind(ty, self.self_type, self.facts),
                    None => local.init.as_ref().and_then(|init| {
                        expr_kind(&init.expr, Some(&self.env), self.self_type, self.facts)
                    }),
                };
                self.env.bind(ident.ident.to_string(), kind);
            }
            visit::visit_local(self, local);
        }
        fn visit_item(&mut self, _: &'ast Item) {}
    }
    let mut lets = Lets {
        env: LocalEnv::default(),
        self_type,
        facts,
    };
    for input in &sig.inputs {
        if let syn::FnArg::Typed(typed) = input
            && let syn::Pat::Ident(ident) = &*typed.pat
        {
            let kind = type_kind(&typed.ty, self_type, facts);
            lets.env.bind(ident.ident.to_string(), kind);
        }
    }
    if let Some(block) = block {
        lets.visit_block(block);
    }
    lets.env
}

struct Rewriter<'a> {
    scope: &'a FileScope,
    facts: &'a CrateFacts,
    label: &'a str,
    changed: bool,
    errors: Vec<String>,
    self_types: Vec<Option<String>>,
    envs: Vec<LocalEnv>,
}

impl Rewriter<'_> {
    /// A path naming one of `names` as vstd's: a single segment the file's
    /// vstd imports make visible, or any path rooted at a vstd crate.
    fn names_vstd(&self, path: &syn::Path, names: &[&str]) -> bool {
        let Some(last) = path.segments.last() else {
            return false;
        };
        let last = last.ident.to_string();
        if !names.contains(&last.as_str()) {
            return false;
        }
        if path.segments.len() == 1 {
            path.leading_colon.is_none() && self.scope.is_vstd(&last)
        } else {
            is_vstd_root(&path.segments[0].ident.to_string())
        }
    }

    fn is_view_bound(&self, bound: &TypeParamBound) -> bool {
        matches!(bound, TypeParamBound::Trait(trait_bound) if self.names_vstd(&trait_bound.path, VIEW_TRAITS))
    }

    fn strip_view_bounds(&mut self, bounds: &mut Punctuated<TypeParamBound, Token![+]>) {
        if bounds.iter().any(|bound| self.is_view_bound(bound)) {
            let kept = std::mem::take(bounds)
                .into_iter()
                .filter(|bound| !self.is_view_bound(bound))
                .collect();
            *bounds = kept;
            self.changed = true;
        }
    }

    /// `Ghost<T>` / `Tracked<T>` as a type.
    fn is_ghost_wrapper_type(&self, ty: &Type) -> bool {
        let Type::Path(path) = ty else {
            return false;
        };
        path.qself.is_none()
            && self.names_vstd(&path.path, GHOST_WRAPPERS)
            && matches!(
                &path.path.segments.last().unwrap().arguments,
                syn::PathArguments::AngleBracketed(args) if args.args.len() == 1
            )
    }

    /// `Ghost::assume_new()`, `Ghost::<T>::assume_new_fallback(..)`, ...
    fn is_ghost_constructor(&self, func: &Expr) -> bool {
        let Expr::Path(path) = func else {
            return false;
        };
        let segments = &path.path.segments;
        if path.qself.is_some() || segments.len() < 2 {
            return false;
        }
        let constructor = segments.last().unwrap().ident.to_string();
        if !GHOST_CONSTRUCTORS.contains(&constructor.as_str()) {
            return false;
        }
        let mut owner = path.path.clone();
        owner.segments.pop();
        let owner_last = owner.segments.pop().unwrap().into_value();
        let mut owner_path = owner.clone();
        owner_path.segments.push(syn::PathSegment::from(owner_last.ident));
        self.names_vstd(&owner_path, GHOST_WRAPPERS)
    }

    fn self_type(&self) -> Option<&str> {
        self.self_types.last().and_then(|ty| ty.as_deref())
    }

    fn with_fn_env(&mut self, sig: &syn::Signature, block: Option<&syn::Block>) {
        let env = build_env(sig, block, self.self_type(), self.facts);
        self.envs.push(env);
    }

    /// Rule 3 for one method call; `Some(replacement)` when it lowers.
    fn lower_exec_call(&mut self, call: &syn::ExprMethodCall) -> Option<Expr> {
        let method = call.method.to_string();
        let candidates = EXEC_SURFACE
            .iter()
            .filter(|item| {
                item.method == method
                    && item.args == call.args.len()
                    && self.scope.is_vstd(item.provider)
            })
            .collect::<Vec<_>>();
        if candidates.is_empty() {
            return None;
        }
        let kind = expr_kind(&call.receiver, self.envs.last(), self.self_type(), self.facts);
        let receiver = match kind.as_ref() {
            Some(kind) => kind.receiver(),
            None => {
                if candidates.iter().any(|item| item.vstd_only) {
                    self.errors.push(format!(
                        "--verus-exec: {}: cannot tell whether `.{method}(..)` on `{}` is vstd's `{}::{method}`; its receiver's type is not known here (annotate the binding with its type)",
                        self.label,
                        call.receiver.to_token_stream(),
                        candidates[0].provider,
                    ));
                }
                return None;
            }
        };
        let item = candidates.iter().find(|item| Some(item.receiver) == receiver)?;
        match item.lowering {
            Lowering::SameAsStd => None,
            Lowering::Unsupported => {
                self.errors.push(format!(
                    "--verus-exec: {}: vstd's `{}::{method}` has no C++ lowering",
                    self.label, item.provider
                ));
                None
            }
            Lowering::IndexAssign | Lowering::IndexSwap => {
                // Indexing binds tighter than a prefix operator or a cast, so
                // only a postfix-shaped receiver can be indexed as written.
                let receiver: Expr = match &*call.receiver {
                    receiver @ (Expr::Path(_)
                    | Expr::Field(_)
                    | Expr::Index(_)
                    | Expr::MethodCall(_)
                    | Expr::Call(_)
                    | Expr::Paren(_)) => receiver.clone(),
                    receiver => syn::parse_quote!((#receiver)),
                };
                let receiver = &receiver;
                let index = &call.args[0];
                let value = &call.args[1];
                let swap = item.lowering == Lowering::IndexSwap;
                let assign = |index: &Expr, value: &Expr| -> Expr {
                    if swap {
                        syn::parse_quote!(::core::mem::swap(&mut #receiver[#index], #value))
                    } else {
                        syn::parse_quote!(#receiver[#index] = #value)
                    }
                };
                // Rust evaluates the call's arguments left to right before the
                // body's `self[i] = value`; keep that order when either
                // argument could have a side effect.
                Some(if is_pure(index) && is_pure(value) {
                    assign(index, value)
                } else {
                    let index_name = syn::Ident::new("__vstd_exec_index", proc_macro2::Span::call_site());
                    let value_name = syn::Ident::new("__vstd_exec_value", proc_macro2::Span::call_site());
                    let body = assign(
                        &syn::parse_quote!(#index_name),
                        &syn::parse_quote!(#value_name),
                    );
                    syn::parse_quote!({
                        let #index_name = #index;
                        let #value_name = #value;
                        #body;
                    })
                })
            }
        }
    }
}

/// An expression whose evaluation cannot have a side effect or depend on one:
/// paths, literals, field and tuple-index projections, casts, references,
/// and constructor calls (`Some(x)`, `Pair(a, b)`) over such operands.
fn is_pure(expr: &Expr) -> bool {
    match expr {
        Expr::Path(_) | Expr::Lit(_) => true,
        Expr::Field(field) => is_pure(&field.base),
        Expr::Paren(paren) => is_pure(&paren.expr),
        Expr::Group(group) => is_pure(&group.expr),
        Expr::Cast(cast) => is_pure(&cast.expr),
        Expr::Reference(reference) => is_pure(&reference.expr),
        Expr::Tuple(tuple) => tuple.elems.iter().all(is_pure),
        Expr::Call(call) => {
            matches!(&*call.func, Expr::Path(path) if path.path.segments.last().is_some_and(|segment| {
                segment.ident.to_string().starts_with(|c: char| c.is_ascii_uppercase())
            })) && call.args.iter().all(is_pure)
        }
        _ => false,
    }
}

fn is_empty_block_stmt(stmt: &syn::Stmt) -> bool {
    matches!(
        stmt,
        syn::Stmt::Expr(Expr::Block(block), _)
            if block.attrs.is_empty() && block.label.is_none() && block.block.stmts.is_empty()
    )
}

impl VisitMut for Rewriter<'_> {
    fn visit_file_mut(&mut self, file: &mut syn::File) {
        self.items(&mut file.items);
        visit_mut::visit_file_mut(self, file);
    }

    fn visit_item_mod_mut(&mut self, module: &mut syn::ItemMod) {
        if let Some((_, items)) = &mut module.content {
            self.items(items);
        }
        visit_mut::visit_item_mod_mut(self, module);
    }

    fn visit_item_mut(&mut self, item: &mut Item) {
        if is_cfg_verus(item_attrs(item)) {
            return;
        }
        visit_mut::visit_item_mut(self, item);
    }

    fn visit_item_impl_mut(&mut self, item: &mut syn::ItemImpl) {
        let self_type = match &*item.self_ty {
            Type::Path(path) => path.path.segments.last().map(|segment| segment.ident.to_string()),
            _ => None,
        };
        self.self_types.push(self_type);
        visit_mut::visit_item_impl_mut(self, item);
        self.self_types.pop();
    }

    fn visit_item_fn_mut(&mut self, item: &mut syn::ItemFn) {
        self.self_types.push(None);
        self.with_fn_env(&item.sig, Some(&item.block));
        visit_mut::visit_item_fn_mut(self, item);
        self.envs.pop();
        self.self_types.pop();
    }

    fn visit_impl_item_fn_mut(&mut self, item: &mut syn::ImplItemFn) {
        self.with_fn_env(&item.sig, Some(&item.block));
        visit_mut::visit_impl_item_fn_mut(self, item);
        self.envs.pop();
    }

    fn visit_trait_item_fn_mut(&mut self, item: &mut syn::TraitItemFn) {
        self.self_types.push(None);
        self.with_fn_env(&item.sig, item.default.as_ref());
        visit_mut::visit_trait_item_fn_mut(self, item);
        self.envs.pop();
        self.self_types.pop();
    }

    fn visit_type_param_mut(&mut self, param: &mut syn::TypeParam) {
        self.strip_view_bounds(&mut param.bounds);
        if param.bounds.is_empty() {
            param.colon_token = None;
        }
        visit_mut::visit_type_param_mut(self, param);
    }

    fn visit_where_clause_mut(&mut self, clause: &mut syn::WhereClause) {
        for predicate in clause.predicates.iter_mut() {
            if let syn::WherePredicate::Type(predicate) = predicate {
                self.strip_view_bounds(&mut predicate.bounds);
            }
        }
        let before = clause.predicates.len();
        let kept = std::mem::take(&mut clause.predicates)
            .into_iter()
            .filter(|predicate| {
                !matches!(predicate, syn::WherePredicate::Type(predicate) if predicate.bounds.is_empty())
            })
            .collect();
        clause.predicates = kept;
        self.changed |= clause.predicates.len() != before;
        visit_mut::visit_where_clause_mut(self, clause);
    }

    fn visit_generics_mut(&mut self, generics: &mut syn::Generics) {
        visit_mut::visit_generics_mut(self, generics);
        if generics
            .where_clause
            .as_ref()
            .is_some_and(|clause| clause.predicates.is_empty())
        {
            generics.where_clause = None;
            self.changed = true;
        }
    }

    fn visit_item_trait_mut(&mut self, item: &mut syn::ItemTrait) {
        self.strip_view_bounds(&mut item.supertraits);
        if item.supertraits.is_empty() {
            item.colon_token = None;
        }
        visit_mut::visit_item_trait_mut(self, item);
    }

    fn visit_trait_item_type_mut(&mut self, item: &mut syn::TraitItemType) {
        self.strip_view_bounds(&mut item.bounds);
        if item.bounds.is_empty() {
            item.colon_token = None;
        }
        visit_mut::visit_trait_item_type_mut(self, item);
    }

    fn visit_type_impl_trait_mut(&mut self, ty: &mut syn::TypeImplTrait) {
        self.strip_view_bounds(&mut ty.bounds);
        visit_mut::visit_type_impl_trait_mut(self, ty);
    }

    fn visit_type_trait_object_mut(&mut self, ty: &mut syn::TypeTraitObject) {
        self.strip_view_bounds(&mut ty.bounds);
        visit_mut::visit_type_trait_object_mut(self, ty);
    }

    fn visit_type_mut(&mut self, ty: &mut Type) {
        if self.is_ghost_wrapper_type(ty) {
            *ty = syn::parse_str(GHOST_MARKER).expect("marker parses as a type");
            self.changed = true;
            return;
        }
        visit_mut::visit_type_mut(self, ty);
    }

    fn visit_expr_mut(&mut self, expr: &mut Expr) {
        if let Expr::Call(call) = expr
            && self.is_ghost_constructor(&call.func)
        {
            *expr = syn::parse_str(GHOST_MARKER).expect("marker parses as an expression");
            self.changed = true;
            return;
        }
        visit_mut::visit_expr_mut(self, expr);
        if let Expr::MethodCall(call) = expr
            && let Some(replacement) = self.lower_exec_call(call)
        {
            *expr = replacement;
            self.changed = true;
        }
    }

    fn visit_block_mut(&mut self, block: &mut syn::Block) {
        visit_mut::visit_block_mut(self, block);
        let before = block.stmts.len();
        block.stmts.retain(|stmt| !is_empty_block_stmt(stmt));
        self.changed |= block.stmts.len() != before;
    }
}

impl Rewriter<'_> {
    /// Rule 1 at item level: drop `View`/`DeepView` impls.
    fn items(&mut self, items: &mut Vec<Item>) {
        let before = items.len();
        items.retain(|item| match item {
            Item::Impl(item) => !(item
                .trait_
                .as_ref()
                .is_some_and(|(_, path, _)| self.names_vstd(path, VIEW_TRAITS))
                && !is_cfg_verus(&item.attrs)),
            _ => true,
        });
        self.changed |= items.len() != before;
    }
}

// ---------------------------------------------------------------------------
// rule 4: spec-only datatypes

/// Names referenced by an item, and whether any path in it is rooted at vstd.
#[derive(Default)]
struct References {
    names: BTreeSet<String>,
    vstd_path: bool,
}

impl<'ast> Visit<'ast> for References {
    fn visit_path(&mut self, path: &'ast syn::Path) {
        if let Some(first) = path.segments.first()
            && is_vstd_root(&first.ident.to_string())
        {
            self.vstd_path = true;
        }
        for segment in &path.segments {
            self.names.insert(segment.ident.to_string());
        }
        visit::visit_path(self, path);
    }

    fn visit_attribute(&mut self, _: &'ast syn::Attribute) {}
}

/// A datatype definition: `(unit index, name, references)`.
struct Datatype {
    unit: usize,
    name: String,
    references: References,
}

fn collect_datatypes(items: &[Item], unit: usize, out: &mut Vec<Datatype>) {
    for item in items {
        if is_cfg_verus(item_attrs(item)) {
            continue;
        }
        let mut references = References::default();
        let name = match item {
            Item::Struct(item) => {
                references.visit_item_struct(item);
                &item.ident
            }
            Item::Enum(item) => {
                references.visit_item_enum(item);
                &item.ident
            }
            Item::Union(item) => {
                references.visit_item_union(item);
                &item.ident
            }
            Item::Type(item) => {
                references.visit_item_type(item);
                &item.ident
            }
            Item::Mod(module) => {
                if let Some((_, content)) = &module.content {
                    collect_datatypes(content, unit, out);
                }
                continue;
            }
            _ => continue,
        };
        out.push(Datatype {
            unit,
            name: name.to_string(),
            references,
        });
    }
}

/// Rule 4: remove every datatype that reaches vstd spec vocabulary; returns
/// their names.
fn prune_spec_datatypes(
    files: &mut [syn::File],
    facts: &CrateFacts,
    labels: &[String],
    external: &BTreeSet<String>,
    changed: &mut [bool],
) -> Result<BTreeSet<String>, String> {
    let mut datatypes = Vec::new();
    for (unit, file) in files.iter().enumerate() {
        collect_datatypes(&file.items, unit, &mut datatypes);
    }
    // Spec-only datatypes of lowered dependencies that this crate does not
    // shadow with a definition of its own.
    let mut tainted = external.clone();
    loop {
        let before = tainted.len();
        for datatype in &datatypes {
            let scope = &facts.scopes[datatype.unit];
            let reaches_spec = datatype.references.vstd_path
                || datatype.references.names.iter().any(|name| {
                    (SPEC_TYPES.contains(&name.as_str()) && scope.is_vstd(name))
                        || (tainted.contains(name) && *name != datatype.name)
                });
            if reaches_spec {
                tainted.insert(datatype.name.clone());
            }
        }
        if tainted.len() == before {
            break;
        }
    }
    // A name defined both as a spec-only datatype and as a datatype that is
    // not cannot be pruned by name.
    let ambiguous = datatypes
        .iter()
        .filter(|datatype| tainted.contains(&datatype.name))
        .filter(|datatype| {
            datatypes.iter().any(|other| {
                other.name == datatype.name
                    && !(other.references.vstd_path
                        || other.references.names.iter().any(|name| {
                            (SPEC_TYPES.contains(&name.as_str())
                                && facts.scopes[other.unit].is_vstd(name))
                                || (tainted.contains(name) && *name != other.name)
                        }))
            })
        })
        .map(|datatype| format!("`{}` ({})", datatype.name, labels[datatype.unit]))
        .collect::<BTreeSet<_>>();
    if !ambiguous.is_empty() {
        return Err(format!(
            "--verus-exec: spec-only datatype name(s) also name executable datatypes: {}",
            ambiguous.into_iter().collect::<Vec<_>>().join(", ")
        ));
    }
    for (unit, file) in files.iter_mut().enumerate() {
        changed[unit] |= prune_items(&mut file.items, &tainted);
    }
    Ok(tainted)
}

fn impl_self_name(item: &syn::ItemImpl) -> Option<String> {
    match &*item.self_ty {
        Type::Path(path) => path.path.segments.last().map(|segment| segment.ident.to_string()),
        _ => None,
    }
}

fn prune_items(items: &mut Vec<Item>, tainted: &BTreeSet<String>) -> bool {
    let before = items.len();
    items.retain(|item| {
        if is_cfg_verus(item_attrs(item)) {
            return true;
        }
        match item {
            Item::Struct(_) | Item::Enum(_) | Item::Union(_) | Item::Type(_) => {
                !item_name(item).is_some_and(|name| tainted.contains(&name))
            }
            Item::Impl(item) => !impl_self_name(item).is_some_and(|name| tainted.contains(&name)),
            _ => true,
        }
    });
    let mut changed = items.len() != before;
    for item in items.iter_mut() {
        if let Item::Mod(module) = item
            && let Some((_, content)) = &mut module.content
        {
            changed |= prune_items(content, tainted);
        }
    }
    changed
}

// ---------------------------------------------------------------------------
// rule 5: imports

struct ImportContext<'a> {
    pruned: &'a BTreeSet<String>,
    /// The subset of `pruned` that dependencies pruned.
    external: &'a BTreeSet<String>,
    local_modules: &'a BTreeSet<String>,
    file_original: &'a BTreeSet<String>,
    file_lowered: &'a BTreeSet<String>,
    crate_original: &'a BTreeSet<String>,
    crate_lowered: &'a BTreeSet<String>,
}

impl ImportContext<'_> {
    fn keep_leaf(&self, visibility: &syn::Visibility, path: &[String], source: &str, local: &str) -> bool {
        let crate_relative = path.first().is_some_and(|root| {
            matches!(root.as_str(), "crate" | "self" | "super") || self.local_modules.contains(root)
        });
        let imported = if source == "self" { local } else { source };
        if crate_relative && self.pruned.contains(imported) {
            return false;
        }
        // A dependency's spec-only datatype, imported from that dependency.
        let from_dependency = path
            .first()
            .is_some_and(|root| !crate_relative && !matches!(root.as_str(), "std" | "core" | "alloc"));
        if from_dependency && self.external.contains(imported) {
            return false;
        }
        // An import the lowering orphaned: used by the original outside
        // imports, and no longer used by the lowered source.
        match visibility {
            syn::Visibility::Inherited => {
                !(self.file_original.contains(local) && !self.file_lowered.contains(local))
            }
            syn::Visibility::Restricted(_) => {
                !(self.crate_original.contains(local) && !self.crate_lowered.contains(local))
            }
            syn::Visibility::Public(_) => true,
        }
    }
}

fn filter_use_tree(
    tree: &UseTree,
    prefix: &mut Vec<String>,
    keep: &dyn Fn(&[String], &str, &str) -> bool,
) -> Option<UseTree> {
    match tree {
        UseTree::Path(path) => {
            let segment = path.ident.to_string();
            if prefix.is_empty() && is_vstd_root(&segment) {
                return None;
            }
            prefix.push(segment);
            let inner = filter_use_tree(&path.tree, prefix, keep);
            prefix.pop();
            let mut path = path.clone();
            *path.tree = inner?;
            Some(UseTree::Path(path))
        }
        UseTree::Name(name) => {
            let source = name.ident.to_string();
            let local = if source == "self" {
                prefix.last().cloned().unwrap_or_default()
            } else {
                source.clone()
            };
            if prefix.is_empty() && is_vstd_root(&source) {
                return None;
            }
            keep(prefix, &source, &local).then(|| tree.clone())
        }
        UseTree::Rename(rename) => {
            let source = rename.ident.to_string();
            if prefix.is_empty() && is_vstd_root(&source) {
                return None;
            }
            keep(prefix, &source, &rename.rename.to_string()).then(|| tree.clone())
        }
        UseTree::Glob(_) => Some(tree.clone()),
        UseTree::Group(group) => {
            let items = group
                .items
                .iter()
                .filter_map(|tree| filter_use_tree(tree, prefix, keep))
                .collect::<Punctuated<UseTree, Token![,]>>();
            if items.is_empty() {
                return None;
            }
            let mut group = group.clone();
            group.items = items;
            Some(UseTree::Group(group))
        }
    }
}

/// Rule 5 over one item list (and inline modules); returns whether anything
/// changed.
fn filter_imports(items: &mut Vec<Item>, context: &ImportContext<'_>) -> bool {
    let mut changed = false;
    let mut kept = Vec::with_capacity(items.len());
    for mut item in std::mem::take(items) {
        match &mut item {
            Item::Use(item_use) if !is_cfg_verus(&item_use.attrs) => {
                let visibility = item_use.vis.clone();
                let keep = |path: &[String], source: &str, local: &str| {
                    context.keep_leaf(&visibility, path, source, local)
                };
                match filter_use_tree(&item_use.tree, &mut Vec::new(), &keep) {
                    Some(tree) => {
                        if tree.to_token_stream().to_string() != item_use.tree.to_token_stream().to_string() {
                            item_use.tree = tree;
                            changed = true;
                        }
                        kept.push(item);
                    }
                    None => changed = true,
                }
            }
            Item::Mod(module) if !is_cfg_verus(&module.attrs) => {
                if let Some((_, content)) = &mut module.content {
                    changed |= filter_imports(content, context);
                }
                kept.push(item);
            }
            _ => kept.push(item),
        }
    }
    *items = kept;
    changed
}

// ---------------------------------------------------------------------------
// fail-closed audit

struct Audit<'a> {
    scope: &'a FileScope,
    facts: &'a CrateFacts,
    pruned: &'a BTreeSet<String>,
    label: &'a str,
    item: String,
    errors: Vec<String>,
}

impl Audit<'_> {
    fn report(&mut self, what: String) {
        let item = if self.item.is_empty() {
            String::new()
        } else {
            format!(" (in `{}`)", self.item)
        };
        self.errors.push(format!("--verus-exec: {}{item}: {what}", self.label));
    }

    /// Check a path in a type position (`type_position`: a single segment
    /// names a type) or a value/pattern position (a single segment may be a
    /// local binding, so only qualified paths are judged by their root).
    fn check_path(&mut self, path: &syn::Path, type_position: bool) {
        let Some(first) = path.segments.first() else {
            return;
        };
        let root = first.ident.to_string();
        if is_vstd_root(&root) {
            self.report(format!(
                "`{}` is a vstd path this pass does not lower (not in the vstd executable-surface table)",
                path.to_token_stream().to_string().replace(' ', "")
            ));
            return;
        }
        if root == GHOST_MARKER {
            return;
        }
        if path.segments.len() == 1 && !type_position {
            return;
        }
        let vstd_name = self.scope.is_vstd(&root) && !self.facts.defined.contains(&root);
        if vstd_name && SPEC_TYPES.contains(&root.as_str()) {
            self.report(format!("the vstd spec type `{root}` is used in executable code"));
        } else if vstd_name && GHOST_WRAPPERS.contains(&root.as_str()) {
            self.report(format!(
                "`{}` survives lowering (only `{root}<T>` types and `{root}::assume_new*` constructors lower)",
                path.to_token_stream().to_string().replace(' ', "")
            ));
        } else if vstd_name && VIEW_TRAITS.contains(&root.as_str()) {
            self.report(format!("the vstd spec trait `{root}` is used in executable code"));
        } else if self.pruned.contains(&root) {
            self.report(format!(
                "the spec-only datatype `{root}` (pruned because it reaches vstd spec types) is used in executable code"
            ));
        }
    }
}

impl<'ast> Visit<'ast> for Audit<'_> {
    fn visit_item(&mut self, item: &'ast Item) {
        if is_cfg_verus(item_attrs(item)) || matches!(item, Item::Use(_)) {
            return;
        }
        let outer = std::mem::take(&mut self.item);
        self.item = item_name(item).unwrap_or_else(|| match item {
            Item::Impl(item) => format!("impl {}", item.self_ty.to_token_stream()),
            _ => outer.clone(),
        });
        visit::visit_item(self, item);
        self.item = outer;
    }

    fn visit_type_path(&mut self, ty: &'ast syn::TypePath) {
        if ty.qself.is_none() {
            self.check_path(&ty.path, true);
        }
        visit::visit_type_path(self, ty);
    }

    fn visit_trait_bound(&mut self, bound: &'ast syn::TraitBound) {
        self.check_path(&bound.path, true);
        visit::visit_trait_bound(self, bound);
    }

    fn visit_expr_path(&mut self, expr: &'ast syn::ExprPath) {
        if expr.qself.is_none() {
            self.check_path(&expr.path, false);
        }
        visit::visit_expr_path(self, expr);
    }

    fn visit_expr_struct(&mut self, expr: &'ast syn::ExprStruct) {
        self.check_path(&expr.path, true);
        visit::visit_expr_struct(self, expr);
    }

    fn visit_pat_struct(&mut self, pat: &'ast syn::PatStruct) {
        self.check_path(&pat.path, true);
        visit::visit_pat_struct(self, pat);
    }

    fn visit_pat_tuple_struct(&mut self, pat: &'ast syn::PatTupleStruct) {
        self.check_path(&pat.path, false);
        visit::visit_pat_tuple_struct(self, pat);
    }

    fn visit_item_impl(&mut self, item: &'ast syn::ItemImpl) {
        if let Some((_, path, _)) = &item.trait_ {
            self.check_path(path, true);
        }
        visit::visit_item_impl(self, item);
    }

    fn visit_macro(&mut self, mac: &'ast syn::Macro) {
        self.check_path(&mac.path, false);
        visit::visit_macro(self, mac);
    }

    fn visit_attribute(&mut self, _: &'ast syn::Attribute) {}
}

/// Ghost values used where their value would matter.
struct GhostFlow<'a> {
    facts: &'a CrateFacts,
    label: &'a str,
    /// Ghost-typed locals of the enclosing function bodies.
    locals: Vec<BTreeSet<String>>,
    errors: Vec<String>,
}

impl<'a> GhostFlow<'a> {
    fn new(facts: &'a CrateFacts, label: &'a str) -> Self {
        GhostFlow {
            facts,
            label,
            locals: Vec::new(),
            errors: Vec::new(),
        }
    }

    fn is_ghost(&self, expr: &Expr) -> bool {
        match peel(expr) {
            expr if is_marker_expr(expr) => true,
            Expr::Path(path) if path.qself.is_none() && path.path.segments.len() == 1 => {
                let name = path.path.segments[0].ident.to_string();
                self.locals.last().is_some_and(|locals| locals.contains(&name))
            }
            Expr::Field(field) => match &field.member {
                syn::Member::Named(name) => {
                    let name = name.to_string();
                    self.facts.ghost_fields.contains(&name) && !self.facts.plain_fields.contains(&name)
                }
                syn::Member::Unnamed(_) => false,
            },
            _ => false,
        }
    }

    fn reject(&mut self, expr: &Expr, context: &str) {
        if self.is_ghost(expr) {
            self.errors.push(format!(
                "--verus-exec: {}: the ghost value `{}` is used as {context}, a position that needs its (erased) value",
                self.label,
                expr.to_token_stream().to_string()
            ));
        }
    }

    fn enter_fn(&mut self, sig: &syn::Signature, block: &syn::Block) {
        let mut locals = BTreeSet::new();
        for input in &sig.inputs {
            if let syn::FnArg::Typed(typed) = input
                && let syn::Pat::Ident(ident) = &*typed.pat
                && is_marker_type(&typed.ty)
            {
                locals.insert(ident.ident.to_string());
            }
        }
        struct Lets<'b>(&'b mut BTreeSet<String>);
        impl<'ast> Visit<'ast> for Lets<'_> {
            fn visit_local(&mut self, local: &'ast syn::Local) {
                let (pat, ty) = match &local.pat {
                    syn::Pat::Type(typed) => (&*typed.pat, Some(&*typed.ty)),
                    pat => (pat, None),
                };
                if let syn::Pat::Ident(ident) = pat {
                    let ghost = ty.is_some_and(is_marker_type)
                        || local.init.as_ref().is_some_and(|init| is_marker_expr(&init.expr));
                    if ghost {
                        self.0.insert(ident.ident.to_string());
                    }
                }
                visit::visit_local(self, local);
            }
            fn visit_item(&mut self, _: &'ast Item) {}
        }
        Lets(&mut locals).visit_block(block);
        self.locals.push(locals);
    }

    fn check_call_args(&mut self, name: Option<String>, args: &Punctuated<Expr, Token![,]>) {
        for (position, arg) in args.iter().enumerate() {
            if !self.is_ghost(arg) {
                continue;
            }
            let accepted = name.as_ref().is_some_and(|name| {
                if name.starts_with(|c: char| c.is_ascii_uppercase()) {
                    // A tuple-struct or enum-variant constructor stores it.
                    return true;
                }
                self.facts.fn_params.get(name).is_some_and(|signatures| {
                    let matching = signatures
                        .iter()
                        .filter(|flags| flags.len() == args.len())
                        .collect::<Vec<_>>();
                    !matching.is_empty() && matching.iter().all(|flags| flags[position])
                })
            });
            if !accepted {
                self.errors.push(format!(
                    "--verus-exec: {}: the ghost value `{}` is passed to `{}`, which does not take a ghost value in that position",
                    self.label,
                    arg.to_token_stream(),
                    name.as_deref().unwrap_or("<expression>")
                ));
            }
        }
    }
}

impl<'ast> Visit<'ast> for GhostFlow<'_> {
    fn visit_item(&mut self, item: &'ast Item) {
        if is_cfg_verus(item_attrs(item)) {
            return;
        }
        visit::visit_item(self, item);
    }

    fn visit_item_fn(&mut self, item: &'ast syn::ItemFn) {
        self.enter_fn(&item.sig, &item.block);
        visit::visit_item_fn(self, item);
        self.locals.pop();
    }

    fn visit_impl_item_fn(&mut self, item: &'ast syn::ImplItemFn) {
        self.enter_fn(&item.sig, &item.block);
        visit::visit_impl_item_fn(self, item);
        self.locals.pop();
    }

    fn visit_trait_item_fn(&mut self, item: &'ast syn::TraitItemFn) {
        if let Some(block) = &item.default {
            self.enter_fn(&item.sig, block);
            visit::visit_trait_item_fn(self, item);
            self.locals.pop();
        }
    }

    fn visit_expr(&mut self, expr: &'ast Expr) {
        match expr {
            Expr::MethodCall(call) => {
                if call.method != "clone" {
                    self.reject(&call.receiver, &format!("the receiver of `.{}()`", call.method));
                }
                self.check_call_args(Some(call.method.to_string()), &call.args);
            }
            Expr::Call(call) => {
                let name = match &*call.func {
                    Expr::Path(path) => path.path.segments.last().map(|segment| segment.ident.to_string()),
                    _ => None,
                };
                self.check_call_args(name, &call.args);
            }
            Expr::Binary(binary) => {
                self.reject(&binary.left, "an operand");
                self.reject(&binary.right, "an operand");
            }
            Expr::Unary(unary) => self.reject(&unary.expr, "an operand"),
            Expr::Field(field) => self.reject(&field.base, "a field base"),
            Expr::Index(index) => {
                self.reject(&index.expr, "an indexed value");
                self.reject(&index.index, "an index");
            }
            Expr::Cast(cast) => self.reject(&cast.expr, "a cast operand"),
            Expr::If(expr_if) => self.reject(&expr_if.cond, "a condition"),
            Expr::While(expr_while) => self.reject(&expr_while.cond, "a condition"),
            Expr::Match(expr_match) => self.reject(&expr_match.expr, "a match scrutinee"),
            Expr::Macro(mac) => self.check_macro(&mac.mac),
            _ => {}
        }
        visit::visit_expr(self, expr);
    }

    fn visit_stmt_macro(&mut self, stmt: &'ast syn::StmtMacro) {
        self.check_macro(&stmt.mac);
        visit::visit_stmt_macro(self, stmt);
    }
}

impl GhostFlow<'_> {
    fn check_macro(&mut self, mac: &syn::Macro) {
        let locals = self.locals.last().cloned().unwrap_or_default();
        let used = idents_in(&mac.tokens);
        if used.contains(GHOST_MARKER) || used.iter().any(|name| locals.contains(name)) {
            self.errors.push(format!(
                "--verus-exec: {}: a ghost value is used inside `{}!`",
                self.label,
                mac.path.to_token_stream().to_string().replace(' ', "")
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn pretty(source: &str) -> String {
        prettyplease::unparse(&syn::parse_file(source).expect("expected source parses"))
    }

    /// Lower a crate given as `(identity, erased source)` pairs whose
    /// original text is the same as the erased text.
    fn lower(units: &[(&str, &str)]) -> Result<Vec<String>, String> {
        lower_with_originals(
            &units
                .iter()
                .map(|(identity, source)| (*identity, *source, *source))
                .collect::<Vec<_>>(),
        )
    }

    fn lower_with_originals(units: &[(&str, &str, &str)]) -> Result<Vec<String>, String> {
        let identities = units.iter().map(|(identity, ..)| PathBuf::from(identity)).collect::<Vec<_>>();
        let mut lowered = units
            .iter()
            .zip(&identities)
            .map(|((_, original, erased), identity)| LowerUnit {
                identity,
                original,
                prepared: erased.to_string(),
            })
            .collect::<Vec<_>>();
        lower_crate("demo", &mut lowered)?;
        Ok(lowered.into_iter().map(|unit| unit.prepared).collect())
    }

    fn lower_one(source: &str) -> Result<String, String> {
        lower(&[("src/lib.rs", source)]).map(|mut out| out.remove(0))
    }

    #[test]
    fn view_impls_and_view_bounds_are_dropped() {
        let out = lower_one(
            r#"
use vstd::prelude::*;
pub struct Slab<V: View> { pub inner: Vec<Option<V>> }
impl<V: View> View for Slab<V> { type V = Map<nat, V::V>; }
impl<V: DeepView + Copy> DeepView for Slab<V> { type V = Seq<V::V>; }
impl<V: View + Copy> Slab<V> where V: View, V: Clone + View {
    pub fn new() -> Self { Slab { inner: Vec::new() } }
}
pub trait Keyed: View { fn key(&self) -> u64; }
"#,
        )
        .unwrap();
        assert_eq!(
            out,
            pretty(
                r#"
pub struct Slab<V> { pub inner: Vec<Option<V>> }
impl<V: Copy> Slab<V> where V: Clone {
    pub fn new() -> Self { Slab { inner: Vec::new() } }
}
pub trait Keyed { fn key(&self) -> u64; }
"#
            )
        );
    }

    #[test]
    fn vstd_imports_are_dropped_and_other_imports_kept() {
        let out = lower_one(
            r#"
use vstd::prelude::*;
use ::vstd::seq::Seq;
pub use vstd::prelude::nat;
use {vstd::set::Set, std::cell::Cell};
use std::collections::HashMap;
pub fn f(c: &Cell<u8>, _m: &HashMap<u8, u8>) -> u8 { c.get() }
"#,
        )
        .unwrap();
        assert_eq!(
            out,
            pretty(
                r#"
use {std::cell::Cell};
use std::collections::HashMap;
pub fn f(c: &Cell<u8>, _m: &HashMap<u8, u8>) -> u8 { c.get() }
"#
            )
        );
    }

    #[test]
    fn ghost_and_tracked_lower_to_the_marker_everywhere() {
        let out = lower_one(
            r#"
use vstd::prelude::*;
pub type InstantView = nat;
pub struct Log { pub events: Seq<u64> }
#[derive(Clone, Copy)]
pub struct Entry { pub deadline: u64, pub log_index: Ghost<int> }
pub struct Reactor { pub log: Ghost<Log>, pub token: Tracked<u8> }
impl Reactor {
    pub fn new() -> Self {
        Reactor { log: Ghost::assume_new_fallback(|| unreachable!()), token: Tracked::assume_new() }
    }
    fn pop(&mut self) -> Option<(u64, Ghost<int>, Ghost<InstantView>)> {
        let log_idx: Ghost<int> = Ghost::assume_new_fallback(|| unreachable!());
        let deadline: Ghost<InstantView> = Ghost::<InstantView>::assume_new();
        let entry = Entry { deadline: 1, log_index: log_idx };
        Some((entry.deadline, log_idx, deadline))
    }
    fn drain(&mut self) {
        match self.pop() {
            Some((d, _g, _h)) => { let _ = d; }
            None => {}
        }
    }
}
"#,
        )
        .unwrap();
        assert_eq!(
            out,
            pretty(
                r#"
#[derive(Clone, Copy)]
pub struct Entry { pub deadline: u64, pub log_index: RustyVerusGhost }
pub struct Reactor { pub log: RustyVerusGhost, pub token: RustyVerusGhost }
impl Reactor {
    pub fn new() -> Self {
        Reactor { log: RustyVerusGhost, token: RustyVerusGhost }
    }
    fn pop(&mut self) -> Option<(u64, RustyVerusGhost, RustyVerusGhost)> {
        let log_idx: RustyVerusGhost = RustyVerusGhost;
        let deadline: RustyVerusGhost = RustyVerusGhost;
        let entry = Entry { deadline: 1, log_index: log_idx };
        Some((entry.deadline, log_idx, deadline))
    }
    fn drain(&mut self) {
        match self.pop() {
            Some((d, _g, _h)) => { let _ = d; }
            None => {}
        }
    }
}
"#
            )
        );
        for spec in ["nat", "int", "Seq", "Log", "InstantView", "Ghost", "Tracked"] {
            assert!(
                !idents_in(&syn::parse_file(&out).unwrap().to_token_stream()).contains(spec),
                "{spec} survives: {out}"
            );
        }
    }

    #[test]
    fn spec_only_datatypes_are_pruned_across_files_with_their_impls_and_imports() {
        let out = lower(&[
            (
                "src/spec.rs",
                r#"
use vstd::prelude::*;
pub type TID = nat;
pub struct TaskView { pub id: TID }
pub enum Event { Tick, Drain { ids: Seq<TID> } }
pub enum Outer { Inner(Event) }
pub type Log = Seq<Outer>;
impl TaskView { pub fn id(&self) -> u64 { 0 } }
pub enum DrainSource { TaskWake, Deferred }
pub struct Exec { pub id: u64 }
"#,
            ),
            (
                "src/exec.rs",
                r#"
use vstd::prelude::*;
use crate::spec::{Log, TaskView, Exec};
pub struct Executor { pub log: Ghost<Log>, pub exec: Exec }
impl View for Executor { type V = TaskView; }
"#,
            ),
        ])
        .unwrap();
        assert_eq!(
            out[0],
            pretty("pub enum DrainSource { TaskWake, Deferred } pub struct Exec { pub id: u64 }")
        );
        assert_eq!(
            out[1],
            pretty(
                "use crate::spec::{Exec}; pub struct Executor { pub log: RustyVerusGhost, pub exec: Exec }"
            )
        );
    }

    #[test]
    fn imports_orphaned_by_erasure_are_dropped_and_unused_ones_kept() {
        let original = r#"
use vstd::prelude::*;
use other_crate::types::{InstantView, Duration};
use std::io::Write;
verus! {
pub struct S { pub d: Duration }
pub open spec fn later(i: InstantView) -> bool { true }
}
"#;
        let erased = r#"
use vstd::prelude::*;
use other_crate::types::{InstantView, Duration};
use std::io::Write;
pub struct S { pub d: Duration }
"#;
        let out = lower_with_originals(&[("src/lib.rs", original, erased)]).unwrap();
        assert_eq!(
            out[0],
            pretty(
                r#"
use other_crate::types::{Duration};
use std::io::Write;
pub struct S { pub d: Duration }
"#
            )
        );
    }

    #[test]
    fn vec_set_lowers_to_index_assignment() {
        let out = lower_one(
            r#"
use vstd::prelude::*;
pub struct Slab<V> { pub inner: Vec<Option<V>>, pub words: [u64; 4] }
impl<V> Slab<V> {
    fn set_slot(&mut self, idx: usize, value: V) {
        self.inner.set(idx, Some(value));
    }
    fn fill(&mut self, v: &mut Vec<u64>, mut x: u64) {
        let mut local: Vec<u8> = Vec::new();
        local.set(0, 1u8);
        let mut other = Vec::with_capacity(2);
        other.set(1, 2u64);
        v.set(next(), next());
        v.set_and_swap(0, &mut x);
        self.words.set(1, 7);
    }
}
fn next() -> usize { 0 }
"#,
        )
        .unwrap();
        assert_eq!(
            out,
            pretty(
                r#"
pub struct Slab<V> { pub inner: Vec<Option<V>>, pub words: [u64; 4] }
impl<V> Slab<V> {
    fn set_slot(&mut self, idx: usize, value: V) {
        self.inner[idx] = Some(value);
    }
    fn fill(&mut self, v: &mut Vec<u64>, mut x: u64) {
        let mut local: Vec<u8> = Vec::new();
        local[0] = 1u8;
        let mut other = Vec::with_capacity(2);
        other[1] = 2u64;
        {
            let __vstd_exec_index = next();
            let __vstd_exec_value = next();
            v[__vstd_exec_index] = __vstd_exec_value;
        };
        ::core::mem::swap(&mut v[0], &mut x);
        self.words[1] = 7;
    }
}
fn next() -> usize { 0 }
"#
            )
        );
    }

    #[test]
    fn set_on_other_receivers_is_left_alone_and_without_the_prelude_is_not_vstds() {
        let source = r#"
use vstd::prelude::*;
use std::cell::Cell;
pub struct Bits { pub cell: Cell<u8> }
impl Bits {
    fn set(&self, _i: usize, _v: bool) {}
    fn f(&self) { self.cell.set(1); let b = Bits { cell: Cell::new(0) }; let b: Bits = b; b.set(0, true); }
}
"#;
        let out = lower_one(source).unwrap();
        assert!(out.contains("b.set(0, true)") && out.contains("self.cell.set(1)"), "{out}");
        let without_prelude = "pub fn f(v: &mut Vec<u8>, w: Wrapper) { v.set(0, 1); w.x.set(1, 2); } verus_marker_fn!();";
        // Not a Verus file at all: `.set` stays whatever it is.
        let out = lower_one(without_prelude).unwrap();
        assert_eq!(out, without_prelude, "untouched units are returned byte for byte");
    }

    #[test]
    fn empty_statements_are_removed() {
        let out = lower_one(
            "use vstd::prelude::*; fn f(x: u64) -> u64 { {} let y = x; {}; if y > 0 { {} } y }",
        )
        .unwrap();
        assert_eq!(out, pretty("fn f(x: u64) -> u64 { let y = x; if y > 0 {} y }"));
    }

    #[test]
    fn unit_without_residue_is_returned_byte_for_byte() {
        let source = "pub  fn   plain( ) -> u8 { 1 }\n";
        assert_eq!(lower_one(source).unwrap(), source);
    }

    #[test]
    fn cfg_verus_items_are_left_alone() {
        let source = "#[cfg(verus)]\nuse vstd::prelude::*;\npub fn f() {}\n";
        assert_eq!(lower_one(source).unwrap(), source);
    }

    fn lower_error(source: &str) -> String {
        lower_one(source).expect_err(source)
    }

    #[test]
    fn ghost_values_in_value_positions_fail_closed() {
        for (body, needle) in [
            ("let g: Ghost<int> = Ghost::assume_new(); let x = g + 1;", "an operand"),
            ("let g: Ghost<int> = Ghost::assume_new(); g.view();", "receiver of `.view()`"),
            ("let g: Ghost<int> = Ghost::assume_new(); if g { }", "a condition"),
            ("let g: Ghost<int> = Ghost::assume_new(); takes_u64(g);", "passed to `takes_u64`"),
            ("let g: Ghost<int> = Ghost::assume_new(); println!(\"{:?}\", g);", "inside `println!`"),
            ("let t: Tracked<u8> = Tracked::assume_new(); let n = t as u64;", "a cast operand"),
        ] {
            let error = lower_error(&format!(
                "use vstd::prelude::*; fn takes_u64(_x: u64) {{}} fn f() {{ {body} }}"
            ));
            assert!(error.contains(needle), "{body}: {error}");
        }
        // Passing a ghost to a parameter declared ghost is fine.
        lower_one(
            "use vstd::prelude::*; fn keep(_g: Ghost<int>) {} fn f() { let g: Ghost<int> = Ghost::assume_new(); keep(g); let o = Some(g); let _ = o; }",
        )
        .unwrap();
    }

    #[test]
    fn surviving_spec_vocabulary_fails_closed() {
        for (source, needle) in [
            ("use vstd::prelude::*; pub fn f(x: nat) {}", "spec type `nat`"),
            ("use vstd::prelude::*; pub fn f() -> Seq<u8> { unimplemented!() }", "spec type `Seq`"),
            (
                "use vstd::prelude::*; pub type Log = Seq<u8>; pub fn f(_l: &Log) {}",
                "spec-only datatype `Log`",
            ),
            (
                "use vstd::prelude::*; pub fn f() { vstd::pervasive::runtime_assert(true); }",
                "`vstd::pervasive::runtime_assert` is a vstd path",
            ),
            (
                "use vstd::prelude::*; pub fn f() { let _g: Ghost<u8> = Ghost::new(1); }",
                "`Ghost::new` survives lowering",
            ),
            (
                "use vstd::prelude::*; pub fn f<T: View>(t: T) -> T::V { t.view() }",
                "",
            ),
            ("use vstd::prelude::*; pub fn f(s: &str) -> usize { s.unicode_len() }", "has no C++ lowering"),
            (
                "use vstd::prelude::*; pub fn f(w: &Wrapper) { w.get().set(0, 1); }",
                "cannot tell whether `.set(..)`",
            ),
            ("use vstd::prelude::*; pub struct RustyVerusGhost;", "reserved"),
        ] {
            let error = lower_one(source);
            if needle.is_empty() {
                // `T::V` after its `View` bound is dropped is a Rust error the
                // C++ side reports; what must not happen is a silent success
                // that still names `View`.
                if let Ok(out) = error {
                    assert!(!out.contains("View"), "{out}");
                }
                continue;
            }
            let error = error.expect_err(source);
            assert!(error.contains(needle), "{source}: {error}");
        }
    }

    #[test]
    fn spec_and_exec_datatypes_sharing_a_name_fail_closed() {
        let error = lower(&[
            ("src/a.rs", "use vstd::prelude::*; pub type Id = nat;"),
            ("src/b.rs", "pub struct Id(pub u64);"),
        ])
        .unwrap_err();
        assert!(error.contains("also name executable datatypes"), "{error}");
    }

    #[test]
    fn codegen_maps_the_reserved_marker_to_the_ghost_tag() {
        assert_eq!(crate::types::map_std_type(GHOST_MARKER), Some(("rusty::Ghost", false)));
    }

    #[test]
    fn exec_surface_table_covers_every_vstd_exec_trait_the_prelude_exports() {
        for provider in [
            "VecAdditionalExecFns",
            "ArrayAdditionalExecFns",
            "StrSliceExecFns",
            "StringExecFns",
            "StringExecFnsIsAscii",
        ] {
            assert!(PRELUDE_NAMES.contains(&provider));
            assert!(EXEC_SURFACE.iter().any(|item| item.provider == provider), "{provider}");
        }
    }
}
