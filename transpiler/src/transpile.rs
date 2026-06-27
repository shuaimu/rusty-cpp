use crate::codegen::CodeGen;
use crate::types::UserTypeMap;
use quote::ToTokens;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use syn::visit::{self, Visit};

/// Cross-crate UFCS trait manifest (book § 3.2.7). Emitted as a sidecar JSON
/// next to a crate's `.cppm`, and consumed when
/// transpiling a dependent crate so it can classify + module-qualify calls to
/// the dependency's trait methods (`<module>::<Tr>_::m`). Records ONLY methods
/// for which an `<Tr>_::m` free function was ACTUALLY emitted (the pruned
/// owner map), so a consumer never qualifies to a non-existent symbol.
#[derive(Clone, Debug, Default, Serialize, Deserialize, Eq, PartialEq)]
pub struct UfcsTraitManifest {
    #[serde(default = "default_ufcs_trait_manifest_version")]
    pub version: u32,
    /// C++ module name the trait namespaces live in (e.g. "itertools").
    pub module: String,
    /// Trait names this crate DECLARES (for `use dep::Tr` recognition).
    #[serde(default)]
    pub declared_traits: Vec<String>,
    /// Declared trait name → the method names it declares (required + default).
    /// Lets a downstream crate's UFCS dedup be METHOD-AWARE: a dep declaring a
    /// trait of the same NAME (e.g. the ubiquitous private `Sealed`) must not
    /// suppress THIS crate's same-named-but-unrelated trait's free functions
    /// unless the dep's trait actually provides the same method.
    #[serde(default)]
    pub declared_trait_methods: BTreeMap<String, Vec<String>>,
    /// Method name → owning trait names, restricted to actually-emitted
    /// `<Tr>_::m` free functions.
    #[serde(default)]
    pub method_owners: BTreeMap<String, Vec<String>>,
    /// Types this crate declares, with the metadata a downstream crate needs to
    /// reference them across the C++ module boundary (book § 3.2.7): the
    /// declaration-module path (so a re-exported name can be QUALIFIED rather than
    /// bound to a same-named enclosing namespace — e.g. serde's `private_::de`)
    /// and the generic-TYPE-param arity (so `BytesDeserializer` is emitted as
    /// `BytesDeserializer<E>`, not bare). Only types with an UNAMBIGUOUS module
    /// path are listed.
    #[serde(default)]
    pub declared_types: Vec<UfcsDeclaredType>,
}

/// One entry of `UfcsTraitManifest::declared_types` (book § 3.2.7): cross-crate
/// type metadata for a crate-declared type.
#[derive(Clone, Debug, Default, Serialize, Deserialize, Eq, PartialEq)]
pub struct UfcsDeclaredType {
    /// Bare type name, e.g. `BytesDeserializer`.
    pub name: String,
    /// `::`-joined, C++-escaped declaration-module path, e.g. `de::value`.
    pub module_path: String,
    /// Number of generic TYPE params (lifetimes/consts excluded), e.g. 1 for
    /// `BytesDeserializer<E>`.
    pub arity: usize,
}

fn default_ufcs_trait_manifest_version() -> u32 {
    1
}

/// Load + merge dependency UFCS trait manifests (book § 3.2.7). Later entries
/// don't conflict in practice (distinct crate modules); on the same method/trait
/// the union is taken. Missing files are skipped (best-effort, like dep .cppm).
pub fn load_ufcs_trait_manifests(paths: &[PathBuf]) -> Vec<UfcsTraitManifest> {
    let mut out = Vec::new();
    for p in paths {
        let Ok(text) = fs::read_to_string(p) else {
            continue;
        };
        if let Ok(m) = serde_json::from_str::<UfcsTraitManifest>(&text) {
            out.push(m);
        }
    }
    out
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct CppModuleSymbolIndex {
    pub modules: BTreeMap<String, CppModuleIndexModule>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct CppModuleIndexModule {
    pub namespace: Option<String>,
    pub symbols: BTreeMap<String, CppModuleIndexSymbol>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct CppModuleIndexSymbol {
    pub kind: Option<String>,
    pub callable_signatures: Vec<String>,
}

#[derive(Clone, Debug, Deserialize)]
struct CppModuleSymbolIndexFile {
    #[serde(default = "default_cpp_module_symbol_index_version")]
    version: u32,
    #[serde(default)]
    modules: BTreeMap<String, CppModuleIndexModuleFile>,
}

#[derive(Clone, Debug, Default, Deserialize)]
struct CppModuleIndexModuleFile {
    #[serde(default)]
    namespace: Option<String>,
    #[serde(default)]
    symbols: BTreeMap<String, CppModuleIndexSymbolFile>,
}

#[derive(Clone, Debug, Default, Deserialize)]
struct CppModuleIndexSymbolFile {
    #[serde(default)]
    kind: Option<String>,
    #[serde(default)]
    callable_signatures: Vec<String>,
}

fn default_cpp_module_symbol_index_version() -> u32 {
    1
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TranspileOptions {
    /// Opt-in diagnostic-only prototype for by-value SCC cycle-breaking planning.
    /// Default is `false`.
    pub by_value_cycle_breaking_prototype: bool,
    /// Optional C++ module symbol index for `use cpp::...` interop resolution.
    pub cpp_module_symbol_index: Option<CppModuleSymbolIndex>,
    /// Source paths used to load the configured C++ module symbol index.
    /// Used in diagnostics so unresolved-symbol errors point to the configured index input.
    pub cpp_module_symbol_index_sources: Vec<PathBuf>,
    /// Maps Rust external crate roots to transpiled C++ module namespaces available
    /// in the current compilation unit (for example `serde_core` -> `serde_core`).
    pub external_crate_module_aliases: HashMap<String, String>,
    /// UFCS cross-crate (book § 3.2.7): when set, write a `UfcsTraitManifest`
    /// JSON here after emission (records this crate's declared traits + the
    /// actually-emitted `<Tr>_::m` owner map). No-op unless a path is set.
    pub emit_ufcs_trait_manifest_path: Option<PathBuf>,
    /// UFCS cross-crate: dependency manifest paths to load + merge, so calls to
    /// a dependency's trait methods classify and qualify to `<module>::<Tr>_::m`.
    pub dependency_ufcs_trait_manifests: Vec<PathBuf>,
    /// In module mode, prefer `import std;` over explicit standard-header includes.
    /// Requires Stage D toolchain setup that provides a prebuilt `std` module.
    pub use_import_std_in_modules: bool,
    /// Prefer `rusty::Unit` alias spelling for Rust `()` in generated
    /// output. Defaults to `true` (see `impl Default`) — the two C++
    /// types are identical via `using Unit = std::tuple<>;` but the
    /// alias reads cleaner in generated DSL code. Set `false` (or pass
    /// `--prefer-std-tuple-alias` on the CLI) to keep the legacy
    /// `std::tuple<>` spelling.
    pub prefer_rusty_unit_alias: bool,
    /// Prefer `rusty::StrView` / `rusty::Span<...>` spellings in generated output.
    pub prefer_rusty_view_aliases: bool,
    /// Lower Rust traits to plain C++ Interface + Adapter classes
    /// (replaces `pro::proxy<...>` facade emission).
    /// See docs/rusty-cpp-transpiler.md § 3.2.9 for the design.
    pub interface_traits: bool,
    /// True when transpiling a single inline-rust `#if RUSTYCPP_RUST` block
    /// whose surrounding translation unit already does `import rusty;`.
    /// Suppresses emission of the `runtime_path_fallback_helpers_text()`
    /// preamble (`struct TokenTree; namespace rusty { ... }`): it is redundant
    /// (the imported rusty module provides those helpers) and — because an
    /// inline block is spliced into a consumer namespace (e.g. `namespace rrr`)
    /// — it would otherwise create a shadowing `<ns>::rusty` and break every
    /// emitted `rusty::*` reference (`rusty::detail::deref_if_pointer_like`,
    /// `rusty::Option`, ...). Defaults to `false` (module / standalone mode
    /// still emits the preamble).
    pub inline_rust_block: bool,
    /// Cross-file enum declarations collected during a crate-mode pre-pass.
    /// Used to seed the per-file codegen's `data_enum_variants_by_enum` /
    /// `c_like_enum_variants` registries so that bare-glob variant patterns
    /// (`use Foo::*; match { Variant(...) => ... }`) resolve when `Foo` is
    /// declared in a sibling file. Empty for single-file mode.
    pub cross_file_enums: Vec<syn::ItemEnum>,
    /// Cross-file impl blocks collected during a crate-mode pre-pass —
    /// every `Item::Impl` across the crate. Used by the per-file codegen
    /// to (a) inject forward declarations for cross-module orphan impl
    /// methods into the host struct's body when that struct is emitted,
    /// and (b) emit out-of-line member definitions instead of free-
    /// standing template functions when an orphan impl block is
    /// processed. Empty for single-file mode.
    pub cross_file_impl_blocks: Vec<syn::ItemImpl>,
    /// Cross-file struct declarations collected during a crate-mode
    /// pre-pass. Used to determine where each host type is declared so
    /// the orphan-impl emitter knows whether the host file will absorb
    /// the methods (and the orphan emission should therefore be
    /// suppressed). Empty for single-file mode.
    pub cross_file_structs: Vec<syn::ItemStruct>,
    /// Cross-file type-alias declarations (`pub type Foo<K> = Bar<...>;`)
    /// collected during a crate-mode pre-pass. Used to resolve orphan
    /// impl blocks targeting a type alias back to the underlying struct
    /// so the methods are absorbed into the struct's body and the
    /// orphan emission is suppressed. Empty for single-file mode.
    pub cross_file_type_aliases: Vec<syn::ItemType>,
    /// Every C++ module name produced by the current crate-mode run
    /// (e.g. `["btree_port.btree.node", "btree_port.btree.map", …]`).
    /// Used by `emit_use` to detect when a Rust `use super::sibling::*`
    /// path is referring to a sibling module that we ourselves are
    /// generating, in which case we must emit `import …;` instead of
    /// a global-namespace `using ::sibling::*;` (which fails name
    /// lookup because `::sibling` doesn't exist outside Rust's
    /// module tree). Empty in single-file mode; populated by main.rs
    /// before per-file transpilation begins.
    pub crate_module_names: Vec<String>,
    /// Optional C++ namespace to wrap all exported items in. When
    /// `Some("foo::bar")`, codegen emits `export namespace foo::bar { … }`
    /// around the module's items (in module mode); `None` keeps the
    /// legacy flat-export behavior. Used to disambiguate sibling
    /// modules that export same-named types — see rusty-std-book §2.10.
    pub cxx_namespace: Option<String>,
    /// When true, auto-derive `cxx_namespace` from the module name
    /// (replace `.` with `::`) AND emit namespace aliases for each
    /// imported sibling module so path-qualified emit shapes resolve
    /// to the sibling's namespace. Option 2 in rusty-std-book §2.10's
    /// fix matrix — the spec-correct rendering of Rust's module tree.
    pub auto_namespace: bool,
    /// True when transpiling a dependency crate (not the crate under test).
    /// The strict-auto `<auto>` backstop is skipped for dependency output:
    /// a leak in a *used* dependency surfaces at the C++ compile stage anyway,
    /// while leaks in dependencies that aren't compiled (e.g. an unused
    /// dev-dependency) are harmless — so a transpile-time panic there is a
    /// false failure. The backstop still fires for the crate under test.
    pub is_dependency: bool,
}

/// Classification of a method *name* across the whole crate, used by the UFCS
/// call-site lowering (book § 3.2.3) to pick the emission shape **without any
/// type inference**: a purely-inherent name stays native `x.m()`, a
/// purely-trait name becomes `m(x)`, and a name that is *both* (inherent on one
/// type, a trait method on another) needs the member-first UFCS shim.
//
// Phase 1 of the UFCS trait migration (book § 3.2): wired and tested here;
// consumed by the call-site lowering in a later phase, hence `allow(dead_code)`.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MethodNameClass {
    /// Only ever appears in inherent `impl Type { fn m }` blocks.
    Inherent,
    /// Only ever appears as a trait method (`trait Tr { fn m }` /
    /// `impl Tr for Type { fn m }`).
    TraitOnly,
    /// Appears as both an inherent method and a trait method somewhere.
    Both,
}

/// Walk every `impl`/`trait` in the crate and classify each method *name* as
/// inherent-only, trait-only, or both. Purely syntactic (no types): an `impl`
/// with a `for Tr` clause contributes a *trait* use; an `impl` without one, an
/// *inherent* use; a `trait` definition's methods (including defaults) are
/// trait uses. Recurses into inline modules.
#[allow(dead_code)]
pub fn classify_method_names(items: &[syn::Item]) -> HashMap<String, MethodNameClass> {
    // UFCS lowering applies ONLY to traits this crate DECLARES. Prelude/std
    // traits a crate merely *implements* (`Clone`, `Display`, `Debug`,
    // `PartialOrd`, `Iterator`, `Deref`, …) already have working dedicated
    // lowering on the non-UFCS path — that's why those crates compile with the
    // flag off. If we also lowered their method names (`clone`, `fmt`, `cmp`,
    // `len`, `as_ref`, …), we'd intercept calls on *std and rusty-library*
    // receivers that share the name but are not this crate's trait impls, and
    // neither the free-call branches nor the member fallback would resolve
    // (Phase-7 fallout category A). So `impl Tr for U` contributes a *trait*
    // use only when `Tr` is crate-declared; otherwise it contributes nothing
    // (the call stays whatever the non-UFCS path makes it).
    let declared_traits = collect_declared_trait_names(items);
    let mut inherent: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut trait_named: std::collections::HashSet<String> = std::collections::HashSet::new();
    collect_method_name_uses(items, &declared_traits, &mut inherent, &mut trait_named);

    let mut out = HashMap::new();
    for name in inherent.union(&trait_named) {
        let class = match (inherent.contains(name), trait_named.contains(name)) {
            (true, true) => MethodNameClass::Both,
            (true, false) => MethodNameClass::Inherent,
            (false, true) => MethodNameClass::TraitOnly,
            (false, false) => unreachable!("name came from the union of the two sets"),
        };
        out.insert(name.clone(), class);
    }
    out
}

/// Crates whose emitted module purview is wrapped in `namespace <crate> { … }`
/// (so a `class ser::Serialize` etc. doesn't ODR-collide with the same-named
/// namespace in an imported dependency — see
/// `wrap_module_purview_in_crate_namespace`). Post-wrap, references to the
/// crate's own items must be qualified to `::<crate>::…`. Currently narrow
/// (Phase-1): serde_bytes only; widening this list also widens the wrap and its
/// re-qualification in lockstep.
pub fn crate_is_namespace_wrapped(crate_name: &str) -> bool {
    // Applied SELECTIVELY to crates that actually collide (a shared module name with
    // an imported dependency). serde_bytes (Phase 1) and hashbrown (collides with
    // indexmap on `set`/`map`/`iter`). A flip-to-ALL was measured (2026-06-27) and
    // regresses 11/14 crates: the self-re-qualification needs per-pattern work it
    // does not yet do — re-qualifying a namespace SHARED with a dependency (serde's
    // `de` vs serde_core's `de`) the way Rule 1 deliberately avoids, declared
    // crate-root TYPES (either's `Either_Left`), and non-type-holding own modules
    // (bitflags's `external`). Widen one crate at a time, matrix-gated.
    matches!(crate_name, "serde_bytes" | "hashbrown" | "either" | "bitflags")
}

/// Short names of every trait this crate DECLARES (`trait Tr { … }`), recursing
/// into inline modules. Used to scope UFCS lowering + emission to crate-declared
/// traits (prelude/std-trait impls are left to the non-UFCS path).
pub fn collect_declared_trait_names(items: &[syn::Item]) -> std::collections::HashSet<String> {
    let mut out = std::collections::HashSet::new();
    collect_declared_trait_names_into(items, &mut out);
    out
}

fn collect_declared_trait_names_into(
    items: &[syn::Item],
    out: &mut std::collections::HashSet<String>,
) {
    for item in items {
        match item {
            syn::Item::Trait(t) => {
                out.insert(t.ident.to_string());
            }
            syn::Item::Mod(m) => {
                if let Some((_, nested)) = &m.content {
                    collect_declared_trait_names_into(nested, out);
                }
            }
            _ => {}
        }
    }
}

/// Trait name → the method names it DECLARES (required + default), across all
/// modules. Feeds the per-crate UFCS manifest so a downstream crate's dedup can
/// be METHOD-AWARE — a dependency declaring a same-NAMED but unrelated trait
/// (the ubiquitous private `Sealed`) must not suppress this crate's free
/// functions unless that dependency's trait actually provides the same method.
pub fn collect_declared_trait_methods(
    items: &[syn::Item],
) -> std::collections::BTreeMap<String, Vec<String>> {
    let mut out = std::collections::BTreeMap::new();
    collect_declared_trait_methods_into(items, &mut out);
    out
}

fn collect_declared_trait_methods_into(
    items: &[syn::Item],
    out: &mut std::collections::BTreeMap<String, Vec<String>>,
) {
    for item in items {
        match item {
            syn::Item::Trait(t) => {
                let entry = out.entry(t.ident.to_string()).or_insert_with(Vec::new);
                for ti in &t.items {
                    if let syn::TraitItem::Fn(f) = ti {
                        let name = f.sig.ident.to_string();
                        if !entry.contains(&name) {
                            entry.push(name);
                        }
                    }
                }
            }
            syn::Item::Mod(m) => {
                if let Some((_, nested)) = &m.content {
                    collect_declared_trait_methods_into(nested, out);
                }
            }
            _ => {}
        }
    }
}

/// Map each method name to the set of crate-declared traits that have a
/// CONCRETE (non-generic) `impl Tr for U` providing it — i.e. exactly the
/// methods for which `emit_ufcs_trait_impl_block_free_functions` emits (and
/// `…_decls` early-declares) a `<Tr>_::m` free function. Used to QUALIFY
/// the UFCS method-call shim to `<Tr>_::m(recv)` when exactly one trait
/// owns a name, so the unqualified `m(recv)` can't be shadowed by a local of
/// the same name (`let bits = x.bits();` → `auto bits = …bits(__self)…`).
///
/// DELIBERATELY excludes (a) default trait methods with no concrete impl —
/// those aren't emitted as free functions (`Flags_::is_empty` wouldn't
/// exist) — and (b) generic/blanket impls like `impl<T> IntoEither for T` whose
/// `<Tr>_` namespace isn't reliably available at the (earlier) call site.
/// For those, the unqualified shim + member fallback is kept (the prior, safe
/// behavior). Qualifying to a non-existent `<Tr>_::m` is a HARD error (not
/// SFINAE), so this set must contain only names that truly resolve.
pub fn collect_concrete_trait_impl_method_owners(
    items: &[syn::Item],
    declared_traits: &std::collections::HashSet<String>,
) -> HashMap<String, std::collections::BTreeSet<String>> {
    // Traits that declare an associated CONSTANT are emitted via the runtime-
    // helper path (`emit_trait_interface_pattern` skips them, `has_assoc_const`),
    // so their methods live in `<Tr>RuntimeHelper`, NOT `namespace <Tr>_`.
    // Qualifying to `<Tr>_::m` for those would name a non-existent member
    // (a HARD error). Exclude them — their method calls fall through to the
    // member-call lowering (which is what works flag-off). Surfaced by bitflags'
    // `Flags` trait (`const FLAGS`, `type Bits`): `complement`/`contains`/`bits`
    // are NOT in `Flags_`. (Assoc-TYPE-only traits like ToOwned DO use the
    // interface + free-function path, so they are NOT excluded.)
    let mut assoc_const_traits = std::collections::HashSet::new();
    collect_assoc_const_trait_names_into(items, &mut assoc_const_traits);
    let mut out: HashMap<String, std::collections::BTreeSet<String>> = HashMap::new();
    collect_concrete_trait_impl_method_owners_into(items, declared_traits, &assoc_const_traits, &mut out);
    out
}

fn collect_assoc_const_trait_names_into(
    items: &[syn::Item],
    out: &mut std::collections::HashSet<String>,
) {
    for item in items {
        match item {
            syn::Item::Trait(t) => {
                if t.items.iter().any(|ti| matches!(ti, syn::TraitItem::Const(_))) {
                    out.insert(t.ident.to_string());
                }
            }
            syn::Item::Mod(m) => {
                if let Some((_, nested)) = &m.content {
                    collect_assoc_const_trait_names_into(nested, out);
                }
            }
            _ => {}
        }
    }
}

fn collect_concrete_trait_impl_method_owners_into(
    items: &[syn::Item],
    declared_traits: &std::collections::HashSet<String>,
    assoc_const_traits: &std::collections::HashSet<String>,
    out: &mut HashMap<String, std::collections::BTreeSet<String>>,
) {
    for item in items {
        match item {
            syn::Item::Impl(impl_block) => {
                let Some((_, trait_path, _)) = &impl_block.trait_ else {
                    continue;
                };
                let Some(trait_name) =
                    trait_path.segments.last().map(|s| s.ident.to_string())
                else {
                    continue;
                };
                // Only crate-declared traits (foreign-trait impls aren't UFCS-
                // lowered), skip assoc-const (runtime-helper) traits, and only
                // concrete impls (no type-param generics) — generic/blanket
                // impls don't reliably emit an early-declared `<Tr>_`.
                if !declared_traits.contains(&trait_name)
                    || assoc_const_traits.contains(&trait_name)
                {
                    continue;
                }
                let has_type_generics = impl_block
                    .generics
                    .params
                    .iter()
                    .any(|p| matches!(p, syn::GenericParam::Type(_)));
                if has_type_generics {
                    continue;
                }
                for ii in &impl_block.items {
                    if let syn::ImplItem::Fn(method) = ii
                        && matches!(method.sig.inputs.first(), Some(syn::FnArg::Receiver(_)))
                    {
                        out.entry(method.sig.ident.to_string())
                            .or_default()
                            .insert(trait_name.clone());
                    }
                }
            }
            syn::Item::Trait(t) => {
                // Default-bodied trait methods (§ 3.2.13) are emitted as
                // `Self`-templated free functions in `<Tr>_`, so they own their
                // name too. Skip assoc-const (runtime-helper) traits, matching
                // the impl branch and the default-method emitter.
                let trait_name = t.ident.to_string();
                if !assoc_const_traits.contains(&trait_name) {
                    for ti in &t.items {
                        if let syn::TraitItem::Fn(m) = ti
                            && m.default.is_some()
                            && matches!(m.sig.inputs.first(), Some(syn::FnArg::Receiver(_)))
                        {
                            out.entry(m.sig.ident.to_string())
                                .or_default()
                                .insert(trait_name.clone());
                        }
                    }
                }
            }
            syn::Item::Mod(m) => {
                if let Some((_, nested)) = &m.content {
                    collect_concrete_trait_impl_method_owners_into(
                        nested,
                        declared_traits,
                        assoc_const_traits,
                        out,
                    );
                }
            }
            _ => {}
        }
    }
}

fn collect_method_name_uses(
    items: &[syn::Item],
    declared_traits: &std::collections::HashSet<String>,
    inherent: &mut std::collections::HashSet<String>,
    trait_named: &mut std::collections::HashSet<String>,
) {
    for item in items {
        match item {
            syn::Item::Impl(impl_block) => {
                // A trait impl counts as a *trait* use only when the implemented
                // trait is crate-declared (see `classify_method_names`).
                let impl_trait_name = impl_block.trait_.as_ref().and_then(|(_, path, _)| {
                    path.segments.last().map(|s| s.ident.to_string())
                });
                let is_crate_trait_impl = impl_trait_name
                    .as_ref()
                    .is_some_and(|n| declared_traits.contains(n));
                for impl_item in &impl_block.items {
                    if let syn::ImplItem::Fn(method) = impl_item {
                        let name = method.sig.ident.to_string();
                        if impl_block.trait_.is_some() {
                            // foreign/prelude-trait impls contribute nothing
                            if is_crate_trait_impl {
                                trait_named.insert(name);
                            }
                        } else {
                            inherent.insert(name);
                        }
                    }
                }
            }
            syn::Item::Trait(t) => {
                for trait_item in &t.items {
                    if let syn::TraitItem::Fn(method) = trait_item {
                        trait_named.insert(method.sig.ident.to_string());
                    }
                }
            }
            syn::Item::Mod(m) => {
                if let Some((_, nested)) = &m.content {
                    collect_method_name_uses(nested, declared_traits, inherent, trait_named);
                }
            }
            _ => {}
        }
    }
}

impl Default for TranspileOptions {
    fn default() -> Self {
        Self {
            by_value_cycle_breaking_prototype: false,
            is_dependency: false,
            cpp_module_symbol_index: None,
            cpp_module_symbol_index_sources: Vec::new(),
            external_crate_module_aliases: HashMap::new(),
            emit_ufcs_trait_manifest_path: None,
            dependency_ufcs_trait_manifests: Vec::new(),
            use_import_std_in_modules: false,
            // Default to the `rusty::Unit` alias spelling (replacing
            // `std::tuple<>` post-emission). The two C++ types are
            // identical via `using Unit = std::tuple<>;`, but the alias
            // reads cleaner in DSL-generated code and matches the
            // hand-written rusty-cpp surface. Set
            // `prefer_rusty_unit_alias: false` (or pass
            // `--prefer-std-tuple-alias` on the CLI) for the legacy
            // `std::tuple<>` spelling.
            prefer_rusty_unit_alias: true,
            prefer_rusty_view_aliases: false,
            interface_traits: false,
            inline_rust_block: false,
            cross_file_enums: Vec::new(),
            cross_file_impl_blocks: Vec::new(),
            cross_file_structs: Vec::new(),
            cross_file_type_aliases: Vec::new(),
            crate_module_names: Vec::new(),
            cxx_namespace: None,
            auto_namespace: false,
        }
    }
}

pub fn load_cpp_module_symbol_index_files(
    index_paths: &[PathBuf],
) -> Result<CppModuleSymbolIndex, String> {
    let mut merged = CppModuleSymbolIndex::default();
    for path in index_paths {
        let content = fs::read_to_string(path).map_err(|e| {
            format!(
                "Failed to read C++ module symbol index {}: {}",
                path.display(),
                e
            )
        })?;
        let file = parse_cpp_module_symbol_index_file(path, &content)?;
        merge_cpp_module_symbol_index_file(&mut merged, path, file)?;
    }
    Ok(merged)
}

fn parse_cpp_module_symbol_index_file(
    path: &Path,
    content: &str,
) -> Result<CppModuleSymbolIndexFile, String> {
    let ext = path
        .extension()
        .and_then(|s| s.to_str())
        .map(|s| s.to_ascii_lowercase());
    let parsed: CppModuleSymbolIndexFile = match ext.as_deref() {
        Some("json") => serde_json::from_str(content).map_err(|e| {
            format!(
                "Invalid JSON C++ module symbol index {}: {}",
                path.display(),
                e
            )
        })?,
        Some("toml") => toml::from_str(content).map_err(|e| {
            format!(
                "Invalid TOML C++ module symbol index {}: {}",
                path.display(),
                e
            )
        })?,
        _ => match serde_json::from_str(content) {
            Ok(v) => v,
            Err(json_err) => toml::from_str(content).map_err(|toml_err| {
                format!(
                    "Failed to parse C++ module symbol index {} as JSON ({}) or TOML ({})",
                    path.display(),
                    json_err,
                    toml_err
                )
            })?,
        },
    };

    if parsed.version != 1 {
        return Err(format!(
            "Unsupported C++ module symbol index version {} in {} (expected version 1)",
            parsed.version,
            path.display()
        ));
    }
    Ok(parsed)
}

fn merge_cpp_module_symbol_index_file(
    merged: &mut CppModuleSymbolIndex,
    source_path: &Path,
    file: CppModuleSymbolIndexFile,
) -> Result<(), String> {
    for (raw_module_path, module) in file.modules {
        let module_path = canonical_cpp_module_path(&raw_module_path);
        if module_path.is_empty() {
            return Err(format!(
                "C++ module symbol index {} contains an empty module path key",
                source_path.display()
            ));
        }

        let incoming = CppModuleIndexModule {
            namespace: module.namespace,
            symbols: module
                .symbols
                .into_iter()
                .map(|(name, symbol)| {
                    (
                        name,
                        CppModuleIndexSymbol {
                            kind: symbol.kind,
                            callable_signatures: symbol.callable_signatures,
                        },
                    )
                })
                .collect(),
        };

        if let Some(existing) = merged.modules.get_mut(&module_path) {
            merge_cpp_module_entry(existing, &incoming, source_path, &module_path)?;
        } else {
            merged.modules.insert(module_path, incoming);
        }
    }
    Ok(())
}

fn merge_cpp_module_entry(
    existing: &mut CppModuleIndexModule,
    incoming: &CppModuleIndexModule,
    source_path: &Path,
    module_path: &str,
) -> Result<(), String> {
    match (&existing.namespace, &incoming.namespace) {
        (Some(a), Some(b)) if a != b => {
            return Err(format!(
                "C++ module symbol index {} has conflicting namespace for module '{}': '{}' vs '{}'",
                source_path.display(),
                module_path,
                a,
                b
            ));
        }
        (None, Some(ns)) => {
            existing.namespace = Some(ns.clone());
        }
        _ => {}
    }

    for (symbol_name, symbol) in &incoming.symbols {
        if symbol_name.trim().is_empty() {
            return Err(format!(
                "C++ module symbol index {} has empty symbol name in module '{}'",
                source_path.display(),
                module_path
            ));
        }
        if let Some(existing_symbol) = existing.symbols.get(symbol_name) {
            if existing_symbol != symbol {
                return Err(format!(
                    "C++ module symbol index {} has conflicting definition for '{}::{}'",
                    source_path.display(),
                    module_path,
                    symbol_name
                ));
            }
        } else {
            existing.symbols.insert(symbol_name.clone(), symbol.clone());
        }
    }
    Ok(())
}

fn canonical_cpp_module_path(path: &str) -> String {
    path.trim().replace('.', "::")
}

fn cpp_symbol_kind_contains(symbol: &CppModuleIndexSymbol, needle: &str) -> bool {
    symbol
        .kind
        .as_deref()
        .is_some_and(|kind| kind.to_ascii_lowercase().contains(needle))
}

fn cpp_symbol_is_member_method(symbol: &CppModuleIndexSymbol) -> bool {
    cpp_symbol_kind_contains(symbol, "method")
}

fn collect_cpp_module_member_symbol_map(
    index: &CppModuleSymbolIndex,
) -> HashMap<String, HashSet<String>> {
    let mut by_module: HashMap<String, HashSet<String>> = HashMap::new();
    for (module_path, module_entry) in &index.modules {
        let mut member_symbols = HashSet::new();
        for (symbol_name, symbol) in &module_entry.symbols {
            if cpp_symbol_is_member_method(symbol) {
                member_symbols.insert(symbol_name.clone());
            }
        }
        if !member_symbols.is_empty() {
            by_module.insert(module_path.clone(), member_symbols);
        }
    }
    by_module
}

/// Transpile Rust source code to C++ code.
/// If `module_name` is provided, emit C++20 module declarations.
pub fn transpile(rust_source: &str, module_name: Option<&str>) -> Result<String, String> {
    transpile_with_type_map(rust_source, module_name, &UserTypeMap::default())
}

/// Transpile with user-provided type mappings for external crate types.
pub fn transpile_with_type_map(
    rust_source: &str,
    module_name: Option<&str>,
    type_map: &UserTypeMap,
) -> Result<String, String> {
    transpile_with_type_map_and_extension_hints_and_options(
        rust_source,
        module_name,
        type_map,
        &HashSet::new(),
        &TranspileOptions::default(),
    )
}

/// Transpile with user-provided type mappings plus cross-source extension-method hints.
pub fn transpile_with_type_map_and_extension_hints(
    rust_source: &str,
    module_name: Option<&str>,
    type_map: &UserTypeMap,
    extension_method_hints: &HashSet<String>,
) -> Result<String, String> {
    transpile_with_type_map_and_extension_hints_and_options(
        rust_source,
        module_name,
        type_map,
        extension_method_hints,
        &TranspileOptions::default(),
    )
}

/// Transpile with user-provided type mappings plus cross-source extension-method
/// hints and explicit transpilation options.
pub fn transpile_with_type_map_and_extension_hints_and_options(
    rust_source: &str,
    module_name: Option<&str>,
    type_map: &UserTypeMap,
    extension_method_hints: &HashSet<String>,
    options: &TranspileOptions,
) -> Result<String, String> {
    transpile_full_with_options(
        rust_source,
        module_name,
        type_map,
        extension_method_hints,
        None,
        options,
    )
}

/// Transpile with all options including crate name for path stripping.
pub fn transpile_full(
    rust_source: &str,
    module_name: Option<&str>,
    type_map: &UserTypeMap,
    extension_method_hints: &HashSet<String>,
    crate_name: Option<&str>,
) -> Result<String, String> {
    transpile_full_with_options(
        rust_source,
        module_name,
        type_map,
        extension_method_hints,
        crate_name,
        &TranspileOptions::default(),
    )
}

/// Transpile with all options including crate name for path stripping and
/// explicit transpilation options.
pub fn transpile_full_with_options(
    rust_source: &str,
    module_name: Option<&str>,
    type_map: &UserTypeMap,
    extension_method_hints: &HashSet<String>,
    crate_name: Option<&str>,
    options: &TranspileOptions,
) -> Result<String, String> {
    let profile_transpile = std::env::var_os("RUSTY_CPP_PROFILE_TRANSPILE").is_some();
    let profile_this_call = profile_transpile && rust_source.lines().take(2001).count() >= 2000;
    let profile_start = std::time::Instant::now();
    let module_label = module_name.unwrap_or("<none>");
    let crate_label = crate_name.unwrap_or("<none>");
    let log_profile = |label: &str| {
        if profile_this_call {
            eprintln!(
                "[rusty-cpp][transpile-full] module={} crate={} {}: {:.3}s",
                module_label,
                crate_label,
                label,
                profile_start.elapsed().as_secs_f64()
            );
        }
    };
    log_profile("start");
    let file: syn::File = parse_with_expand_hygiene_fallback(rust_source)
        .map_err(|e| format!("Parse error: {}", e))?;
    log_profile("parse_with_expand_hygiene_fallback");
    let has_cpp_module_imports = file_contains_cpp_module_imports(&file);
    log_profile("file_contains_cpp_module_imports");
    if has_cpp_module_imports {
        match options.cpp_module_symbol_index.as_ref() {
            Some(index) if !index.modules.is_empty() => {}
            Some(_) => {
                return Err(
                    "Found `use cpp::...` import, but configured C++ module symbol index is empty"
                        .to_string(),
                )
            }
            None => {
                return Err(
                    "Found `use cpp::...` import, but no C++ module symbol index is configured. Pass --cpp-module-index <path>"
                        .to_string(),
                )
            }
        }
    }
    log_profile("cpp_module_index_validation");
    if has_cpp_module_imports {
        if let Some(index) = options.cpp_module_symbol_index.as_ref() {
            let resolution_diagnostics = collect_cpp_foreign_call_resolution_diagnostics(
                &file,
                index,
                &options.cpp_module_symbol_index_sources,
            );
            if !resolution_diagnostics.is_empty() {
                return Err(format!(
                    "Unresolved or invalid `cpp::` symbol usage detected:\n- {}",
                    resolution_diagnostics.join("\n- ")
                ));
            }
        }
    }
    log_profile("cpp_foreign_call_resolution_diagnostics");
    let cpp_call_unsafe_violations = collect_cpp_foreign_call_unsafe_violations(&file);
    log_profile("collect_cpp_foreign_call_unsafe_violations");
    if !cpp_call_unsafe_violations.is_empty() {
        return Err(format!(
            "Foreign C++ calls imported through `cpp::` require `unsafe` context:\n- {}",
            cpp_call_unsafe_violations.join("\n- ")
        ));
    }

    let mut codegen = if extension_method_hints.is_empty() {
        CodeGen::with_type_map(type_map.clone())
    } else {
        CodeGen::with_type_map_and_extension_hints(type_map.clone(), extension_method_hints.clone())
    };
    if let Some(name) = crate_name {
        codegen.set_crate_name(name);
    }
    codegen.set_by_value_cycle_breaking_prototype(options.by_value_cycle_breaking_prototype);
    codegen.set_is_dependency_module(options.is_dependency);
    codegen.set_external_crate_module_aliases(options.external_crate_module_aliases.clone());
    codegen.set_use_import_std_in_modules(options.use_import_std_in_modules);
    codegen.set_cxx_namespace(options.cxx_namespace.clone());
    codegen.set_auto_namespace(options.auto_namespace);
    codegen.set_prefer_rusty_unit_alias(options.prefer_rusty_unit_alias);
    codegen.set_prefer_rusty_view_aliases(options.prefer_rusty_view_aliases);
    codegen.set_interface_traits(options.interface_traits);
    codegen.inline_rust_block = options.inline_rust_block;
    codegen.set_cross_file_enums(options.cross_file_enums.clone());
    codegen.set_cross_file_impl_blocks(options.cross_file_impl_blocks.clone());
    codegen.set_cross_file_structs(options.cross_file_structs.clone());
    codegen.set_cross_file_type_aliases(options.cross_file_type_aliases.clone());
    codegen.set_crate_module_names(options.crate_module_names.clone());
    if let Some(index) = options.cpp_module_symbol_index.as_ref() {
        let member_symbols = collect_cpp_module_member_symbol_map(index);
        codegen.set_cpp_module_member_symbols(member_symbols);
    }
    // UFCS cross-crate (book § 3.2.7): load dependency trait manifests so the
    // classifier + call-site qualification know the dependency's trait methods
    // and the module each lives in. Merged during emit_file.
    if !options.dependency_ufcs_trait_manifests.is_empty() {
        codegen.set_dependency_ufcs_trait_manifests(load_ufcs_trait_manifests(
            &options.dependency_ufcs_trait_manifests,
        ));
    }
    log_profile("codegen_setup");
    codegen.emit_file(&file, module_name);
    log_profile("codegen_emit_file");
    // UFCS cross-crate: emit this crate's trait manifest (declared traits +
    // actually-emitted `<Tr>_::m` owner map) for dependents to consume.
    if let Some(path) = options.emit_ufcs_trait_manifest_path.as_ref() {
        let manifest = codegen.build_ufcs_trait_manifest(module_name.unwrap_or(""));
        if let Ok(json) = serde_json::to_string_pretty(&manifest) {
            let _ = fs::write(path, json);
        }
    }
    let mut output_str = codegen.into_output();
    println!("RUSTY_DEDUP_TRACE_X9Z called len={}", output_str.len());
    // Generic dedup of consecutive identical `= default;` operator lines.
    // `#[derive(Eq, PartialEq, Ord, PartialOrd)]` lowers each pair (Eq +
    // PartialEq → operator==, Ord + PartialOrd → operator<=>) to the same
    // defaulted overload — C++ rejects two defaulted overloads with the
    // same signature. The per-struct dedup in `emit_struct` catches most
    // cases, but some emit paths leave duplicates. A textual dedup of
    // adjacent identical operator-default lines is always safe.
    output_str = {
        let mut out = String::with_capacity(output_str.len());
        let mut prev_trimmed: Option<String> = None;
        for line in output_str.split_inclusive('\n') {
            let trimmed = line.trim().to_string();
            let is_defaulted_declarator = trimmed.ends_with("= default;")
                && (trimmed.contains("operator==")
                    || trimmed.contains("operator<=>")
                    || trimmed.contains("operator<")
                    || trimmed.contains("operator>")
                    || trimmed.contains("operator!="));
            if is_defaulted_declarator
                && prev_trimmed.as_ref().is_some_and(|prev| prev == &trimmed)
            {
                continue;
            }
            out.push_str(line);
            prev_trimmed = Some(trimmed);
        }
        out
    };
    Ok(output_str)
}

fn parse_with_expand_hygiene_fallback(rust_source: &str) -> Result<syn::File, syn::Error> {
    match syn::parse_str::<syn::File>(rust_source) {
        Ok(file) => Ok(file),
        Err(primary_err) => {
            // rustc/cargo-expand output can contain hygiene-prefixed statement
            // forms such as `super let ...` that are not valid source syntax.
            // Normalize that artifact and retry parsing once.
            let normalized = rust_source.replace("super let ", "let ");
            if normalized == rust_source {
                return Err(primary_err);
            }
            syn::parse_str::<syn::File>(&normalized).map_err(|_| primary_err)
        }
    }
}

fn file_contains_cpp_module_imports(file: &syn::File) -> bool {
    file.items.iter().any(item_contains_cpp_module_import)
}

fn item_contains_cpp_module_import(item: &syn::Item) -> bool {
    match item {
        syn::Item::Use(use_item) => use_tree_contains_cpp_module_root(&use_item.tree, true),
        syn::Item::Mod(module) => module
            .content
            .as_ref()
            .is_some_and(|(_, items)| items.iter().any(item_contains_cpp_module_import)),
        _ => false,
    }
}

fn use_tree_contains_cpp_module_root(tree: &syn::UseTree, at_root: bool) -> bool {
    match tree {
        syn::UseTree::Path(path) => {
            if at_root && path.ident == "cpp" {
                return true;
            }
            use_tree_contains_cpp_module_root(&path.tree, false)
        }
        syn::UseTree::Group(group) => group
            .items
            .iter()
            .any(|item| use_tree_contains_cpp_module_root(item, at_root)),
        syn::UseTree::Name(_) | syn::UseTree::Rename(_) | syn::UseTree::Glob(_) => false,
    }
}

fn collect_cpp_foreign_call_unsafe_violations(file: &syn::File) -> Vec<String> {
    let mut visitor = CppForeignCallSafetyVisitor::default();
    visitor.visit_file(file);
    visitor.into_diagnostics()
}

#[derive(Default)]
struct CppForeignCallSafetyVisitor {
    cpp_binding_scopes: Vec<HashMap<String, String>>,
    unsafe_context_depth: usize,
    diagnostics: Vec<String>,
    diagnostic_keys: HashSet<String>,
    context_stack: Vec<String>,
}

impl CppForeignCallSafetyVisitor {
    fn push_cpp_binding_scope(&mut self, bindings: HashMap<String, String>) {
        self.cpp_binding_scopes.push(bindings);
    }

    fn pop_cpp_binding_scope(&mut self) {
        self.cpp_binding_scopes.pop();
    }

    fn lookup_cpp_binding(&self, binding: &str) -> Option<&str> {
        for scope in self.cpp_binding_scopes.iter().rev() {
            if let Some(module_path) = scope.get(binding) {
                return Some(module_path);
            }
        }
        None
    }

    fn current_context_label(&self) -> String {
        if self.context_stack.is_empty() {
            "<module>".to_string()
        } else {
            self.context_stack.join("::")
        }
    }

    fn record_safe_context_cpp_call_violation(
        &mut self,
        call: &syn::ExprCall,
        binding_name: &str,
        module_path: &str,
    ) {
        let call_site = call.to_token_stream().to_string();
        let context = self.current_context_label();
        let key = format!("{}|{}", context, call_site);
        if self.diagnostic_keys.insert(key) {
            self.diagnostics.push(format!(
                "safe-context foreign C++ call requires `unsafe`: `{}` (binding `{}` -> `{}`) in `{}`",
                call_site, binding_name, module_path, context
            ));
        }
    }

    fn check_cpp_call_requires_unsafe(&mut self, call: &syn::ExprCall) {
        if self.unsafe_context_depth > 0 {
            return;
        }
        let syn::Expr::Path(path_expr) = call.func.as_ref() else {
            return;
        };
        if path_expr.path.segments.len() < 2 {
            return;
        }
        let Some(first_segment) = path_expr.path.segments.first() else {
            return;
        };
        let binding_name = first_segment.ident.to_string();
        let Some(module_path) = self
            .lookup_cpp_binding(&binding_name)
            .map(ToOwned::to_owned)
        else {
            return;
        };
        self.record_safe_context_cpp_call_violation(call, &binding_name, &module_path);
    }

    fn into_diagnostics(mut self) -> Vec<String> {
        self.diagnostics.sort();
        self.diagnostics.dedup();
        self.diagnostics
    }
}

impl<'ast> Visit<'ast> for CppForeignCallSafetyVisitor {
    fn visit_file(&mut self, file: &'ast syn::File) {
        self.push_cpp_binding_scope(collect_cpp_bindings_from_items(&file.items));
        for item in &file.items {
            self.visit_item(item);
        }
        self.pop_cpp_binding_scope();
    }

    fn visit_item_mod(&mut self, module: &'ast syn::ItemMod) {
        let Some((_, items)) = &module.content else {
            return;
        };
        self.context_stack.push(module.ident.to_string());
        self.push_cpp_binding_scope(collect_cpp_bindings_from_items(items));
        for item in items {
            self.visit_item(item);
        }
        self.pop_cpp_binding_scope();
        self.context_stack.pop();
    }

    fn visit_item_fn(&mut self, function: &'ast syn::ItemFn) {
        self.context_stack.push(function.sig.ident.to_string());
        let was_unsafe = function.sig.unsafety.is_some();
        if was_unsafe {
            self.unsafe_context_depth += 1;
        }
        visit::visit_block(self, &function.block);
        if was_unsafe {
            self.unsafe_context_depth -= 1;
        }
        self.context_stack.pop();
    }

    fn visit_impl_item_fn(&mut self, method: &'ast syn::ImplItemFn) {
        self.context_stack.push(method.sig.ident.to_string());
        let was_unsafe = method.sig.unsafety.is_some();
        if was_unsafe {
            self.unsafe_context_depth += 1;
        }
        visit::visit_block(self, &method.block);
        if was_unsafe {
            self.unsafe_context_depth -= 1;
        }
        self.context_stack.pop();
    }

    fn visit_block(&mut self, block: &'ast syn::Block) {
        self.push_cpp_binding_scope(collect_cpp_bindings_from_stmts(&block.stmts));
        for stmt in &block.stmts {
            self.visit_stmt(stmt);
        }
        self.pop_cpp_binding_scope();
    }

    fn visit_expr_unsafe(&mut self, unsafe_expr: &'ast syn::ExprUnsafe) {
        self.unsafe_context_depth += 1;
        visit::visit_expr_unsafe(self, unsafe_expr);
        self.unsafe_context_depth -= 1;
    }

    fn visit_expr_call(&mut self, call: &'ast syn::ExprCall) {
        self.check_cpp_call_requires_unsafe(call);
        visit::visit_expr_call(self, call);
    }
}

fn collect_cpp_foreign_call_resolution_diagnostics(
    file: &syn::File,
    index: &CppModuleSymbolIndex,
    index_sources: &[PathBuf],
) -> Vec<String> {
    let mut visitor = CppForeignCallResolutionVisitor::new(index, index_sources);
    visitor.visit_file(file);
    visitor.into_diagnostics()
}

struct CppForeignCallResolutionVisitor<'a> {
    cpp_binding_scopes: Vec<HashMap<String, String>>,
    diagnostics: Vec<String>,
    diagnostic_keys: HashSet<String>,
    context_stack: Vec<String>,
    index: &'a CppModuleSymbolIndex,
    index_source_label: String,
}

impl<'a> CppForeignCallResolutionVisitor<'a> {
    fn new(index: &'a CppModuleSymbolIndex, index_sources: &[PathBuf]) -> Self {
        Self {
            cpp_binding_scopes: Vec::new(),
            diagnostics: Vec::new(),
            diagnostic_keys: HashSet::new(),
            context_stack: Vec::new(),
            index,
            index_source_label: format_cpp_module_index_sources(index_sources),
        }
    }

    fn push_cpp_binding_scope(&mut self, bindings: HashMap<String, String>) {
        self.cpp_binding_scopes.push(bindings);
    }

    fn pop_cpp_binding_scope(&mut self) {
        self.cpp_binding_scopes.pop();
    }

    fn lookup_cpp_binding(&self, binding: &str) -> Option<&str> {
        for scope in self.cpp_binding_scopes.iter().rev() {
            if let Some(module_path) = scope.get(binding) {
                return Some(module_path);
            }
        }
        None
    }

    fn current_context_label(&self) -> String {
        if self.context_stack.is_empty() {
            "<module>".to_string()
        } else {
            self.context_stack.join("::")
        }
    }

    fn record_diagnostic(
        &mut self,
        site: &str,
        module_path: &str,
        symbol_name: &str,
        detail: &str,
    ) {
        let context = self.current_context_label();
        let key = format!("{}|{}|{}|{}", context, module_path, symbol_name, detail);
        if self.diagnostic_keys.insert(key) {
            self.diagnostics.push(format!(
                "{} (module `{}`, symbol `{}`, index source `{}`, call `{}`, context `{}`)",
                detail, module_path, symbol_name, self.index_source_label, site, context
            ));
        }
    }

    fn resolve_cpp_symbol_for_path(&self, path: &syn::Path) -> Option<(String, String)> {
        if path.segments.len() < 2 {
            return None;
        }
        let first_segment = path.segments.first()?;
        let binding_name = first_segment.ident.to_string();
        let module_path = self.lookup_cpp_binding(&binding_name)?.to_string();
        let symbol_name = path
            .segments
            .iter()
            .skip(1)
            .map(|seg| seg.ident.to_string())
            .collect::<Vec<String>>()
            .join("::");
        if symbol_name.is_empty() {
            return None;
        }
        Some((module_path, symbol_name))
    }

    fn lookup_index_symbol<'b>(
        &self,
        module: &'b CppModuleIndexModule,
        symbol_name: &str,
    ) -> Option<&'b CppModuleIndexSymbol> {
        module.symbols.get(symbol_name).or_else(|| {
            symbol_name
                .rsplit("::")
                .next()
                .and_then(|tail| module.symbols.get(tail))
        })
    }

    fn symbol_kind_contains(symbol: &CppModuleIndexSymbol, needle: &str) -> bool {
        symbol
            .kind
            .as_deref()
            .is_some_and(|kind| kind.to_ascii_lowercase().contains(needle))
    }

    fn symbol_is_macro(symbol: &CppModuleIndexSymbol) -> bool {
        Self::symbol_kind_contains(symbol, "macro")
    }

    fn symbol_is_template(symbol: &CppModuleIndexSymbol) -> bool {
        Self::symbol_kind_contains(symbol, "template")
    }

    fn symbol_is_member_method(symbol: &CppModuleIndexSymbol) -> bool {
        Self::symbol_kind_contains(symbol, "method")
    }

    fn symbol_is_callable_kind(symbol: &CppModuleIndexSymbol) -> bool {
        Self::symbol_kind_contains(symbol, "function")
            || Self::symbol_kind_contains(symbol, "method")
            || Self::symbol_kind_contains(symbol, "callable")
            || Self::symbol_kind_contains(symbol, "ctor")
            || Self::symbol_kind_contains(symbol, "constructor")
    }

    fn validate_cpp_module_symbol_access(
        &mut self,
        site: &str,
        module_path: &str,
        symbol_name: &str,
    ) -> Option<CppModuleIndexSymbol> {
        let Some(module) = self.index.modules.get(module_path) else {
            self.record_diagnostic(
                site,
                module_path,
                symbol_name,
                "module path is not present in configured C++ module symbol index",
            );
            return None;
        };
        let Some(symbol) = self.lookup_index_symbol(module, symbol_name) else {
            self.record_diagnostic(
                site,
                module_path,
                symbol_name,
                "symbol is not present in configured C++ module symbol index module entry",
            );
            return None;
        };
        Some(symbol.clone())
    }

    fn validate_cpp_call_symbol(&mut self, call: &syn::ExprCall) {
        let syn::Expr::Path(path_expr) = call.func.as_ref() else {
            return;
        };
        let Some((module_path, symbol_name)) = self.resolve_cpp_symbol_for_path(&path_expr.path)
        else {
            return;
        };
        let call_site = call.to_token_stream().to_string();

        let Some(symbol) =
            self.validate_cpp_module_symbol_access(&call_site, &module_path, &symbol_name)
        else {
            return;
        };
        if Self::symbol_is_macro(&symbol) {
            self.record_diagnostic(
                &call_site,
                &module_path,
                &symbol_name,
                "TODO(leaf22.7): `cpp::` macro exports are unsupported in MVP",
            );
            return;
        }

        let call_arity = call.args.len();
        let member_style_arity = (path_expr.path.segments.len() > 2
            && call_arity > 0
            && Self::symbol_is_member_method(&symbol))
        .then_some(call_arity - 1);
        if Self::symbol_is_template(&symbol) && symbol.callable_signatures.is_empty() {
            self.record_diagnostic(
                &call_site,
                &module_path,
                &symbol_name,
                "TODO(leaf22.7): template-only export without indexed callable signatures is unsupported in MVP",
            );
            return;
        }
        if symbol.callable_signatures.is_empty() {
            self.record_diagnostic(
                &call_site,
                &module_path,
                &symbol_name,
                "call cannot be matched to indexed callable family (no callable signatures indexed)",
            );
            return;
        }

        let mut has_arity_match = false;
        for signature in &symbol.callable_signatures {
            if parse_callable_signature_arity(signature).is_some_and(|arity| {
                arity == call_arity || member_style_arity.is_some_and(|adjusted| arity == adjusted)
            }) {
                has_arity_match = true;
                break;
            }
        }
        if !has_arity_match {
            let arity_label = if let Some(adjusted) = member_style_arity {
                format!("{} (receiver-adjusted: {})", call_arity, adjusted)
            } else {
                call_arity.to_string()
            };
            self.record_diagnostic(
                &call_site,
                &module_path,
                &symbol_name,
                &format!(
                    "call cannot be matched to indexed callable family (arity {} does not match signatures [{}])",
                    arity_label,
                    symbol.callable_signatures.join(", ")
                ),
            );
        }
    }

    fn validate_cpp_value_symbol(&mut self, path_expr: &syn::ExprPath) {
        let Some((module_path, symbol_name)) = self.resolve_cpp_symbol_for_path(&path_expr.path)
        else {
            return;
        };
        let path_site = path_expr.to_token_stream().to_string();
        if path_expr.path.segments.len() > 2 {
            self.record_diagnostic(
                &path_site,
                &module_path,
                &symbol_name,
                "TODO(leaf22.7): member-function import syntax is unsupported for `cpp::` MVP (only module constants are supported in non-call positions)",
            );
            return;
        }
        let Some(symbol) =
            self.validate_cpp_module_symbol_access(&path_site, &module_path, &symbol_name)
        else {
            return;
        };

        if Self::symbol_is_macro(&symbol) {
            self.record_diagnostic(
                &path_site,
                &module_path,
                &symbol_name,
                "TODO(leaf22.7): `cpp::` macro exports are unsupported in MVP",
            );
            return;
        }

        if Self::symbol_is_template(&symbol) && symbol.callable_signatures.is_empty() {
            self.record_diagnostic(
                &path_site,
                &module_path,
                &symbol_name,
                "TODO(leaf22.7): template-only export without indexed callable signatures is unsupported in MVP",
            );
            return;
        }

        if Self::symbol_is_callable_kind(&symbol) || !symbol.callable_signatures.is_empty() {
            self.record_diagnostic(
                &path_site,
                &module_path,
                &symbol_name,
                "TODO(leaf22.7): non-call function symbol usage is unsupported for `cpp::` MVP (only module constants are supported in value position)",
            );
        }
    }

    fn validate_cpp_macro_symbol_with_site(&mut self, path: &syn::Path, site: &str) {
        let Some((module_path, symbol_name)) = self.resolve_cpp_symbol_for_path(path) else {
            return;
        };
        self.record_diagnostic(
            site,
            &module_path,
            &symbol_name,
            "TODO(leaf22.7): `cpp::` macro imports are unsupported in MVP",
        );
    }

    fn into_diagnostics(mut self) -> Vec<String> {
        self.diagnostics.sort();
        self.diagnostics.dedup();
        self.diagnostics
    }
}

impl<'ast> Visit<'ast> for CppForeignCallResolutionVisitor<'_> {
    fn visit_file(&mut self, file: &'ast syn::File) {
        self.push_cpp_binding_scope(collect_cpp_bindings_from_items(&file.items));
        for item in &file.items {
            self.visit_item(item);
        }
        self.pop_cpp_binding_scope();
    }

    fn visit_item_mod(&mut self, module: &'ast syn::ItemMod) {
        let Some((_, items)) = &module.content else {
            return;
        };
        self.context_stack.push(module.ident.to_string());
        self.push_cpp_binding_scope(collect_cpp_bindings_from_items(items));
        for item in items {
            self.visit_item(item);
        }
        self.pop_cpp_binding_scope();
        self.context_stack.pop();
    }

    fn visit_item_fn(&mut self, function: &'ast syn::ItemFn) {
        self.context_stack.push(function.sig.ident.to_string());
        visit::visit_block(self, &function.block);
        self.context_stack.pop();
    }

    fn visit_impl_item_fn(&mut self, method: &'ast syn::ImplItemFn) {
        self.context_stack.push(method.sig.ident.to_string());
        visit::visit_block(self, &method.block);
        self.context_stack.pop();
    }

    fn visit_block(&mut self, block: &'ast syn::Block) {
        self.push_cpp_binding_scope(collect_cpp_bindings_from_stmts(&block.stmts));
        for stmt in &block.stmts {
            self.visit_stmt(stmt);
        }
        self.pop_cpp_binding_scope();
    }

    fn visit_expr_call(&mut self, call: &'ast syn::ExprCall) {
        self.validate_cpp_call_symbol(call);
        let cpp_bound_call_path = match call.func.as_ref() {
            syn::Expr::Path(path_expr) => {
                self.resolve_cpp_symbol_for_path(&path_expr.path).is_some()
            }
            _ => false,
        };
        if !cpp_bound_call_path {
            self.visit_expr(&call.func);
        }
        for arg in &call.args {
            self.visit_expr(arg);
        }
    }

    fn visit_expr_path(&mut self, path_expr: &'ast syn::ExprPath) {
        self.validate_cpp_value_symbol(path_expr);
        visit::visit_expr_path(self, path_expr);
    }

    fn visit_expr_macro(&mut self, expr_macro: &'ast syn::ExprMacro) {
        let site = expr_macro.to_token_stream().to_string();
        self.validate_cpp_macro_symbol_with_site(&expr_macro.mac.path, &site);
        visit::visit_expr_macro(self, expr_macro);
    }

    fn visit_stmt_macro(&mut self, stmt_macro: &'ast syn::StmtMacro) {
        let site = stmt_macro.mac.to_token_stream().to_string();
        self.validate_cpp_macro_symbol_with_site(&stmt_macro.mac.path, &site);
        visit::visit_stmt_macro(self, stmt_macro);
    }
}

fn format_cpp_module_index_sources(index_sources: &[PathBuf]) -> String {
    if index_sources.is_empty() {
        "<unknown>".to_string()
    } else {
        index_sources
            .iter()
            .map(|path| path.display().to_string())
            .collect::<Vec<String>>()
            .join(", ")
    }
}

fn parse_callable_signature_arity(signature: &str) -> Option<usize> {
    let start = signature.find('(')?;
    let end = signature.rfind(')')?;
    if end < start {
        return None;
    }
    let args = signature[start + 1..end].trim();
    if args.is_empty() {
        return Some(0);
    }

    let mut arity = 1usize;
    let mut paren_depth = 0usize;
    let mut angle_depth = 0usize;
    let mut bracket_depth = 0usize;
    let mut brace_depth = 0usize;
    for ch in args.chars() {
        match ch {
            '(' => paren_depth += 1,
            ')' => paren_depth = paren_depth.saturating_sub(1),
            '<' => angle_depth += 1,
            '>' => angle_depth = angle_depth.saturating_sub(1),
            '[' => bracket_depth += 1,
            ']' => bracket_depth = bracket_depth.saturating_sub(1),
            '{' => brace_depth += 1,
            '}' => brace_depth = brace_depth.saturating_sub(1),
            ',' if paren_depth == 0
                && angle_depth == 0
                && bracket_depth == 0
                && brace_depth == 0 =>
            {
                arity += 1;
            }
            _ => {}
        }
    }
    Some(arity)
}

fn collect_cpp_bindings_from_items(items: &[syn::Item]) -> HashMap<String, String> {
    let mut bindings = HashMap::new();
    for item in items {
        if let syn::Item::Use(use_item) = item {
            collect_cpp_bindings_from_use_tree(&use_item.tree, true, false, "", &mut bindings);
        }
    }
    bindings
}

fn collect_cpp_bindings_from_stmts(stmts: &[syn::Stmt]) -> HashMap<String, String> {
    let mut bindings = HashMap::new();
    for stmt in stmts {
        if let syn::Stmt::Item(syn::Item::Use(use_item)) = stmt {
            collect_cpp_bindings_from_use_tree(&use_item.tree, true, false, "", &mut bindings);
        }
    }
    bindings
}

fn collect_cpp_bindings_from_use_tree(
    tree: &syn::UseTree,
    at_root: bool,
    in_cpp_root: bool,
    prefix: &str,
    out: &mut HashMap<String, String>,
) {
    match tree {
        syn::UseTree::Path(path) => {
            if in_cpp_root {
                let new_prefix = join_cpp_module_prefix(prefix, &path.ident.to_string());
                collect_cpp_bindings_from_use_tree(&path.tree, false, true, &new_prefix, out);
            } else if at_root && path.ident == "cpp" {
                collect_cpp_bindings_from_use_tree(&path.tree, false, true, "", out);
            } else {
                collect_cpp_bindings_from_use_tree(&path.tree, false, false, prefix, out);
            }
        }
        syn::UseTree::Name(name) => {
            if !in_cpp_root {
                return;
            }
            if name.ident == "self" {
                if let Some(binding) = cpp_module_tail_segment(prefix) {
                    record_cpp_binding(out, binding.to_string(), prefix.to_string());
                }
                return;
            }
            let ident = name.ident.to_string();
            let module_path = join_cpp_module_prefix(prefix, &ident);
            record_cpp_binding(out, ident, module_path);
        }
        syn::UseTree::Rename(rename) => {
            if !in_cpp_root {
                return;
            }
            let target = if rename.ident == "self" {
                prefix.to_string()
            } else {
                join_cpp_module_prefix(prefix, &rename.ident.to_string())
            };
            if target.is_empty() {
                return;
            }
            record_cpp_binding(out, rename.rename.to_string(), target);
        }
        syn::UseTree::Group(group) => {
            for item in &group.items {
                collect_cpp_bindings_from_use_tree(item, at_root, in_cpp_root, prefix, out);
            }
        }
        syn::UseTree::Glob(_) => {}
    }
}

fn join_cpp_module_prefix(prefix: &str, segment: &str) -> String {
    if prefix.is_empty() {
        segment.to_string()
    } else {
        format!("{}::{}", prefix, segment)
    }
}

fn cpp_module_tail_segment(path: &str) -> Option<&str> {
    path.rsplit("::").find(|segment| !segment.is_empty())
}

fn record_cpp_binding(out: &mut HashMap<String, String>, binding: String, module_path: String) {
    if binding.is_empty() || module_path.is_empty() {
        return;
    }
    let canonical = canonical_cpp_module_path(&module_path);
    out.entry(binding).or_insert(canonical);
}

/// Collect extension-method names from a Rust source unit.
/// A method is treated as extension-shaped when it appears in a trait impl
/// targeting a non-local type in that same source unit.
/// Walk a Rust source file and collect every top-level / nested `Item::Enum`
/// declaration. The result is intended to be threaded across files in
/// crate-mode transpilation so each per-file codegen can seed its
/// data-enum / c-like-enum variant tracking from sibling-file enums.
/// Without this seeding, bare-glob variant patterns
/// (`use Foo::*; match { Variant(x) => ... }`) silently miscompile when
/// `Foo` is declared in another file.
pub fn collect_crate_enum_decls(rust_source: &str) -> Vec<syn::ItemEnum> {
    let Ok(file) = syn::parse_str::<syn::File>(rust_source) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    collect_enum_decls_recursive(&file.items, &mut out);
    out
}

fn collect_enum_decls_recursive(items: &[syn::Item], out: &mut Vec<syn::ItemEnum>) {
    for item in items {
        match item {
            syn::Item::Enum(e) => out.push(e.clone()),
            syn::Item::Mod(m) => {
                if let Some((_, nested)) = &m.content {
                    collect_enum_decls_recursive(nested, out);
                }
            }
            _ => {}
        }
    }
}

/// Walk a Rust source file and collect every top-level / nested `Item::Impl`
/// block. The result is intended to be threaded across files in crate-mode
/// transpilation so the per-file codegen can detect when an impl block's
/// host type lives in a different file (a cross-module orphan impl) and
/// emit out-of-line member definitions plus inject the matching forward
/// declarations into the host struct's body.
pub fn collect_crate_impl_blocks(rust_source: &str) -> Vec<syn::ItemImpl> {
    let Ok(file) = syn::parse_str::<syn::File>(rust_source) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    collect_impl_blocks_recursive(&file.items, &mut out);
    out
}

fn collect_impl_blocks_recursive(items: &[syn::Item], out: &mut Vec<syn::ItemImpl>) {
    for item in items {
        match item {
            syn::Item::Impl(i) => out.push(i.clone()),
            syn::Item::Mod(m) => {
                if let Some((_, nested)) = &m.content {
                    collect_impl_blocks_recursive(nested, out);
                }
            }
            _ => {}
        }
    }
}

/// Walk a Rust source file and collect every `Item::Struct`. Cross-file
/// counterpart of `collect_crate_enum_decls`.
pub fn collect_crate_struct_decls(rust_source: &str) -> Vec<syn::ItemStruct> {
    let Ok(file) = syn::parse_str::<syn::File>(rust_source) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    collect_struct_decls_recursive(&file.items, &mut out);
    out
}

fn collect_struct_decls_recursive(items: &[syn::Item], out: &mut Vec<syn::ItemStruct>) {
    for item in items {
        match item {
            syn::Item::Struct(s) => out.push(s.clone()),
            syn::Item::Mod(m) => {
                if let Some((_, nested)) = &m.content {
                    collect_struct_decls_recursive(nested, out);
                }
            }
            _ => {}
        }
    }
}

/// Walk a Rust source file and collect every `Item::Type` (type alias).
/// Cross-file counterpart of `collect_crate_struct_decls`.
pub fn collect_crate_type_aliases(rust_source: &str) -> Vec<syn::ItemType> {
    let Ok(file) = syn::parse_str::<syn::File>(rust_source) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    collect_type_aliases_recursive(&file.items, &mut out);
    out
}

fn collect_type_aliases_recursive(items: &[syn::Item], out: &mut Vec<syn::ItemType>) {
    for item in items {
        match item {
            syn::Item::Type(t) => out.push(t.clone()),
            syn::Item::Mod(m) => {
                if let Some((_, nested)) = &m.content {
                    collect_type_aliases_recursive(nested, out);
                }
            }
            _ => {}
        }
    }
}

pub fn collect_extension_method_hints(rust_source: &str) -> HashSet<String> {
    let Ok(file) = syn::parse_str::<syn::File>(rust_source) else {
        return HashSet::new();
    };

    let mut local_types = HashSet::new();
    collect_local_declared_types(&file.items, &[], &mut local_types);

    let mut methods = HashSet::new();
    collect_extension_method_names(&file.items, &[], &local_types, &mut methods);
    methods
}

fn collect_local_declared_types(
    items: &[syn::Item],
    module_path: &[String],
    out: &mut HashSet<String>,
) {
    for item in items {
        match item {
            syn::Item::Struct(s) => record_local_type(module_path, &s.ident.to_string(), out),
            syn::Item::Enum(e) => record_local_type(module_path, &e.ident.to_string(), out),
            syn::Item::Type(t) => record_local_type(module_path, &t.ident.to_string(), out),
            syn::Item::Mod(m) => {
                if let Some((_, nested)) = &m.content {
                    let mut nested_path = module_path.to_vec();
                    nested_path.push(m.ident.to_string());
                    collect_local_declared_types(nested, &nested_path, out);
                }
            }
            _ => {}
        }
    }
}

fn record_local_type(module_path: &[String], type_name: &str, out: &mut HashSet<String>) {
    out.insert(type_name.to_string());
    if !module_path.is_empty() {
        out.insert(format!("{}::{}", module_path.join("::"), type_name));
    }
}

fn collect_extension_method_names(
    items: &[syn::Item],
    module_path: &[String],
    local_types: &HashSet<String>,
    out: &mut HashSet<String>,
) {
    for item in items {
        match item {
            syn::Item::Impl(impl_block) => {
                if impl_block.trait_.is_none() {
                    continue;
                }
                let Some(tp) = (match impl_block.self_ty.as_ref() {
                    syn::Type::Path(tp) => Some(tp),
                    _ => None,
                }) else {
                    continue;
                };

                let raw_self_name = tp
                    .path
                    .segments
                    .iter()
                    .map(|s| s.ident.to_string())
                    .collect::<Vec<_>>()
                    .join("::");
                let scoped_self_name = qualify_relative_path(&raw_self_name, module_path);
                if local_types.contains(&raw_self_name) || local_types.contains(&scoped_self_name) {
                    continue;
                }

                for impl_item in &impl_block.items {
                    if let syn::ImplItem::Fn(method) = impl_item {
                        out.insert(method.sig.ident.to_string());
                    }
                }
            }
            syn::Item::Mod(m) => {
                if let Some((_, nested)) = &m.content {
                    let mut nested_path = module_path.to_vec();
                    nested_path.push(m.ident.to_string());
                    collect_extension_method_names(nested, &nested_path, local_types, out);
                }
            }
            _ => {}
        }
    }
}

fn qualify_relative_path(raw: &str, module_path: &[String]) -> String {
    let parts: Vec<&str> = raw.split("::").collect();
    if parts.is_empty() {
        return raw.to_string();
    }
    if parts.len() == 1 {
        if module_path.is_empty() {
            return raw.to_string();
        }
        return format!("{}::{}", module_path.join("::"), raw);
    }

    let mut resolved_prefix = module_path.to_vec();
    let mut idx = 0usize;
    let mut had_relative_prefix = false;
    while idx < parts.len() {
        match parts[idx] {
            "self" => {
                had_relative_prefix = true;
                idx += 1;
            }
            "super" => {
                had_relative_prefix = true;
                if !resolved_prefix.is_empty() {
                    resolved_prefix.pop();
                }
                idx += 1;
            }
            "crate" => {
                had_relative_prefix = true;
                resolved_prefix.clear();
                idx += 1;
            }
            _ => break,
        }
    }

    if !had_relative_prefix {
        return raw.to_string();
    }

    let mut out_parts = resolved_prefix;
    out_parts.extend(parts[idx..].iter().map(|s| s.to_string()));
    if out_parts.is_empty() {
        raw.to_string()
    } else {
        out_parts.join("::")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use tempfile::tempdir;

    #[test]
    fn test_transpile_basic() {
        let result = transpile("fn main() { let x = 42; }", None);
        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output.contains("void main()"));
        assert!(output.contains("const auto x = 42;"));
    }

    #[test]
    fn test_transpile_error() {
        let result = transpile("fn {{{ invalid", None);
        assert!(result.is_err());
    }

    #[test]
    fn test_transpile_parses_cargo_expand_super_let_hygiene_artifact() {
        let result = transpile(
            r#"
            fn f(v: i32) -> i32 {
                let out = {
                    super let mut inner = v;
                    inner += 1;
                    inner
                };
                out
            }
            "#,
            None,
        );
        assert!(result.is_ok(), "{result:?}");
        let output = result.unwrap();
        assert!(output.contains("int32_t f"));
    }

    #[test]
    fn test_transpile_multiple_items() {
        let result = transpile(
            r#"
            struct Point { x: f64, y: f64 }
            const PI: f64 = 3.14159;
            fn distance(a: &Point, b: &Point) -> f64 {
                0.0
            }
        "#,
            None,
        );
        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output.contains("struct Point"));
        assert!(output.contains("constexpr double PI"));
        assert!(output.contains("double distance"));
    }

    #[test]
    fn test_transpile_complete_program() {
        let result = transpile(
            r#"
            fn add(a: i32, b: i32) -> i32 {
                a + b
            }

            fn main() {
                let result = add(1, 2);
            }
        "#,
            None,
        );
        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output.contains("int32_t add(int32_t a, int32_t b)"));
        // Operands may be bare or wrapped via `rusty::detail::deref_if_pointer_like`.
        assert!(
            output.contains("return a + b;")
                || output.contains(
                    "return rusty::detail::deref_if_pointer_like(a) + rusty::detail::deref_if_pointer_like(b);"
                ),
            "{output}"
        );
        assert!(output.contains("void main()"));
        // Call site may be unqualified `add(...)` or globally anchored
        // `::add(...)`, and integer literals may be wrapped in static_cast.
        assert!(
            output.contains("add(1, 2)")
                || output.contains("::add(1, 2)")
                || output
                    .contains("::add(static_cast<int32_t>(1), static_cast<int32_t>(2))")
                || output
                    .contains("add(static_cast<int32_t>(1), static_cast<int32_t>(2))"),
            "{output}"
        );
    }

    #[test]
    fn test_transpile_with_module() {
        let result = transpile("pub fn hello() {}", Some("my_crate"));
        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output.contains("export module my_crate;"));
        assert!(output.contains("export void hello()"));
    }

    #[test]
    fn test_transpile_without_module() {
        let result = transpile("pub fn hello() {}", None);
        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(!output.contains("export module"));
        // Without module mode, pub is ignored
    }

    #[test]
    fn test_collect_extension_method_hints_detects_non_local_impl_methods() {
        let src = r#"
            struct Local;
            trait TapOps { fn tap(self) -> Self; }
            impl TapOps for Local { fn tap(self) -> Self { self } }
            trait TapOptionOps<T> { fn tap_none<F>(self, f: F) -> Self; }
            impl<T> TapOptionOps<T> for Option<T> { fn tap_none<F>(self, f: F) -> Self { self } }
        "#;
        let hints = collect_extension_method_hints(src);
        assert!(hints.contains("tap_none"));
        assert!(!hints.contains("tap"));
    }

    #[test]
    fn test_transpile_with_extension_hints_rewrites_method_calls() {
        let mut hints = HashSet::new();
        hints.insert("tap".to_string());
        let result = transpile_with_type_map_and_extension_hints(
            "fn f() { let _ = 10.tap(); }",
            None,
            &UserTypeMap::default(),
            &hints,
        );
        assert!(result.is_ok());
        let output = result.unwrap();
        // The call may be a direct `rusty_ext::tap(10)` or wrapped in an
        // autoderef-fallback IIFE that calls `rusty_ext::tap(...)` on the
        // forwarded receiver (legitimate codegen evolution to handle
        // pointer-like receivers uniformly).
        assert!(
            output.contains("static_cast<void>(rusty_ext::tap(10));")
                || (output.contains("static_cast<void>")
                    && output.contains("rusty_ext::tap(")
                    && output.contains("})(10)")),
            "{output}"
        );
    }

    #[test]
    fn test_transpile_with_runtime_extension_hints_keeps_rusty_namespace() {
        let mut hints = HashSet::new();
        hints.insert("size_hint".to_string());
        let result = transpile_with_type_map_and_extension_hints(
            "fn f(iter: std::ops::Range<i32>) { let _ = iter.size_hint(); }",
            None,
            &UserTypeMap::default(),
            &hints,
        );
        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output.contains("rusty::size_hint(iter)"));
    }

    #[test]
    fn test_transpile_with_external_tap_err_hint_routes_to_rusty_ext() {
        let mut hints = HashSet::new();
        hints.insert("tap_err".to_string());
        let result = transpile_with_type_map_and_extension_hints(
            r#"
            fn f(result: Result<i32, i32>) {
                let _ = result.tap_err(|e| {
                    let _ = *e;
                });
            }
            "#,
            None,
            &UserTypeMap::default(),
            &hints,
        );
        assert!(result.is_ok());
        let output = result.unwrap();
        // The call may be a direct `rusty_ext::tap_err(result, ...)` or
        // wrapped in an autoderef-fallback IIFE that forwards `result` as
        // the first argument inside the lambda.
        assert!(
            output.contains("rusty_ext::tap_err(result,")
                || (output.contains("rusty_ext::tap_err(")
                    && output.contains("})(result")),
            "{output}"
        );
        assert!(!output.contains("rusty::tap_err("));
    }

    #[test]
    fn test_transpile_options_toggle_by_value_cycle_breaking_prototype_diagnostics() {
        let src = r#"
            struct A {
                b: B,
            }

            struct B {
                a: A,
            }
        "#;
        let default_out = transpile(src, None).expect("default transpile should succeed");
        assert!(
            !default_out.contains("// PROTOTYPE: by-value cycle-breaking flag enabled"),
            "default mode should not emit prototype cycle-breaking diagnostics\nGot: {default_out}"
        );

        let options = TranspileOptions {
            by_value_cycle_breaking_prototype: true,
            ..TranspileOptions::default()
        };
        let opt_in_out = transpile_full_with_options(
            src,
            None,
            &UserTypeMap::default(),
            &HashSet::new(),
            None,
            &options,
        )
        .expect("opt-in transpile should succeed");
        assert!(
            opt_in_out.contains("// PROTOTYPE: by-value cycle-breaking flag enabled"),
            "opt-in mode should emit prototype cycle-breaking diagnostics\nGot: {opt_in_out}"
        );
    }

    #[test]
    fn test_ufcs_traits_phase2_emits_trait_namespace_free_functions() {
        let src = r#"
            struct Foo { x: i32 }
            trait Greet {
                fn hello(&self) -> i32;
            }
            impl Greet for Foo {
                fn hello(&self) -> i32 { self.x }
            }
        "#;

        // `impl Greet for Foo` is emitted as a free
        // function in `namespace Greet_`, with `self` rewritten to `self_`.
        let options = TranspileOptions {
            ..TranspileOptions::default()
        };
        let on = transpile_full_with_options(
            src,
            None,
            &UserTypeMap::default(),
            &HashSet::new(),
            None,
            &options,
        )
        .expect("ufcs transpile should succeed");
        assert!(
            on.contains("namespace Greet_"),
            "flag-on output must emit the UFCS trait namespace\nGot: {on}"
        );
        assert!(
            on.contains("hello(") && on.contains("self_"),
            "flag-on output must emit the `hello` free function taking a self_ param\nGot: {on}"
        );
    }

    #[test]
    fn test_ufcs_traits_phase3_lowers_trait_call_to_free_dispatch() {
        let src = r#"
            struct Foo { x: i32 }
            trait Greet { fn hello(&self) -> i32; }
            impl Greet for Foo { fn hello(&self) -> i32 { self.x } }
            fn use_it(f: &Foo) -> i32 { f.hello() }
        "#;

        // `f.hello()` (a trait-only crate method) lowers to the
        // free-function dispatch form `... requires { hello(__self) } ...`.
        let options = TranspileOptions {
            ..TranspileOptions::default()
        };
        let on = transpile_full_with_options(
            src,
            None,
            &UserTypeMap::default(),
            &HashSet::new(),
            None,
            &options,
        )
        .expect("ufcs transpile should succeed");
        assert!(
            on.contains("requires { Greet_::hello("),
            "flag-on must lower the trait call `f.hello()` to free dispatch \
             (qualified, since exactly one trait owns `hello`)\nGot: {on}"
        );
    }

    #[test]
    fn test_ufcs_traits_runtime_helper_method_not_intercepted_by_ufcs() {
        // Regression (bitflags): `write_hex` is a TraitOnly crate method, but it
        // also has a hand-written `rusty::write_hex` runtime helper with a
        // forwarding-reference writer param. The UFCS per-type free function
        // takes the writer *by value* (faithful to Rust `mut writer: W`), so a
        // move-only lvalue argument (`rusty::String`) can't bind → the dispatch
        // `requires` fails and falls back to a member call on a primitive
        // receiver, a hard error. Flag-on must keep routing these names to the
        // runtime helper, identical to flag-off.
        let src = r#"
            trait WriteHex {
                fn write_hex<W: std::fmt::Write>(&self, writer: W) -> std::fmt::Result;
            }
            impl WriteHex for u8 {
                fn write_hex<W: std::fmt::Write>(&self, writer: W) -> std::fmt::Result { Ok(()) }
            }
            fn to_writer(value: u8, mut out: String) {
                let _ = value.write_hex(out);
            }
        "#;

        let options = TranspileOptions {
            ..TranspileOptions::default()
        };
        let on = transpile_full_with_options(
            src,
            None,
            &UserTypeMap::default(),
            &HashSet::new(),
            None,
            &options,
        )
        .expect("ufcs transpile should succeed");
        assert!(
            on.contains("rusty::write_hex("),
            "flag-on must route `write_hex` to the runtime helper, not UFCS\nGot: {on}"
        );
        assert!(
            !on.contains("WriteHex_::write_hex("),
            "flag-on must NOT intercept `write_hex` with the UFCS trait shim\nGot: {on}"
        );
    }

    #[test]
    fn test_ufcs_traits_phase4_emits_early_using_before_call_site() {
        let src = r#"
            struct Foo { x: i32 }
            trait Greet { fn hello(&self) -> i32; }
            impl Greet for Foo { fn hello(&self) -> i32 { self.x } }
            fn use_it(f: &Foo) -> i32 { f.hello() }
        "#;
        let options = TranspileOptions {
            ..TranspileOptions::default()
        };
        let on = transpile_full_with_options(
            src,
            None,
            &UserTypeMap::default(),
            &HashSet::new(),
            None,
            &options,
        )
        .expect("ufcs transpile should succeed");

        // Phase 4: a `using namespace Greet_;` is emitted so the call
        // site's unqualified `hello(__self)` resolves to the trait free
        // function, and it must appear BEFORE the call site (`use_it`) so
        // ordinary lookup at the body sees it.
        let using_pos = on
            .find("using namespace Greet_;")
            .expect("must emit `using namespace Greet_;`");
        // Anchor on the call-site dispatch (uniquely in the function body),
        // not `use_it`'s forward declaration (which precedes the using). The
        // call is qualified (`Greet_::hello`) since one trait owns `hello`.
        let call_pos = on
            .find("requires { Greet_::hello(")
            .expect("must emit the trait-call dispatch in use_it");
        assert!(
            using_pos < call_pos,
            "the trait `using` must precede the call site\nGot: {on}"
        );
    }

    #[test]
    fn test_ufcs_traits_phase5_associated_types_resolve() {
        // Associated types are handled by the existing `<Trait>Traits<U>` map
        // (orthogonal to dispatch), so they resolve in the UFCS static path:
        //  - concrete `Self::Output` in the free function → the bound type,
        //  - generic `T::Output` → `ProducerTraits<T>::Output`.
        let src = r#"
            struct Foo { x: i32 }
            trait Producer { type Output; fn produce(&self) -> Self::Output; }
            impl Producer for Foo { type Output = i32; fn produce(&self) -> Self::Output { self.x } }
            fn use_generic<T: Producer>(t: &T) -> T::Output { t.produce() }
        "#;
        let options = TranspileOptions {
            ..TranspileOptions::default()
        };
        let on = transpile_full_with_options(
            src,
            None,
            &UserTypeMap::default(),
            &HashSet::new(),
            None,
            &options,
        )
        .expect("ufcs transpile should succeed");

        // Concrete associated return type resolved in the trait free function.
        assert!(
            on.contains("int32_t produce(const Foo& self_)"),
            "concrete `Self::Output` must resolve to int32_t in the free function\nGot: {on}"
        );
        // Generic associated type routed through the `<Trait>Traits<T>` map.
        assert!(
            on.contains("ProducerTraits<T>::Output"),
            "generic `T::Output` must route through ProducerTraits<T>::Output\nGot: {on}"
        );
    }

    #[test]
    fn test_ufcs_traits_phase6_call_shim_has_dyn_member_fallback() {
        // Phase 6 (book § 3.2.10): a `dyn Tr` receiver derefs to the abstract
        // interface `Tr&`, for which there is NO `m(const Tr&)` free function.
        // So under the flag the call-site shim gains a final MEMBER fallback
        // `deref(__self).m()` (which for a dyn receiver hits the virtual
        // member → adapter override → the static `<Tr>_::m` impl, so
        // static and dynamic dispatch bottom out in the same implementation).
        let src = r#"
            struct Foo { x: i32 }
            trait Greet { fn hello(&self) -> i32; }
            impl Greet for Foo { fn hello(&self) -> i32 { self.x } }
            fn use_it(f: &Foo) -> i32 { f.hello() }
        "#;
        let options = TranspileOptions {
            ..TranspileOptions::default()
        };
        let on = transpile_full_with_options(
            src,
            None,
            &UserTypeMap::default(),
            &HashSet::new(),
            None,
            &options,
        )
        .expect("ufcs transpile should succeed");

        // The shim is now 3-branch: a final `else` that calls the member
        // `.hello()` on the dereferenced receiver (the dyn dispatch route).
        assert!(
            on.contains(".hello(); }")
                || on.contains(".hello() ; }")
                || on.contains(").hello();"),
            "flag-on shim must end in a member-call fallback `deref(__self).hello()`\nGot: {on}"
        );
        // And it must be reached only after the two free-function branches:
        // both qualified `requires { Greet_::hello(` guards still present
        // (`hello` is owned by exactly one trait, so the free call is qualified).
        let guard_count = on.matches("requires { Greet_::hello(").count();
        assert!(
            guard_count >= 2,
            "flag-on shim must keep both free-call guards before the member fallback (got {guard_count})\nGot: {on}"
        );

    }

    #[test]
    fn test_ufcs_traits_phase7_qualified_call_disambiguates_two_traits() {
        // Two crate-declared traits share the method name `name`, and `Person`
        // implements both. A disambiguated Rust call `Greet::name(p)` /
        // `Farewell::name(p)` / `<Person as Greet>::name(p)` must lower to the
        // QUALIFIED free function `<Trait>_::name(p)` — not the member
        // `p.name()` (which collapses to whichever impl won the struct's single
        // member slot, silently picking the wrong body).
        let src = r#"
            struct Person { id: i32 }
            trait Greet { fn name(&self) -> i32; }
            trait Farewell { fn name(&self) -> i32; }
            impl Greet for Person { fn name(&self) -> i32 { self.id } }
            impl Farewell for Person { fn name(&self) -> i32 { self.id + 100 } }
            fn via_greet(p: &Person) -> i32 { Greet::name(p) }
            fn via_farewell(p: &Person) -> i32 { Farewell::name(p) }
            fn via_qualified(p: &Person) -> i32 { <Person as Greet>::name(p) }
        "#;
        let options = TranspileOptions {
            ..TranspileOptions::default()
        };
        let on = transpile_full_with_options(
            src,
            None,
            &UserTypeMap::default(),
            &HashSet::new(),
            None,
            &options,
        )
        .expect("ufcs transpile should succeed");

        // The by-value trait-static call now routes through the member-fallback
        // shim (so a foreign trait's member-only impl still resolves), but the
        // PRIMARY `requires { Greet_::name(__self) }` branch still qualifies to
        // the free function for a crate-declared trait with a concrete impl —
        // disambiguation is preserved (the `.name()` member branch is guarded
        // and never taken here). Assert the qualified free call appears for both
        // traits rather than the exact pre-shim `Greet_::name(p)` spelling.
        assert!(
            on.contains("Greet_::name("),
            "`Greet::name(p)` and `<Person as Greet>::name(p)` must qualify to Greet_::name\nGot: {on}"
        );
        assert!(
            on.contains("Farewell_::name("),
            "`Farewell::name(p)` must qualify to Farewell_::name (not collapse to p.name())\nGot: {on}"
        );
    }

    #[test]
    fn test_ufcs_traits_phase7_method_shim_qualified_avoids_local_shadow() {
        // Rust `let bits = x.bits();` binds a local named the same as the trait
        // method. The method-call shim must qualify its free call to
        // `Bits_::bits(__self)` — an unqualified `bits(__self)` would bind
        // to the half-declared local `bits` ("variable 'bits' ... cannot appear
        // in its own initializer"). Qualification applies because exactly one
        // crate-declared trait (`Bits`) owns the name.
        let src = r#"
            struct Flags { v: u32 }
            trait Bits { fn bits(&self) -> u32; }
            impl Bits for Flags { fn bits(&self) -> u32 { self.v } }
            fn read(x: &Flags) -> u32 { let bits = x.bits(); bits }
        "#;
        let options = TranspileOptions {
            ..TranspileOptions::default()
        };
        let on = transpile_full_with_options(
            src,
            None,
            &UserTypeMap::default(),
            &HashSet::new(),
            None,
            &options,
        )
        .expect("ufcs transpile should succeed");
        assert!(
            on.contains("Bits_::bits("),
            "single-owner trait method must qualify its shim free call to Bits_::bits\nGot: {on}"
        );
        assert!(
            !on.contains("requires { bits("),
            "the shim must NOT emit an unqualified `bits(` that shadows the local\nGot: {on}"
        );
    }

    #[test]
    fn test_ufcs_traits_default_method_emits_self_templated_free_function() {
        // § 3.2.13: a default-bodied trait method is emitted ONCE as a
        // `Self`-templated free function in `<Tr>_` (param named `Self_`, since
        // `Self` can't be a template-param name); an overriding impl emits a
        // non-template overload that wins by C++ overload resolution.
        let src = r#"
            struct Foo { id: i32 }
            struct Bar { id: i32 }
            trait Greet {
                fn hello(&self) -> i32;
                fn describe(&self) -> i32 { self.hello() + 1 }
            }
            impl Greet for Foo { fn hello(&self) -> i32 { self.id } }
            impl Greet for Bar { fn hello(&self) -> i32 { self.id } fn describe(&self) -> i32 { 999 } }
            fn d_foo(f: &Foo) -> i32 { f.describe() }
        "#;
        let options = TranspileOptions {
            ..TranspileOptions::default()
        };
        let on = transpile_full_with_options(
            src,
            None,
            &UserTypeMap::default(),
            &HashSet::new(),
            None,
            &options,
        )
        .expect("ufcs transpile should succeed");

        // The default is one Self-templated free function in Greet_.
        assert!(
            on.contains("int32_t describe(const Self_& self_)"),
            "default `describe` must emit a Self-templated free function\nGot: {on}"
        );
        // Bar overrides it → a non-template concrete overload (which wins).
        assert!(
            on.contains("int32_t describe(const Bar& self_)"),
            "the Bar override must emit a concrete (non-template) describe overload\nGot: {on}"
        );
        // The default's body lowers `self.hello()` recursively via UFCS.
        assert!(
            on.contains("Greet_::hello("),
            "default body must lower `self.hello()` to the qualified trait call\nGot: {on}"
        );
        // The call site qualifies to Greet_::describe (default is in the owner map).
        assert!(
            on.contains("Greet_::describe("),
            "`f.describe()` must qualify to Greet_::describe\nGot: {on}"
        );

    }

    #[test]
    fn test_ufcs_cross_crate_emits_trait_manifest() {
        // § 3.2.7: transpiling a crate with `emit_ufcs_trait_manifest_path` set
        // writes a manifest recording its module, declared traits, and the
        // actually-emitted `<Tr>_::m` owner map.
        let src = r#"
            struct Foo { id: i32 }
            trait Greet { fn hello(&self) -> i32; }
            impl Greet for Foo { fn hello(&self) -> i32 { self.id } }
        "#;
        let path = std::env::temp_dir().join("rusty_ufcs_manifest_emit_test.json");
        let _ = std::fs::remove_file(&path);
        let options = TranspileOptions {
            emit_ufcs_trait_manifest_path: Some(path.clone()),
            ..TranspileOptions::default()
        };
        let _ = transpile_full_with_options(
            src,
            Some("depmod"),
            &UserTypeMap::default(),
            &HashSet::new(),
            None,
            &options,
        )
        .expect("ufcs transpile should succeed");

        let text = std::fs::read_to_string(&path).expect("manifest must be written");
        let manifest: UfcsTraitManifest =
            serde_json::from_str(&text).expect("manifest must parse");
        let _ = std::fs::remove_file(&path);
        assert_eq!(manifest.module, "depmod");
        assert!(
            manifest.declared_traits.contains(&"Greet".to_string()),
            "manifest must list declared trait Greet\nGot: {manifest:?}"
        );
        assert_eq!(
            manifest.method_owners.get("hello").map(|v| v.as_slice()),
            Some(["Greet".to_string()].as_slice()),
            "manifest must record hello → Greet (the emitted owner)\nGot: {manifest:?}"
        );
    }

    #[test]
    fn test_ufcs_cross_crate_consumes_manifest_and_classifies() {
        // § 3.2.7: a dependent crate loads a dependency's manifest and lowers a
        // call to the dependency's trait method to the UFCS free call `<Tr>_::m`
        // — even though it never sees the dependency's trait declaration. The
        // call is BARE (not `<module>::<Tr>_`): the transpiler emits each crate
        // at global scope inside its C++ module and resolves cross-crate via
        // `import`, so the dependency's `<Tr>_` is reached bare. The manifest's
        // job is CLASSIFICATION (member-call → UFCS free call).
        let manifest = UfcsTraitManifest {
            version: 1,
            module: "depmod".to_string(),
            declared_traits: vec!["Greet".to_string()],
            declared_trait_methods: std::collections::BTreeMap::from([(
                "Greet".to_string(),
                vec!["hello".to_string()],
            )]),
            method_owners: std::collections::BTreeMap::from([(
                "hello".to_string(),
                vec!["Greet".to_string()],
            )]),
            declared_types: Vec::new(),
        };
        let path = std::env::temp_dir().join("rusty_ufcs_manifest_consume_test.json");
        std::fs::write(&path, serde_json::to_string(&manifest).unwrap()).unwrap();

        // Target calls `x.hello()` on a local type with no local Greet trait.
        let src = r#"
            struct Local { id: i32 }
            fn use_it(x: &Local) -> i32 { x.hello() }
        "#;
        let options = TranspileOptions {
            dependency_ufcs_trait_manifests: vec![path.clone()],
            ..TranspileOptions::default()
        };
        let on = transpile_full_with_options(
            src,
            Some("target"),
            &UserTypeMap::default(),
            &HashSet::new(),
            None,
            &options,
        )
        .expect("ufcs transpile should succeed");
        let _ = std::fs::remove_file(&path);

        assert!(
            on.contains("Greet_::hello("),
            "`x.hello()` must lower to the UFCS free call Greet_::hello (from the manifest)\nGot: {on}"
        );

        // Without the manifest, `hello` isn't a known trait method → not lowered
        // to a UFCS free call (stays a plain member call).
        let off_opts = TranspileOptions {
            ..TranspileOptions::default()
        };
        let without = transpile_full_with_options(
            src,
            Some("target"),
            &UserTypeMap::default(),
            &HashSet::new(),
            None,
            &off_opts,
        )
        .expect("transpile should succeed");
        assert!(
            !without.contains("Greet_::hello"),
            "without the manifest there must be no UFCS free call for hello\nGot: {without}"
        );
    }

    #[test]
    fn test_transpile_options_prefer_rusty_view_aliases() {
        let src = r#"
            fn keep_views(s: &str, b: &[u8]) -> (&str, &[u8]) {
                (s, b)
            }
        "#;

        let default_out = transpile(src, None).expect("default transpile should succeed");
        assert!(
            default_out.contains("std::string_view") || default_out.contains("std::span<"),
            "default output should use std view spellings\nGot: {default_out}"
        );

        let options = TranspileOptions {
            prefer_rusty_view_aliases: true,
            ..TranspileOptions::default()
        };
        let alias_out = transpile_full_with_options(
            src,
            None,
            &UserTypeMap::default(),
            &HashSet::new(),
            None,
            &options,
        )
        .expect("alias mode transpile should succeed");

        assert!(
            alias_out.contains("rusty::StrView"),
            "alias mode should use rusty::StrView\nGot: {alias_out}"
        );
        assert!(
            alias_out.contains("rusty::Span<"),
            "alias mode should use rusty::Span\nGot: {alias_out}"
        );
        assert!(
            !alias_out.contains("std::string_view"),
            "alias mode should remove std::string_view spellings\nGot: {alias_out}"
        );
    }

    #[test]
    fn test_load_cpp_module_symbol_index_json() {
        let dir = tempdir().expect("tempdir");
        let index_path = dir.path().join("cpp_index.json");
        std::fs::write(
            &index_path,
            r#"
{
  "version": 1,
  "modules": {
    "std": {
      "namespace": "std",
      "symbols": {
        "max": {
          "kind": "function",
          "callable_signatures": ["int(int,int)"]
        }
      }
    }
  }
}
"#,
        )
        .expect("write json index");

        let index = load_cpp_module_symbol_index_files(&[index_path]).expect("load json index");
        let std_module = index.modules.get("std").expect("std module");
        assert_eq!(std_module.namespace.as_deref(), Some("std"));
        let max = std_module.symbols.get("max").expect("max symbol");
        assert_eq!(max.kind.as_deref(), Some("function"));
        assert_eq!(max.callable_signatures, vec!["int(int,int)".to_string()]);
    }

    #[test]
    fn test_load_cpp_module_symbol_index_toml() {
        let dir = tempdir().expect("tempdir");
        let index_path = dir.path().join("cpp_index.toml");
        std::fs::write(
            &index_path,
            r#"
version = 1

[modules.std]
namespace = "std"

[modules.std.symbols.max]
kind = "function"
callable_signatures = ["int(int,int)"]
"#,
        )
        .expect("write toml index");

        let index = load_cpp_module_symbol_index_files(&[index_path]).expect("load toml index");
        let std_module = index.modules.get("std").expect("std module");
        assert_eq!(std_module.namespace.as_deref(), Some("std"));
        let max = std_module.symbols.get("max").expect("max symbol");
        assert_eq!(max.kind.as_deref(), Some("function"));
        assert_eq!(max.callable_signatures, vec!["int(int,int)".to_string()]);
    }

    #[test]
    fn test_cpp_module_import_requires_symbol_index() {
        let err = transpile("use cpp::std as cpp_std;\nfn f() {}", None)
            .expect_err("cpp import without index should fail");
        assert!(err.contains("no C++ module symbol index is configured"));
        assert!(err.contains("--cpp-module-index"));
    }

    #[test]
    fn test_cpp_module_import_with_symbol_index_is_allowed() {
        let mut modules = BTreeMap::new();
        modules.insert(
            "std".to_string(),
            CppModuleIndexModule {
                namespace: Some("std".to_string()),
                symbols: BTreeMap::new(),
            },
        );
        let options = TranspileOptions {
            cpp_module_symbol_index: Some(CppModuleSymbolIndex { modules }),
            ..TranspileOptions::default()
        };

        let output = transpile_full_with_options(
            "use cpp::std as cpp_std;\nfn f() {}",
            None,
            &UserTypeMap::default(),
            &HashSet::new(),
            None,
            &options,
        )
        .expect("cpp import with index should transpile");
        assert!(output.contains("// C++ module import (reserved cpp::): std as cpp_std"));
    }

    #[test]
    fn test_cpp_module_foreign_call_requires_unsafe_context() {
        let mut modules = BTreeMap::new();
        let mut symbols = BTreeMap::new();
        symbols.insert(
            "max".to_string(),
            CppModuleIndexSymbol {
                kind: Some("function".to_string()),
                callable_signatures: vec!["int(int,int)".to_string()],
            },
        );
        modules.insert(
            "std".to_string(),
            CppModuleIndexModule {
                namespace: Some("std".to_string()),
                symbols,
            },
        );
        let options = TranspileOptions {
            cpp_module_symbol_index: Some(CppModuleSymbolIndex { modules }),
            ..TranspileOptions::default()
        };

        let err = transpile_full_with_options(
            r#"
use cpp::std as cpp_std;
fn max2(lo: i32, hi: i32) -> i32 {
    cpp_std::max(lo, hi)
}
"#,
            None,
            &UserTypeMap::default(),
            &HashSet::new(),
            None,
            &options,
        )
        .expect_err("safe-context foreign C++ call should fail");

        assert!(err.contains("require `unsafe` context"));
        assert!(err.contains("cpp_std"));
        assert!(err.contains("max2"));
    }

    #[test]
    fn test_cpp_module_foreign_call_in_unsafe_context_is_allowed() {
        let mut modules = BTreeMap::new();
        let mut symbols = BTreeMap::new();
        symbols.insert(
            "max".to_string(),
            CppModuleIndexSymbol {
                kind: Some("function".to_string()),
                callable_signatures: vec!["int(int,int)".to_string()],
            },
        );
        modules.insert(
            "std".to_string(),
            CppModuleIndexModule {
                namespace: Some("std".to_string()),
                symbols,
            },
        );
        let options = TranspileOptions {
            cpp_module_symbol_index: Some(CppModuleSymbolIndex { modules }),
            ..TranspileOptions::default()
        };

        let output = transpile_full_with_options(
            r#"
use cpp::std as cpp_std;
fn max2(lo: i32, hi: i32) -> i32 {
    unsafe { cpp_std::max(lo, hi) }
}
"#,
            None,
            &UserTypeMap::default(),
            &HashSet::new(),
            None,
            &options,
        )
        .expect("unsafe-context foreign C++ call should transpile");

        assert!(output.contains("// @unsafe"));
        assert!(output.contains("std::max("));
    }

    #[test]
    fn test_cpp_module_call_errors_when_module_path_missing_from_index() {
        let mut modules = BTreeMap::new();
        modules.insert(
            "std".to_string(),
            CppModuleIndexModule {
                namespace: Some("std".to_string()),
                symbols: BTreeMap::new(),
            },
        );
        let options = TranspileOptions {
            cpp_module_symbol_index: Some(CppModuleSymbolIndex { modules }),
            cpp_module_symbol_index_sources: vec![PathBuf::from("/tmp/cpp-index.toml")],
            ..TranspileOptions::default()
        };

        let err = transpile_full_with_options(
            r#"
use cpp::alpha::beta;
fn f(v: i32) -> i32 {
    unsafe { beta::transform(v) }
}
"#,
            None,
            &UserTypeMap::default(),
            &HashSet::new(),
            None,
            &options,
        )
        .expect_err("missing cpp module path should fail");

        assert!(err.contains("module path is not present"));
        assert!(err.contains("module `alpha::beta`"));
        assert!(err.contains("symbol `transform`"));
        assert!(err.contains("/tmp/cpp-index.toml"));
    }

    #[test]
    fn test_cpp_module_call_errors_when_symbol_missing_from_index_module() {
        let mut modules = BTreeMap::new();
        let mut symbols = BTreeMap::new();
        symbols.insert(
            "max".to_string(),
            CppModuleIndexSymbol {
                kind: Some("function".to_string()),
                callable_signatures: vec!["int(int,int)".to_string()],
            },
        );
        modules.insert(
            "std".to_string(),
            CppModuleIndexModule {
                namespace: Some("std".to_string()),
                symbols,
            },
        );
        let options = TranspileOptions {
            cpp_module_symbol_index: Some(CppModuleSymbolIndex { modules }),
            cpp_module_symbol_index_sources: vec![PathBuf::from("/tmp/cpp-index.toml")],
            ..TranspileOptions::default()
        };

        let err = transpile_full_with_options(
            r#"
use cpp::std as cpp_std;
fn f() -> i32 {
    unsafe { cpp_std::min(1, 2) }
}
"#,
            None,
            &UserTypeMap::default(),
            &HashSet::new(),
            None,
            &options,
        )
        .expect_err("missing indexed symbol should fail");

        assert!(err.contains("symbol is not present"));
        assert!(err.contains("module `std`"));
        assert!(err.contains("symbol `min`"));
        assert!(err.contains("/tmp/cpp-index.toml"));
    }

    #[test]
    fn test_cpp_module_call_errors_when_signature_family_does_not_match_call_shape() {
        let mut modules = BTreeMap::new();
        let mut symbols = BTreeMap::new();
        symbols.insert(
            "max".to_string(),
            CppModuleIndexSymbol {
                kind: Some("function".to_string()),
                callable_signatures: vec!["int(int,int)".to_string()],
            },
        );
        modules.insert(
            "std".to_string(),
            CppModuleIndexModule {
                namespace: Some("std".to_string()),
                symbols,
            },
        );
        let options = TranspileOptions {
            cpp_module_symbol_index: Some(CppModuleSymbolIndex { modules }),
            cpp_module_symbol_index_sources: vec![PathBuf::from("/tmp/cpp-index.toml")],
            ..TranspileOptions::default()
        };

        let err = transpile_full_with_options(
            r#"
use cpp::std as cpp_std;
fn f() -> i32 {
    unsafe { cpp_std::max(1) }
}
"#,
            None,
            &UserTypeMap::default(),
            &HashSet::new(),
            None,
            &options,
        )
        .expect_err("call arity mismatch should fail");

        assert!(err.contains("call cannot be matched to indexed callable family"));
        assert!(err.contains("arity 1"));
        assert!(err.contains("int(int,int)"));
        assert!(err.contains("/tmp/cpp-index.toml"));
    }

    #[test]
    fn test_cpp_module_constant_value_access_is_allowed() {
        let mut modules = BTreeMap::new();
        let mut symbols = BTreeMap::new();
        symbols.insert(
            "ANSWER".to_string(),
            CppModuleIndexSymbol {
                kind: Some("constant".to_string()),
                callable_signatures: Vec::new(),
            },
        );
        modules.insert(
            "std".to_string(),
            CppModuleIndexModule {
                namespace: Some("std".to_string()),
                symbols,
            },
        );
        let options = TranspileOptions {
            cpp_module_symbol_index: Some(CppModuleSymbolIndex { modules }),
            cpp_module_symbol_index_sources: vec![PathBuf::from("/tmp/cpp-index.toml")],
            ..TranspileOptions::default()
        };

        let output = transpile_full_with_options(
            r#"
use cpp::std as cpp_std;
fn f() -> i32 {
    cpp_std::ANSWER
}
"#,
            None,
            &UserTypeMap::default(),
            &HashSet::new(),
            None,
            &options,
        )
        .expect("module-constant access should transpile");

        assert!(output.contains("std::ANSWER"));
    }

    #[test]
    fn test_cpp_module_constant_access_errors_when_symbol_missing_from_index_module() {
        let mut modules = BTreeMap::new();
        let mut symbols = BTreeMap::new();
        symbols.insert(
            "max".to_string(),
            CppModuleIndexSymbol {
                kind: Some("function".to_string()),
                callable_signatures: vec!["int(int,int)".to_string()],
            },
        );
        modules.insert(
            "std".to_string(),
            CppModuleIndexModule {
                namespace: Some("std".to_string()),
                symbols,
            },
        );
        let options = TranspileOptions {
            cpp_module_symbol_index: Some(CppModuleSymbolIndex { modules }),
            cpp_module_symbol_index_sources: vec![PathBuf::from("/tmp/cpp-index.toml")],
            ..TranspileOptions::default()
        };

        let err = transpile_full_with_options(
            r#"
use cpp::std as cpp_std;
fn f() -> i32 {
    cpp_std::ANSWER
}
"#,
            None,
            &UserTypeMap::default(),
            &HashSet::new(),
            None,
            &options,
        )
        .expect_err("missing module constant should fail");

        assert!(err.contains("symbol is not present"));
        assert!(err.contains("symbol `ANSWER`"));
        assert!(err.contains("/tmp/cpp-index.toml"));
    }

    #[test]
    fn test_cpp_module_call_member_function_import_syntax_is_allowed() {
        let mut modules = BTreeMap::new();
        let mut symbols = BTreeMap::new();
        symbols.insert(
            "vector::push_back".to_string(),
            CppModuleIndexSymbol {
                kind: Some("method".to_string()),
                callable_signatures: vec!["void(int)".to_string()],
            },
        );
        modules.insert(
            "std".to_string(),
            CppModuleIndexModule {
                namespace: Some("std".to_string()),
                symbols,
            },
        );
        let options = TranspileOptions {
            cpp_module_symbol_index: Some(CppModuleSymbolIndex { modules }),
            cpp_module_symbol_index_sources: vec![PathBuf::from("/tmp/cpp-index.toml")],
            ..TranspileOptions::default()
        };

        let out = transpile_full_with_options(
            r#"
use cpp::std as cpp_std;
fn f(v: i32) -> i32 {
    let mut vec: *mut i32 = core::ptr::null_mut();
    unsafe { cpp_std::vector::push_back(vec, v) }
    0
}
"#,
            None,
            &UserTypeMap::default(),
            &HashSet::new(),
            None,
            &options,
        )
        .expect("member-function import syntax should transpile");

        assert!(out.contains("vec->push_back("));
    }

    #[test]
    fn test_cpp_module_call_errors_for_template_only_export_without_call_shape() {
        let mut modules = BTreeMap::new();
        let mut symbols = BTreeMap::new();
        symbols.insert(
            "sort".to_string(),
            CppModuleIndexSymbol {
                kind: Some("function_template".to_string()),
                callable_signatures: Vec::new(),
            },
        );
        modules.insert(
            "std".to_string(),
            CppModuleIndexModule {
                namespace: Some("std".to_string()),
                symbols,
            },
        );
        let options = TranspileOptions {
            cpp_module_symbol_index: Some(CppModuleSymbolIndex { modules }),
            cpp_module_symbol_index_sources: vec![PathBuf::from("/tmp/cpp-index.toml")],
            ..TranspileOptions::default()
        };

        let err = transpile_full_with_options(
            r#"
use cpp::std as cpp_std;
fn f(v: i32) -> i32 {
    unsafe { cpp_std::sort(v) }
}
"#,
            None,
            &UserTypeMap::default(),
            &HashSet::new(),
            None,
            &options,
        )
        .expect_err("template-only symbol without callable shape should fail");

        assert!(err.contains("TODO(leaf22.7)"));
        assert!(
            err.contains("template-only export without indexed callable signatures is unsupported")
        );
        assert!(err.contains("symbol `sort`"));
        assert!(err.contains("/tmp/cpp-index.toml"));
    }

    #[test]
    fn test_cpp_module_macro_usage_errors_as_unsupported_surface() {
        let mut modules = BTreeMap::new();
        modules.insert(
            "std".to_string(),
            CppModuleIndexModule {
                namespace: Some("std".to_string()),
                symbols: BTreeMap::new(),
            },
        );
        let options = TranspileOptions {
            cpp_module_symbol_index: Some(CppModuleSymbolIndex { modules }),
            cpp_module_symbol_index_sources: vec![PathBuf::from("/tmp/cpp-index.toml")],
            ..TranspileOptions::default()
        };

        let err = transpile_full_with_options(
            r#"
use cpp::std as cpp_std;
fn f() -> i32 {
    unsafe {
        let _ = cpp_std::max!(1, 2);
    }
    0
}
"#,
            None,
            &UserTypeMap::default(),
            &HashSet::new(),
            None,
            &options,
        )
        .expect_err("cpp macro usage should fail under MVP limits");

        assert!(err.contains("TODO(leaf22.7)"));
        assert!(err.contains("`cpp::` macro imports are unsupported in MVP"));
        assert!(err.contains("symbol `max`"));
        assert!(err.contains("/tmp/cpp-index.toml"));
    }

    // --- UFCS trait migration (book § 3.2.3): method-name classifier ---

    fn classify(src: &str) -> HashMap<String, MethodNameClass> {
        let file = syn::parse_str::<syn::File>(src).expect("parse");
        classify_method_names(&file.items)
    }

    #[test]
    fn test_classify_method_names_inherent_only() {
        let m = classify("struct Foo; impl Foo { fn bar(&self) {} }");
        assert_eq!(m.get("bar"), Some(&MethodNameClass::Inherent));
    }

    #[test]
    fn test_classify_method_names_trait_only() {
        let m = classify(
            "trait Tr { fn baz(&self); } struct Foo; impl Tr for Foo { fn baz(&self) {} }",
        );
        assert_eq!(m.get("baz"), Some(&MethodNameClass::TraitOnly));
    }

    #[test]
    fn test_classify_method_names_both() {
        // `len` is inherent on Foo AND a method of trait Sz → Both.
        let m = classify(
            "struct Foo; impl Foo { fn len(&self) -> usize { 0 } } \
             trait Sz { fn len(&self) -> usize; }",
        );
        assert_eq!(m.get("len"), Some(&MethodNameClass::Both));
    }

    #[test]
    fn test_classify_method_names_recurses_modules() {
        let m = classify(
            "mod a { trait Tr { fn m(&self); } } \
             mod b { struct F; impl F { fn n(&self) {} } }",
        );
        assert_eq!(m.get("m"), Some(&MethodNameClass::TraitOnly));
        assert_eq!(m.get("n"), Some(&MethodNameClass::Inherent));
    }

    #[test]
    fn test_classify_method_names_trait_default_counts_as_trait() {
        // A default-bodied trait method (no impl) is still a trait use.
        let m = classify("trait Greet { fn hello(&self) -> u8 { 0 } }");
        assert_eq!(m.get("hello"), Some(&MethodNameClass::TraitOnly));
    }

    #[test]
    fn test_classify_method_names_excludes_foreign_trait_impls() {
        // Phase 7: UFCS lowering is scoped to traits the crate DECLARES. A
        // prelude/foreign trait the crate only IMPLEMENTS (here `ForeignTr`)
        // contributes NO trait use — so its method name is not classified and
        // its calls stay on the non-UFCS path (otherwise `clone`/`fmt`/… would
        // be intercepted on unrelated std/library receivers). The crate-declared
        // `Mine` is still classified TraitOnly.
        let m = classify(
            "struct Foo; trait Mine { fn mine(&self); } \
             impl Mine for Foo { fn mine(&self) {} } \
             impl ForeignTr for Foo { fn ext(&self) {} }",
        );
        assert_eq!(m.get("mine"), Some(&MethodNameClass::TraitOnly));
        assert!(
            m.get("ext").is_none(),
            "a foreign-trait impl method must not be classified as a trait use"
        );
    }

    #[test]
    fn test_classify_method_names_foreign_impl_does_not_make_inherent_name_both() {
        // If a name is inherent on a type AND appears only in a foreign-trait
        // impl, it stays Inherent (the foreign use is dropped), not Both.
        let m = classify(
            "struct Foo; impl Foo { fn clone(&self) -> Foo { Foo } } \
             impl ForeignClone for Foo { fn clone(&self) -> Foo { Foo } }",
        );
        assert_eq!(m.get("clone"), Some(&MethodNameClass::Inherent));
    }
}
