use crate::codegen::CodeGen;
use crate::types::UserTypeMap;
use quote::ToTokens;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
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
    /// Trait name → `::`-joined C++-escaped declaration-module path (empty
    /// for crate root). Lets a consumer route a trait's PROVIDED STATIC
    /// (`Error::invalid_value` when the local impl lacks the override) to the
    /// declaring crate's `<Trait>RuntimeHelper::method<LocalSelf>(...)`.
    #[serde(default)]
    pub declared_trait_modules: std::collections::BTreeMap<String, String>,
    /// Declared trait name → the method names it declares (required + default).
    /// Lets a downstream crate's UFCS dedup be METHOD-AWARE: a dep declaring a
    /// trait of the same NAME (e.g. the ubiquitous private `Sealed`) must not
    /// suppress THIS crate's same-named-but-unrelated trait's free functions
    /// unless the dep's trait actually provides the same method.
    #[serde(default)]
    pub declared_trait_methods: BTreeMap<String, Vec<String>>,
    /// Declared trait name → ALL of its associated-type NAMES (`Serializer`
    /// → [Ok, Error, SerializeSeq, …]). Feeds the consumer's
    /// trait_associated_type_names so a method-param projection through a
    /// DEPENDENCY trait bound (`S: Serializer`, `-> Result<S::Ok, S::Error>`)
    /// counts as declared and keeps its spelled return type — the local-only
    /// map made such returns soften to `auto`, which is uncallable between a
    /// deferred declaration and its flushed definition.
    #[serde(default)]
    pub trait_assoc_type_names: BTreeMap<String, Vec<String>>,
    /// Declared trait name → the subset of its methods WITH default bodies.
    /// The `<Trait>RuntimeHelper` only carries these, so a consumer's
    /// member-vs-helper static dispatch must fall back to the helper ONLY for
    /// methods in this list — a required method (serde's `Error::custom`) has
    /// no helper member and the fallback branch is eagerly ill-formed.
    #[serde(default)]
    pub trait_default_methods: BTreeMap<String, Vec<String>>,
    /// `Trait::Assoc` → the SHORT name of the assoc type's first non-marker
    /// trait bound (`Serializer::SerializeMap` → `SerializeMap`). A consumer
    /// typing a local as the projection `S::SerializeMap` uses this to route a
    /// method call on it to the bound trait's preserved collapse body
    /// (§208 phase 2) — it never sees the dependency's trait declaration.
    #[serde(default)]
    pub trait_assoc_type_bounds: BTreeMap<String, String>,
    /// `TraitShort::method` pairs whose impl-collapse LOSING body was preserved
    /// as a `rusty_<Trait>_<method>` member somewhere in this crate's emission
    /// (§208). Consumers probe those members only for pairs in this list.
    #[serde(default)]
    pub preserved_collapse_methods: Vec<String>,
    /// `Trait::method` → the SHORT name of the first `Self::X` projection in
    /// the method's declared RETURN type (`Serializer::serialize_map` →
    /// `SerializeMap`, from `Result<Self::SerializeMap, Self::Error>`). Lets a
    /// consumer type an un-annotated `let m = s.serialize_map(..)?` local as
    /// the projection so §208 collapse-probe routing can fire on it.
    #[serde(default)]
    pub trait_method_return_assoc: BTreeMap<String, String>,
    /// `Trait::method` → whether the trait item's first param is `self`.
    /// A consumer lowering the trait-STATIC call form `T::method(a0, ...)`
    /// on a generic param T may treat `a0` as the receiver ONLY when the
    /// method takes `self` (Equivalent::equivalent); associated fns without
    /// a receiver (Deserialize::deserialize_in_place) must stay on the
    /// trait-static routing paths.
    #[serde(default)]
    pub trait_method_has_receiver: BTreeMap<String, bool>,
    /// `<Trait>::<method>` → the receiver's KIND: 0=`&self`, 1=`&mut self`,
    /// 2=`self`, 3=`mut self`. `trait_method_has_receiver` records only
    /// WHETHER there is one; a consumer spelling an explicit `Self_` template
    /// argument needs to know which, because `&self`/`&mut self` must keep the
    /// receiver's reference category while a by-value `self` consumes it. A
    /// consumer parses only the dependency's MANIFEST — it never sees the
    /// trait declaration — so this cannot be recovered locally.
    #[serde(default)]
    pub trait_method_receiver_kind: BTreeMap<String, u8>,
    /// `<Trait>::<method>` → how many leading template params of the emitted
    /// head precede `Self_`. A consumer may spell ONLY that many explicit
    /// template arguments; `Self_` then deduces from the receiver and the
    /// item-projection defaults behind it fill in. Cannot be recovered
    /// locally — a consumer never sees the dependency's trait declaration.
    #[serde(default)]
    pub trait_method_bare_template_prefix_len: BTreeMap<String, u8>,
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
    /// HYGIENE-ALIAS table (book § 32): a glob-only re-export shell module → the
    /// C++-escaped namespace it aliases. `cargo expand` serializes macro-hygiene
    /// `SyntaxContext`s into name suffixes (`__private228` is `__private` from one
    /// expansion context, whose body is just `pub use crate::private::*`). The numbers
    /// are crate-local, so a consumer's `serde_core::__private228` never matches the
    /// dependency's emission. This is the transpiler's `.rmeta` analog: it records the
    /// shell→canonical linkage ONCE so consumers resolve through it instead of by brittle
    /// number-matching — the same role hygiene contexts play in rustc's crate metadata.
    #[serde(default)]
    pub hygiene_aliases: BTreeMap<String, String>,
    /// `macro_rules!` names this crate exports, so a consumer can recognize and skip a
    /// re-exported dependency macro (no C++ entity to alias). Note: `cargo expand` strips
    /// `macro_rules!`, so this is usually empty and the consumer falls back to a
    /// not-in-surface heuristic in is_macro_rules_import.
    #[serde(default)]
    pub declared_macros: Vec<String>,
    /// Crate-ROOT re-exported item names with C++ entities (named re-exports
    /// + glob-expanded module items), EXCLUDING known macros. Lets a
    /// consumer's self-alias import rescue (`use itertools as it; use
    /// crate::it::interleave;`) emit `using ::itertools::interleave;` only
    /// for names that exist in the crate namespace.
    #[serde(default)]
    pub root_exported_names: Vec<String>,
    /// Every MODULE this crate declares, as `::`-joined C++-escaped crate-relative paths
    /// (`de`, `de::value`, `private_`, `private_::size_hint`). Lets a consumer recognize a
    /// crate-qualified reference to a wrapped dependency's module (`serde_core::private_::size_hint`)
    /// as a NAMESPACE — so a `use` of it emits a namespace alias, not a (broken) type alias.
    /// Separate from declared_types so module-vs-type is unambiguous.
    #[serde(default)]
    pub declared_modules: Vec<String>,
    /// C++ MODULE path (`de`, `ser`, `de::value`, `""` root) → the method names this crate emits
    /// as `<module>::rusty_ext::` free functions there. A consumer's cross-crate rusty_ext bridge
    /// imports a method ONLY from the exact module the dep emits it in
    /// (`using ::<dep>::<module>::rusty_ext::<m>;`) — never every declared trait method, and never
    /// at a guessed module. A REQUIRED method may remain a member with no rusty_ext free function
    /// (e.g. Serializer::serialize_bytes), or be emitted in a different module than the consumer
    /// references it through; either would make a guessed bridge name a non-existent member.
    /// Populated from `emitted_rusty_ext_methods_by_module`.
    /// Free-function name (and `::`-qualified spellings) -> per-argument pass
    /// style, encoded 0=Reference 1=Pointer 2=Value 3=Mixed.
    ///
    /// A consumer needs this to know that a dependency's function takes an
    /// argument BY VALUE, i.e. consumes it. Without it a local passed to such a
    /// call is not marked consumed, is emitted `const`, and `std::move` on a
    /// const lvalue silently selects the COPY constructor — a hard error for a
    /// move-only type and a silent copy (where Rust moved) for everything else.
    /// Cannot be recovered locally: the consumer never sees the dependency's
    /// function signatures.
    #[serde(default)]
    pub function_arg_pass_styles: std::collections::BTreeMap<String, Vec<u8>>,
    #[serde(default)]
    pub rusty_ext_methods_by_module: std::collections::BTreeMap<String, Vec<String>>,
    /// C-like enum VARIANT name → the enum's crate-relative C++ path
    /// (`YAML_STREAM_START_EVENT` → `yaml::yaml_event_type_t`). Rust re-exports
    /// variants into scope (`pub use yaml_event_type_t::*;` chained through
    /// crate-root globs), so a consumer can spell `dep::VARIANT`; C++
    /// enum-class variants only resolve through the ENUM-qualified path
    /// (`::dep::yaml::yaml_event_type_t::VARIANT`). Variant names declared by
    /// more than one enum are omitted (ambiguous).
    #[serde(default)]
    pub c_like_enum_variants: BTreeMap<String, String>,
    /// Crate-root re-exports whose target lives in ANOTHER crate: re-exported
    /// name → full dep-qualified target path (`de` → `serde_core::de` from
    /// serde's `pub use serde_core::{de, ser};`). A consumer spelling
    /// `serde::de::Visitor` must requalify to `::serde_core::de::Visitor` —
    /// the facade's C++ namespace `de` only holds the facade's OWN additions,
    /// not the re-exported dependency module's members.
    #[serde(default)]
    pub cross_crate_reexports: BTreeMap<String, String>,
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
    /// Whether the emitted C++ class stores its generic TYPE args inline
    /// (`Bucket<K, V> { K key; }`, or transitively via another inline-storing
    /// type), so a consumer FIELD of this type requires the args COMPLETE at
    /// declaration. `false` = pointer-backed (`IndexMap`'s Vec storage) — a
    /// forward declaration of the args suffices. Consumers must treat a
    /// MISSING entry (older manifests) as `true` (conservative).
    #[serde(default = "default_args_inline")]
    pub args_inline: bool,
}

fn default_args_inline() -> bool {
    true
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
    /// Named C++ module imported for this Rust interop binding.  This is
    /// intentionally independent from both the binding-path key in `modules`
    /// and the namespace that owns the indexed symbols.
    pub cpp_module: String,
    pub namespace: Option<String>,
    pub symbols: BTreeMap<String, CppModuleIndexSymbol>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct CppModuleIndexSymbol {
    pub kind: Option<String>,
    pub callable_signatures: Vec<String>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CppModuleSymbolIndexFile {
    #[serde(default = "default_cpp_module_symbol_index_version")]
    version: u32,
    #[serde(default)]
    modules: BTreeMap<String, CppModuleIndexModuleFile>,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct CppModuleIndexModuleFile {
    cpp_module: String,
    #[serde(default)]
    namespace: Option<String>,
    #[serde(default)]
    symbols: BTreeMap<String, CppModuleIndexSymbolFile>,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct CppModuleIndexSymbolFile {
    #[serde(default)]
    kind: Option<String>,
    #[serde(default)]
    callable_signatures: Vec<String>,
}

fn default_cpp_module_symbol_index_version() -> u32 {
    1
}

/// One consumer-specific projection of a Rust crate module into the C++
/// module graph.  The Rust path is canonicalized without its leading
/// `crate::` (the crate root itself is the empty string).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConsumerModuleEntry {
    pub rust_module: String,
    pub cpp_module: String,
    pub cpp_namespace: String,
}

/// Consumer-owned module/namespace projection used when the C++ surface must
/// not mirror the Rust crate hierarchy.  Mako, for example, maps many
/// `crate::base::*` / `crate::rpc::*` modules to legacy `rrr.*` named modules
/// whose exported entities all live in the single `rrr` namespace.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ConsumerModuleMap {
    pub modules: BTreeMap<String, ConsumerModuleEntry>,
}

impl ConsumerModuleMap {
    pub fn entry_for_cpp_module(&self, cpp_module: &str) -> Option<&ConsumerModuleEntry> {
        self.modules
            .values()
            .find(|entry| entry.cpp_module == cpp_module)
    }

    pub fn entry_for_rust_module(&self, rust_module: &str) -> Option<&ConsumerModuleEntry> {
        self.modules.get(rust_module)
    }

    pub fn is_empty(&self) -> bool {
        self.modules.is_empty()
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ConsumerModuleMapFile {
    version: u32,
    #[serde(default)]
    module: Vec<ConsumerModuleMapFileEntry>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ConsumerModuleMapFileEntry {
    rust_module: String,
    cpp_module: String,
    cpp_namespace: String,
}

fn canonical_consumer_rust_module_path(raw: &str) -> Result<String, String> {
    let trimmed = raw.trim();
    let path = syn::parse_str::<syn::Path>(trimmed)
        .map_err(|e| format!("invalid Rust module path '{}': {}", raw, e))?;
    if path.leading_colon.is_some()
        || path
            .segments
            .iter()
            .any(|segment| !matches!(segment.arguments, syn::PathArguments::None))
    {
        return Err(format!(
            "Rust module path '{}' must be an unparameterized crate path",
            raw
        ));
    }
    let mut segments = path.segments.iter();
    let Some(root) = segments.next() else {
        return Err("Rust module path must not be empty".to_string());
    };
    if root.ident != "crate" {
        return Err(format!(
            "Rust module path '{}' must begin with 'crate'",
            raw
        ));
    }
    Ok(segments
        .map(|segment| segment.ident.to_string())
        .collect::<Vec<_>>()
        .join("::"))
}

fn valid_cpp_identifier(identifier: &str) -> bool {
    let mut chars = identifier.chars();
    chars
        .next()
        .is_some_and(|ch| ch == '_' || ch.is_ascii_alphabetic())
        && chars.all(|ch| ch == '_' || ch.is_ascii_alphanumeric())
}

fn validate_cpp_qualified_name(raw: &str, separator: &str, label: &str) -> Result<(), String> {
    let segments: Vec<&str> = raw.split(separator).collect();
    if segments.is_empty()
        || segments
            .iter()
            .any(|segment| !valid_cpp_identifier(segment))
    {
        return Err(format!("invalid {} '{}'", label, raw));
    }
    Ok(())
}

/// Load the consumer module projection accepted by
/// `--consumer-module-map`. JSON and TOML are supported; an unknown extension
/// is tried as JSON first and TOML second, matching the C++ symbol-index
/// sidecar behavior.
pub fn load_consumer_module_map(path: &Path) -> Result<ConsumerModuleMap, String> {
    let content = fs::read_to_string(path).map_err(|e| {
        format!(
            "Failed to read consumer module map {}: {}",
            path.display(),
            e
        )
    })?;
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .map(|value| value.to_ascii_lowercase());
    let parsed: ConsumerModuleMapFile = match extension.as_deref() {
        Some("json") => serde_json::from_str(&content).map_err(|e| {
            format!("Invalid JSON consumer module map {}: {}", path.display(), e)
        })?,
        Some("toml") => toml::from_str(&content).map_err(|e| {
            format!("Invalid TOML consumer module map {}: {}", path.display(), e)
        })?,
        _ => match serde_json::from_str(&content) {
            Ok(value) => value,
            Err(json_error) => toml::from_str(&content).map_err(|toml_error| {
                format!(
                    "Failed to parse consumer module map {} as JSON ({}) or TOML ({})",
                    path.display(),
                    json_error,
                    toml_error
                )
            })?,
        },
    };
    if parsed.version != 1 {
        return Err(format!(
            "Unsupported consumer module map version {} in {} (expected version 1)",
            parsed.version,
            path.display()
        ));
    }
    if parsed.module.is_empty() {
        return Err(format!(
            "Consumer module map {} contains no module entries",
            path.display()
        ));
    }

    let mut modules = BTreeMap::new();
    let mut cpp_modules = HashSet::new();
    for entry in parsed.module {
        let rust_module = canonical_consumer_rust_module_path(&entry.rust_module)?;
        validate_cpp_qualified_name(&entry.cpp_module, ".", "C++ module name")?;
        validate_cpp_qualified_name(&entry.cpp_namespace, "::", "C++ namespace")?;
        if modules.contains_key(&rust_module) {
            return Err(format!(
                "Consumer module map {} repeats Rust module '{}'",
                path.display(),
                entry.rust_module
            ));
        }
        if !cpp_modules.insert(entry.cpp_module.clone()) {
            return Err(format!(
                "Consumer module map {} repeats C++ module '{}'",
                path.display(),
                entry.cpp_module
            ));
        }
        modules.insert(
            rust_module.clone(),
            ConsumerModuleEntry {
                rust_module,
                cpp_module: entry.cpp_module,
                cpp_namespace: entry.cpp_namespace,
            },
        );
    }
    Ok(ConsumerModuleMap { modules })
}

/// Delimiter used for an explicitly requested global-module-fragment include.
///
/// This deliberately models only the two C++ include forms.  The preamble API
/// does not accept arbitrary preprocessor text.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GmfIncludeForm {
    Angle,
    Quote,
}

/// One validated-by-transpilation include request for a C++ module's global
/// module fragment.  Entries are emitted in the order supplied.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GmfIncludeSpec {
    pub path: String,
    pub form: GmfIncludeForm,
}

impl GmfIncludeSpec {
    fn render(&self) -> String {
        match self.form {
            GmfIncludeForm::Angle => format!("#include <{}>", self.path),
            GmfIncludeForm::Quote => format!("#include \"{}\"", self.path),
        }
    }
}

const MODULE_PREAMBLE_FILE_VERSION: u32 = 1;

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ModulePreambleFile {
    version: u32,
    #[serde(default, rename = "module")]
    modules: Vec<ModulePreambleFileRow>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ModulePreambleFileRow {
    name: String,
    #[serde(default)]
    includes: Vec<GmfIncludeFileSpec>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct GmfIncludeFileSpec {
    path: String,
    form: GmfIncludeFileForm,
    #[serde(default)]
    when: Option<GmfIncludeFileCondition>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "lowercase")]
enum GmfIncludeFileForm {
    Angle,
    Quote,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct GmfIncludeFileCondition {
    target_os: Vec<String>,
}

/// A loaded, target-filtered module-preamble sidecar.  Selection is separate
/// from loading so crate mode can reject rows that no emitted module collected.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModulePreambleManifest {
    source: PathBuf,
    modules: BTreeMap<String, Vec<GmfIncludeSpec>>,
}

impl ModulePreambleManifest {
    /// Select preambles for one complete emission set.  Missing rows are fine
    /// (most modules need no additional headers), but every row present in the
    /// sidecar must be collected.  That makes renamed/deleted module rows a
    /// deterministic error instead of silently ignoring stale configuration.
    pub fn select_for_modules<'a, I>(
        &self,
        emitted_modules: I,
    ) -> Result<BTreeMap<String, Vec<GmfIncludeSpec>>, String>
    where
        I: IntoIterator<Item = &'a str>,
    {
        let emitted: std::collections::BTreeSet<String> =
            emitted_modules.into_iter().map(str::to_string).collect();
        let stale: Vec<&str> = self
            .modules
            .keys()
            .filter(|name| !emitted.contains(*name))
            .map(String::as_str)
            .collect();
        if !stale.is_empty() {
            let emitted_label = if emitted.is_empty() {
                "<none>".to_string()
            } else {
                emitted.iter().cloned().collect::<Vec<_>>().join(", ")
            };
            return Err(format!(
                "Module preamble {} has stale/uncollected [[module]] row(s): {} (emitted modules: {})",
                self.source.display(),
                stale.join(", "),
                emitted_label
            ));
        }

        Ok(self
            .modules
            .iter()
            .filter(|(name, _)| emitted.contains(*name))
            .map(|(name, includes)| (name.clone(), includes.clone()))
            .collect())
    }
}

fn validate_module_preamble_name(name: &str) -> Result<(), String> {
    let valid_segment = |segment: &str| {
        let mut chars = segment.chars();
        chars
            .next()
            .is_some_and(|c| c == '_' || c.is_ascii_alphabetic())
            && chars.all(|c| c == '_' || c.is_ascii_alphanumeric())
    };
    if name.is_empty() || !name.split('.').all(valid_segment) {
        return Err(format!(
            "invalid module preamble name {:?}: expected dot-separated C++ identifiers",
            name
        ));
    }
    Ok(())
}

fn validate_target_os_name(target_os: &str) -> Result<(), String> {
    if target_os.is_empty()
        || !target_os
            .chars()
            .all(|c| c == '_' || c.is_ascii_lowercase() || c.is_ascii_digit())
    {
        return Err(format!(
            "invalid module-preamble target_os {:?}: expected lowercase ASCII letters, digits, or underscore",
            target_os
        ));
    }
    Ok(())
}

/// Validate the injection-safe subset accepted by the structured preamble
/// API.  Paths are portable, relative include names; directives, delimiters,
/// escapes, whitespace, traversal, and duplicate/conflicting rows are rejected.
pub fn validate_explicit_gmf_includes(includes: &[GmfIncludeSpec]) -> Result<(), String> {
    let mut paths: BTreeMap<&str, GmfIncludeForm> = BTreeMap::new();
    for (index, include) in includes.iter().enumerate() {
        let path = include.path.as_str();
        if path.is_empty() {
            return Err(format!("GMF include #{} has an empty path", index + 1));
        }
        if Path::new(path).is_absolute() || path.starts_with('/') {
            return Err(format!(
                "GMF include #{} path {:?} must be relative",
                index + 1,
                path
            ));
        }
        if path.chars().any(char::is_control) {
            return Err(format!(
                "GMF include #{} path {:?} contains a control character",
                index + 1,
                path
            ));
        }
        if path.contains(['\"', '\'', '<', '>']) {
            return Err(format!(
                "GMF include #{} path {:?} contains a quote or include delimiter",
                index + 1,
                path
            ));
        }
        if path.contains('\\') {
            return Err(format!(
                "GMF include #{} path {:?} contains a backslash; use forward slashes",
                index + 1,
                path
            ));
        }
        if !path
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.' | '+' | '/'))
        {
            return Err(format!(
                "GMF include #{} path {:?} contains a disallowed character",
                index + 1,
                path
            ));
        }
        if path
            .split('/')
            .any(|segment| segment.is_empty() || segment == "." || segment == "..")
        {
            return Err(format!(
                "GMF include #{} path {:?} contains an empty, '.' or '..' component",
                index + 1,
                path
            ));
        }
        if let Some(previous_form) = paths.insert(path, include.form) {
            let kind = if previous_form == include.form {
                "duplicate"
            } else {
                "conflicting angle/quote"
            };
            return Err(format!(
                "GMF include #{} is a {} entry for path {:?}",
                index + 1,
                kind,
                path
            ));
        }
    }
    Ok(())
}

/// Load the strict version-1 TOML sidecar used by `--module-preamble`.
///
/// `when = { target_os = [...] }` is the only supported condition.  If any
/// condition is present, the caller must name the intended target explicitly;
/// host autodetection would be wrong for cross compilation, so omission fails
/// closed.
pub fn load_module_preamble_file(
    path: &Path,
    target_os: Option<&str>,
) -> Result<ModulePreambleManifest, String> {
    if let Some(target_os) = target_os {
        validate_target_os_name(target_os)?;
    }
    let content = fs::read_to_string(path)
        .map_err(|e| format!("Failed to read module preamble {}: {}", path.display(), e))?;
    let file: ModulePreambleFile = toml::from_str(&content)
        .map_err(|e| format!("Invalid TOML module preamble {}: {}", path.display(), e))?;
    if file.version != MODULE_PREAMBLE_FILE_VERSION {
        return Err(format!(
            "Unsupported module preamble version {} in {} (expected version {})",
            file.version,
            path.display(),
            MODULE_PREAMBLE_FILE_VERSION
        ));
    }
    if file.modules.is_empty() {
        return Err(format!(
            "Module preamble {} must contain at least one [[module]] row",
            path.display()
        ));
    }

    let has_condition = file
        .modules
        .iter()
        .flat_map(|row| &row.includes)
        .any(|include| include.when.is_some());
    if has_condition && target_os.is_none() {
        return Err(format!(
            "Module preamble {} contains a target_os condition; pass --preamble-target-os explicitly",
            path.display()
        ));
    }

    let mut modules = BTreeMap::new();
    for row in file.modules {
        validate_module_preamble_name(&row.name)
            .map_err(|e| format!("{} in {}", e, path.display()))?;
        if row.includes.is_empty() {
            return Err(format!(
                "Module preamble {} row {:?} has no includes",
                path.display(),
                row.name
            ));
        }

        let mut unfiltered = Vec::with_capacity(row.includes.len());
        let mut conditions = Vec::with_capacity(row.includes.len());
        for include in row.includes {
            if let Some(condition) = &include.when {
                if condition.target_os.is_empty() {
                    return Err(format!(
                        "Module preamble {} row {:?} has an empty target_os condition",
                        path.display(),
                        row.name
                    ));
                }
                let mut seen = std::collections::BTreeSet::new();
                for value in &condition.target_os {
                    validate_target_os_name(value).map_err(|e| {
                        format!("{} in module {:?} of {}", e, row.name, path.display())
                    })?;
                    if !seen.insert(value) {
                        return Err(format!(
                            "Module preamble {} row {:?} repeats target_os {:?}",
                            path.display(),
                            row.name,
                            value
                        ));
                    }
                }
            }
            unfiltered.push(GmfIncludeSpec {
                path: include.path,
                form: match include.form {
                    GmfIncludeFileForm::Angle => GmfIncludeForm::Angle,
                    GmfIncludeFileForm::Quote => GmfIncludeForm::Quote,
                },
            });
            conditions.push(include.when);
        }
        validate_explicit_gmf_includes(&unfiltered)
            .map_err(|e| format!("{} in module {:?} of {}", e, row.name, path.display()))?;

        let selected = unfiltered
            .into_iter()
            .zip(conditions)
            .filter_map(|(include, condition)| {
                let enabled = condition.as_ref().is_none_or(|condition| {
                    let target_os = target_os.expect("conditions require target_os above");
                    condition.target_os.iter().any(|value| value == target_os)
                });
                enabled.then_some(include)
            })
            .collect::<Vec<_>>();
        if modules.insert(row.name.clone(), selected).is_some() {
            return Err(format!(
                "Module preamble {} repeats [[module]] name {:?}",
                path.display(),
                row.name
            ));
        }
    }

    Ok(ModulePreambleManifest {
        source: path.to_path_buf(),
        modules,
    })
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TranspileOptions {
    /// Enable `CodeGen::set_crate_name` on the `--crate` path (using the module
    /// name, which IS the crate name there), otherwise hard-coded to `None`.
    ///
    /// Setting it enables `wrap_module_purview_in_crate_namespace` — the wrap
    /// that ALSO requalifies the crate's own qualified self-references (Rules
    /// 1-5). That distinguishes it from the blunt `--cxx-namespace` textual
    /// wrap, which does no requalification and leaves a crate referring to
    /// `control::tag::Tag` from inside the wrap looking for
    /// `<ns>::control::tag::control::tag::Tag` (272 such errors on hashbrown).
    ///
    /// OPT-IN ON PURPOSE: the `--crate` entry points are shared with the
    /// `alloc` and `path` stdlib ports, which are emitted unwrapped today and
    /// are green in the parity matrix. Default `None` keeps their emission
    /// byte-identical.
    pub crate_namespace_wrap: bool,
    /// Is this crate INSIDE the `rusty` umbrella module's re-export closure?
    ///
    /// The C++20 umbrella (`include/rusty/rusty.cppm`) `export import`s the
    /// ported collection modules (`vec_port`, `rc_port`, `std_port`, …). So a
    /// crate that the umbrella re-exports must NOT import the umbrella back —
    /// that is a module cycle — and therefore has to name every collection by
    /// its DEEP path (`::rusty::port::vec::Vec`) over a NARROW import of the
    /// declaring module. A crate OUTSIDE the closure is an ordinary CONSUMER of
    /// rusty: `rusty::Vec` is its stable spelling and must survive verbatim
    /// (re-seated by a using-declaration onto the same narrow import).
    ///
    /// This is a property of the BUILD GRAPH, not of the Rust source — nothing
    /// in the source can decide it, which is why it is an explicit flag set by
    /// the invoking pipeline rather than a heuristic on the module name. It
    /// replaces the former `module_is_std_port` name heuristic (`*_port`
    /// suffix), which could not express the case that actually occurs:
    /// `hashbrown` is transpiled BOTH as `std_port`'s vendored path dependency
    /// (inside the closure -> deep paths) AND as a standalone parity-matrix
    /// crate (outside -> alias spelling), under the same module name and with
    /// opposite classification.
    ///
    /// Default `false` = outside the closure = consumer, the safe default for
    /// every third-party crate.
    ///
    /// Set by: `docs/rusty/build.sh`, `docs/alloc/build.sh`,
    /// `docs/path/build.sh`, `docs/port_regen/regen_diff.sh` (CLI
    /// `--in-umbrella-closure`). NOT set by the parity matrix
    /// (`tests/transpile_tests/run_parity_matrix.sh`), whose crates are
    /// consumers.
    pub in_umbrella_closure: bool,
    /// Don't PANIC on an `<auto>` template-argument leak — return the (broken)
    /// output instead. Set by the parity harness for SKIPPABLE test targets:
    /// their `cpp_has_invalid_codegen_pattern` check skips such a target
    /// gracefully, but only if transpilation returns at all. Without this, one
    /// unbuildable target (itertools' quickcheck-based `quick`) aborts the
    /// whole parity run before the skip logic can see it. Essential units
    /// (libs, deps) keep the loud panic.
    pub lenient_auto_template_args: bool,
    /// Opt-in diagnostic-only prototype for by-value SCC cycle-breaking planning.
    /// Default is `false`.
    pub by_value_cycle_breaking_prototype: bool,
    /// Optional C++ module symbol index for `use cpp::...` interop resolution.
    pub cpp_module_symbol_index: Option<CppModuleSymbolIndex>,
    /// Source paths used to load the configured C++ module symbol index.
    /// Used in diagnostics so unresolved-symbol errors point to the configured index input.
    pub cpp_module_symbol_index_sources: Vec<PathBuf>,
    /// Optional consumer-specific projection from Rust crate-module paths to
    /// named C++ modules and their actual namespaces. Unlike
    /// `crate_module_names`, this supports non-isomorphic graphs such as a
    /// legacy flat `rrr` namespace spread across `rrr.*` module units.
    pub consumer_module_map: ConsumerModuleMap,
    /// Optional Rust lexical module for the unit currently being emitted.
    /// The value must be a canonical `crate::...` path and is normalized here
    /// without its leading `crate::`. This is deliberately
    /// separate from `consumer_module_map`: grouped implementation units can
    /// share an interface's C++ module without becoming a second canonical
    /// owner in that map.
    pub consumer_rust_module: Option<String>,
    /// Maps Rust external crate roots to transpiled C++ module namespaces available
    /// in the current compilation unit (for example `serde_core` -> `serde_core`).
    pub external_crate_module_aliases: HashMap<String, String>,
    /// External crate roots whose selected Cargo package identity is trusted
    /// to provide compiler-owned attributes. Empty for source-only/direct
    /// transpilation, which therefore fails closed on lookalike proc macros.
    pub authenticated_cpp_inherit_roots: std::collections::HashSet<String>,
    /// Crate-mode preflight proved that the selected `rusty` facade re-exports
    /// the exact inert `rusty-cpp-markers::cpp_inherit` implementation.  Keep
    /// this separate from package-root authentication: an exact package name
    /// alone does not prove that an attribute macro cannot synthesize hidden
    /// items beside a cpp_name overload contract.
    pub cpp_name_trusted_cpp_inherit_provenance: bool,
    /// Compiler sysroot crate roots that Cargo has proved are not occupied by
    /// an extern-prelude dependency of the current package. Source-only calls
    /// use the ordinary `std`/`core` assumption; Cargo-backed lanes replace it
    /// with manifest-specific provenance before lowering erased `Default`.
    pub authenticated_sysroot_roots: std::collections::HashSet<String>,
    /// Exact Rust item bindings harvested from sibling inline blocks. Inline
    /// payloads form one logical Rust module even though each block is lowered
    /// separately, so compiler-owned markers must see imports in any block.
    pub cross_file_rust_item_import_bindings: RustItemImportBindings,
    /// C++ `using X = Y;` aliases from the translation unit an inline-rust
    /// block is spliced into. Lets type predicates see through a C++ alias
    /// to the underlying rusty type (`WeakClientConnection` ->
    /// `rusty::sync::Weak<..>`), which name-based matching cannot do.
    /// Empty outside inline-rust mode.
    pub cpp_type_aliases: HashMap<String, String>,
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
    /// Ordered, structured include requests for this module's global module
    /// fragment.  Rejected for non-module output and validated before parsing
    /// or code generation.  Empty preserves the historical output byte-for-byte.
    pub explicit_gmf_includes: Vec<GmfIncludeSpec>,
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
    /// Trait declarations harvested from every file of the crate (C9): a
    /// module that imports a crate trait still needs to know its interface
    /// class exists.
    pub cross_file_traits: Vec<syn::ItemTrait>,
    /// B: crate-wide (Rust name -> audited C++ name) for cpp_name identities
    /// owned by ANY file of the crate, so a caller in another file emits the
    /// owner's identity instead of the crate audit rejecting the reference.
    pub cross_file_cpp_name_targets: std::collections::BTreeMap<String, String>,
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
    /// (type, trait) pairs for `#[cpp_inherit] impl Trait for Type` blocks
    /// living in SIBLING inline-rust blocks of the same file. A struct
    /// literal of such a type must lower to the fieldwise ctor (the
    /// emitted C++ struct has a base class, so designated init is
    /// illegal) — but the flag is registered by the collect pass of the
    /// block that CONTAINS the impl, which a sibling block never runs.
    pub cross_file_cpp_inherit: Vec<(syn::ItemStruct, String)>,
    /// Cross-file type-alias declarations (`pub type Foo<K> = Bar<...>;`)
    /// collected during a crate-mode pre-pass. Used to resolve orphan
    /// impl blocks targeting a type alias back to the underlying struct
    /// so the methods are absorbed into the struct's body and the
    /// orphan emission is suppressed. Empty for single-file mode.
    pub cross_file_type_aliases: Vec<syn::ItemType>,
    /// Source-specific, crate-preflight-proven type bindings.  Each record
    /// retains its physical consumer/provider modules, lexical marker scope,
    /// exact marked-use leaf group, C++ namespace, and provider kind.  An
    /// empty set is the fail-closed default outside audited crate mode.
    pub(crate) flat_import_type_authorizations:
        BTreeSet<crate::cpp_abi::FlatImportTypeAuthorization>,
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
    classify_method_names_excluding_traits(items, &std::collections::HashSet::new())
}

/// The ordinary UFCS classifier with an explicit set of local traits that
/// must preserve member dispatch. The compiler-owned
/// `cpp_trait_member_dispatch` marker feeds this set from codegen; keeping the
/// filtering here means a same-named inherent method or an unmarked trait still
/// contributes its normal classification.
pub fn classify_method_names_excluding_traits(
    items: &[syn::Item],
    excluded_traits: &std::collections::HashSet<String>,
) -> HashMap<String, MethodNameClass> {
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
    let declared_trait_paths = collect_declared_trait_paths(items);
    let import_bindings = collect_rust_item_import_bindings(items);
    let mut inherent: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut trait_named: std::collections::HashSet<String> = std::collections::HashSet::new();
    collect_method_name_uses(
        items,
        &declared_trait_paths,
        &import_bindings,
        excluded_traits,
        &[],
        &mut inherent,
        &mut trait_named,
    );

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
/// crate's own items must be qualified to `::<crate>::…`. UNIVERSAL: every
/// transpiled crate is wrapped (the parity matrix is 14/0/1 under the flip).
pub fn crate_is_namespace_wrapped(crate_name: &str) -> bool {
    // Universal namespace wrapping. Every transpiled crate is emitted inside
    // `namespace <crate> { … }` so its modules never ODR-collide with a same-named
    // module in an imported dependency (serde's `de` vs serde_core's `de`). A clean
    // flip-to-ALL required the full self-requalification rule set in
    // `wrap_module_purview_in_crate_namespace` (Rules 1-5: exclusive namespaces, UFCS
    // bridges, crate-root re-exported types, crate-root free fns, test-harness symbols),
    // the dep-name-never-self guard (a dependency name is never requalified to
    // `::<crate>::<dep>`), and cross-crate re-export resolution so a wrapped facade
    // (serde) re-exporting a wrapped dependency's sub-submodule member (serde_core's
    // `de::ignored_any::IgnoredAny`) qualifies to the dependency instead of being
    // dropped as unresolved. Validated 14/0/1 on the parity matrix (serde_yaml is a
    // pre-existing known-fail unrelated to wrapping). The `crate_name` param is retained
    // for a future per-crate opt-out, but no crate currently needs one.
    let _ = crate_name;
    true
}

/// Short names of every trait this crate DECLARES (`trait Tr { … }`), recursing
/// into inline modules. Used to scope UFCS lowering + emission to crate-declared
/// traits (prelude/std-trait impls are left to the non-UFCS path).
/// Trait name → escaped module path, for the manifest's
/// `declared_trait_modules` (first declaration wins on rare name reuse).
pub fn collect_declared_trait_modules(
    items: &[syn::Item],
) -> std::collections::BTreeMap<String, String> {
    fn walk(
        items: &[syn::Item],
        path: &mut Vec<String>,
        out: &mut std::collections::BTreeMap<String, String>,
    ) {
        for item in items {
            match item {
                syn::Item::Trait(t) => {
                    out.entry(t.ident.to_string())
                        .or_insert_with(|| path.join("::"));
                }
                syn::Item::Mod(m) => {
                    if let Some((_, nested)) = &m.content {
                        let seg = m.ident.to_string();
                        // Minimal escape matching codegen's module spelling
                        // (private/mut_/etc get a trailing underscore).
                        let escaped = match seg.as_str() {
                            "private" | "mut" | "new" | "delete" | "default" | "register"
                            | "template" | "typename" | "union" | "unsigned" | "signed" | "int"
                            | "char" | "float" | "double" | "namespace" | "operator" | "class" => {
                                format!("{}_", seg)
                            }
                            _ => seg,
                        };
                        path.push(escaped);
                        walk(nested, path, out);
                        path.pop();
                    }
                }
                _ => {}
            }
        }
    }
    let mut out = std::collections::BTreeMap::new();
    walk(items, &mut Vec::new(), &mut out);
    out
}

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
                if module_is_cfg_disabled(m) {
                    continue;
                }
                if let Some((_, nested)) = &m.content {
                    collect_declared_trait_names_into(nested, out);
                }
            }
            _ => {}
        }
    }
}

/// Fully-qualified lexical paths of every crate-declared trait.  Unlike the
/// legacy short-name registry, these keys distinguish `a::Clash` from
/// `b::Clash` and are therefore safe for compiler-owned, per-trait behavior.
pub(crate) fn collect_declared_trait_paths(
    items: &[syn::Item],
) -> std::collections::HashSet<String> {
    fn walk(
        items: &[syn::Item],
        module_path: &mut Vec<String>,
        out: &mut std::collections::HashSet<String>,
    ) {
        for item in items {
            match item {
                syn::Item::Trait(trait_item) => {
                    let trait_name = trait_item.ident.to_string();
                    let key = if module_path.is_empty() {
                        trait_name
                    } else {
                        format!("{}::{}", module_path.join("::"), trait_name)
                    };
                    out.insert(key);
                }
                syn::Item::Mod(module) => {
                    if module_is_cfg_disabled(module) {
                        continue;
                    }
                    if let Some((_, nested)) = &module.content {
                        module_path.push(module.ident.to_string());
                        walk(nested, module_path, out);
                        module_path.pop();
                    }
                }
                _ => {}
            }
        }
    }

    let mut out = std::collections::HashSet::new();
    walk(items, &mut Vec::new(), &mut out);
    out
}

/// Exact Rust item-import bindings, keyed by `(lexical module, local name)`.
///
/// These bindings intentionally retain Rust paths rather than their emitted
/// C++ spelling.  Compiler-owned attributes and per-trait lowering decisions
/// must be based on the declaration that rustc resolves, not on an equal leaf
/// name or on a C++ namespace alias.
pub type RustItemImportBindings = std::collections::HashMap<
    (String, String),
    std::collections::HashSet<String>,
>;

fn rust_local_module_shadow_key(local_name: &str) -> String {
    // `@` cannot occur in a Rust identifier, so this cannot collide with a
    // source import binding stored in the same compact resolution table.
    format!("@local-module:{local_name}")
}

fn rust_local_trait_shadow_key(local_name: &str) -> String {
    // Traits and modules both live in Rust's type namespace. Keep a distinct
    // sentinel only so the resolver can tell whether a remaining path tail is
    // legal after selecting the declaration.
    format!("@local-trait:{local_name}")
}

fn rust_glob_import_key() -> String {
    // Like the local-module sentinel above, this key cannot collide with a
    // source identifier. A glob can introduce any public leaf, so identity-
    // sensitive resolution must fail closed when one is lexically visible.
    "@glob-import".to_string()
}

pub(crate) fn collect_rust_item_import_bindings(
    items: &[syn::Item],
) -> RustItemImportBindings {
    fn normalized_target(module_path: &[String], segments: &[String]) -> String {
        let mut prefix = Vec::new();
        let mut index = 0usize;
        let mut explicitly_local = false;
        if segments.first().is_some_and(|segment| segment == "crate") {
            index = 1;
            explicitly_local = true;
        } else if segments
            .first()
            .is_some_and(|segment| segment == "self" || segment == "super")
        {
            prefix = module_path.to_vec();
            explicitly_local = true;
            while index < segments.len() {
                match segments[index].as_str() {
                    "self" => index += 1,
                    "super" => {
                        prefix.pop();
                        index += 1;
                    }
                    _ => break,
                }
            }
        }
        let normalized = prefix
            .iter()
            .chain(segments[index..].iter())
            .cloned()
            .collect::<Vec<_>>()
            .join("::");
        if explicitly_local {
            format!("@crate:{normalized}")
        } else {
            normalized
        }
    }

    fn flatten(
        tree: &syn::UseTree,
        prefix: &mut Vec<String>,
        module_path: &[String],
        scope: &str,
        leading_colon: bool,
        out: &mut RustItemImportBindings,
    ) {
        match tree {
            syn::UseTree::Path(path) => {
                prefix.push(path.ident.to_string());
                flatten(
                    &path.tree,
                    prefix,
                    module_path,
                    scope,
                    leading_colon,
                    out,
                );
                prefix.pop();
            }
            syn::UseTree::Name(name) => {
                let mut target = prefix.clone();
                if name.ident != "self" {
                    target.push(name.ident.to_string());
                }
                let Some(local_name) = target.last().cloned() else {
                    return;
                };
                let mut target = normalized_target(module_path, &target);
                if leading_colon {
                    target.insert_str(0, "::");
                }
                if !target.is_empty() {
                    out.entry((scope.to_string(), local_name))
                        .or_default()
                        .insert(target);
                }
            }
            syn::UseTree::Rename(rename) => {
                let mut target = prefix.clone();
                if rename.ident != "self" {
                    target.push(rename.ident.to_string());
                }
                let mut target = normalized_target(module_path, &target);
                if leading_colon {
                    target.insert_str(0, "::");
                }
                let local_name = rename.rename.to_string();
                if local_name != "_" && !target.is_empty() {
                    out.entry((scope.to_string(), local_name))
                        .or_default()
                        .insert(target);
                }
            }
            syn::UseTree::Group(group) => {
                for nested in &group.items {
                    flatten(
                        nested,
                        prefix,
                        module_path,
                        scope,
                        leading_colon,
                        out,
                    );
                }
            }
            syn::UseTree::Glob(_) => {
                let mut target = normalized_target(module_path, prefix);
                if leading_colon {
                    target.insert_str(0, "::");
                }
                out.entry((scope.to_string(), rust_glob_import_key()))
                    .or_default()
                    .insert(target);
            }
        }
    }

    fn walk(
        items: &[syn::Item],
        module_path: &mut Vec<String>,
        out: &mut RustItemImportBindings,
    ) {
        let scope = module_path.join("::");
        for item in items {
            match item {
                syn::Item::Use(item_use) => {
                    flatten(
                        &item_use.tree,
                        &mut Vec::new(),
                        module_path,
                        &scope,
                        item_use.leading_colon.is_some(),
                        out,
                    );
                }
                syn::Item::Mod(module) if !module_is_cfg_disabled(module) => {
                    out.entry((
                        scope.clone(),
                        rust_local_module_shadow_key(&module.ident.to_string()),
                    ))
                    .or_default()
                    .insert(if scope.is_empty() {
                        module.ident.to_string()
                    } else {
                        format!("{}::{}", scope, module.ident)
                    });
                    if let Some((_, nested)) = &module.content {
                        module_path.push(module.ident.to_string());
                        walk(nested, module_path, out);
                        module_path.pop();
                    }
                }
                syn::Item::Trait(item_trait) => {
                    out.entry((
                        scope.clone(),
                        rust_local_trait_shadow_key(&item_trait.ident.to_string()),
                    ))
                    .or_default()
                    .insert(if scope.is_empty() {
                        item_trait.ident.to_string()
                    } else {
                        format!("{}::{}", scope, item_trait.ident)
                    });
                }
                syn::Item::ExternCrate(item_extern_crate) => {
                    let local_name = item_extern_crate
                        .rename
                        .as_ref()
                        .map(|(_, rename)| rename.to_string())
                        .unwrap_or_else(|| item_extern_crate.ident.to_string());
                    out.entry((scope.clone(), local_name))
                        .or_default()
                        .insert(format!("::{}", item_extern_crate.ident));
                }
                _ => {}
            }
        }
    }

    let mut out = RustItemImportBindings::new();
    walk(items, &mut Vec::new(), &mut out);
    out
}

fn nearest_rust_item_import_targets<'a>(
    local_name: &str,
    module_path: &[String],
    bindings: &'a RustItemImportBindings,
) -> Option<(Vec<String>, &'a std::collections::HashSet<String>)> {
    for depth in (0..=module_path.len()).rev() {
        let scope = module_path[..depth].join("::");
        if let Some(targets) = bindings.get(&(scope, local_name.to_string())) {
            return Some((module_path[..depth].to_vec(), targets));
        }
    }
    None
}

fn exact_rust_item_targets<'a>(
    local_name: &str,
    module_path: &[String],
    bindings: &'a RustItemImportBindings,
) -> Option<&'a std::collections::HashSet<String>> {
    bindings.get(&(module_path.join("::"), local_name.to_string()))
}

fn nearest_rust_item_binding_depth(
    local_name: &str,
    module_path: &[String],
    bindings: &RustItemImportBindings,
) -> Option<usize> {
    nearest_rust_item_import_targets(local_name, module_path, bindings)
        .map(|(scope, _)| scope.len())
}

pub(crate) fn rust_glob_import_is_visible(
    module_path: &[String],
    bindings: &RustItemImportBindings,
) -> bool {
    nearest_rust_item_import_targets(&rust_glob_import_key(), module_path, bindings).is_some()
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum ResolvedRustItemPath {
    LocalModule(Vec<String>),
    LocalTrait(String),
    External(Vec<String>),
}

fn append_resolved_rust_path(
    resolved: ResolvedRustItemPath,
    tail: &[String],
    declared_trait_paths: &std::collections::HashSet<String>,
    bindings: &RustItemImportBindings,
    visiting: &mut std::collections::HashSet<(String, String)>,
) -> Option<ResolvedRustItemPath> {
    if tail.is_empty() {
        return Some(resolved);
    }
    match resolved {
        ResolvedRustItemPath::LocalModule(scope) => resolve_local_rust_path(
            &scope,
            tail,
            declared_trait_paths,
            bindings,
            visiting,
        ),
        ResolvedRustItemPath::External(mut path) => {
            path.extend(tail.iter().cloned());
            Some(ResolvedRustItemPath::External(path))
        }
        ResolvedRustItemPath::LocalTrait(_) => None,
    }
}

fn resolve_rust_import_target(
    target: &str,
    binding_scope: &[String],
    declared_trait_paths: &std::collections::HashSet<String>,
    bindings: &RustItemImportBindings,
    visiting: &mut std::collections::HashSet<(String, String)>,
) -> Option<ResolvedRustItemPath> {
    if let Some(local) = target.strip_prefix("@crate:") {
        let segments = local
            .split("::")
            .filter(|segment| !segment.is_empty())
            .map(str::to_string)
            .collect::<Vec<_>>();
        return resolve_local_rust_path(
            &[],
            &segments,
            declared_trait_paths,
            bindings,
            visiting,
        );
    }
    if let Some(external) = target.strip_prefix("::") {
        return Some(ResolvedRustItemPath::External(
            external
                .split("::")
                .filter(|segment| !segment.is_empty())
                .map(str::to_string)
                .collect(),
        ));
    }
    let segments = target
        .split("::")
        .filter(|segment| !segment.is_empty())
        .map(str::to_string)
        .collect::<Vec<_>>();
    resolve_relative_rust_path(
        &segments,
        binding_scope,
        declared_trait_paths,
        bindings,
        visiting,
    )
}

fn resolve_local_rust_path(
    scope: &[String],
    segments: &[String],
    declared_trait_paths: &std::collections::HashSet<String>,
    bindings: &RustItemImportBindings,
    visiting: &mut std::collections::HashSet<(String, String)>,
) -> Option<ResolvedRustItemPath> {
    let head = segments.first()?;
    let tail = &segments[1..];
    let imports = exact_rust_item_targets(head, scope, bindings);
    let module_targets = exact_rust_item_targets(
        &rust_local_module_shadow_key(head),
        scope,
        bindings,
    );
    let trait_targets = exact_rust_item_targets(
        &rust_local_trait_shadow_key(head),
        scope,
        bindings,
    );
    let present = usize::from(imports.is_some())
        + usize::from(module_targets.is_some())
        + usize::from(trait_targets.is_some());
    if present != 1 {
        return None;
    }
    let resolved = if let Some(targets) = imports {
        if targets.len() != 1 {
            return None;
        }
        let visit_key = (scope.join("::"), head.clone());
        if !visiting.insert(visit_key.clone()) {
            return None;
        }
        let resolved = resolve_rust_import_target(
            targets.iter().next()?,
            scope,
            declared_trait_paths,
            bindings,
            visiting,
        );
        visiting.remove(&visit_key);
        resolved?
    } else if let Some(targets) = module_targets {
        if targets.len() != 1 {
            return None;
        }
        ResolvedRustItemPath::LocalModule(
            targets
                .iter()
                .next()?
                .split("::")
                .map(str::to_string)
                .collect(),
        )
    } else {
        let target = trait_targets?.iter().next()?.clone();
        if !declared_trait_paths.contains(&target) {
            return None;
        }
        ResolvedRustItemPath::LocalTrait(target)
    };
    append_resolved_rust_path(resolved, tail, declared_trait_paths, bindings, visiting)
}

fn resolve_relative_rust_path(
    segments: &[String],
    module_path: &[String],
    declared_trait_paths: &std::collections::HashSet<String>,
    bindings: &RustItemImportBindings,
    visiting: &mut std::collections::HashSet<(String, String)>,
) -> Option<ResolvedRustItemPath> {
    let head = segments.first()?;
    let tail = &segments[1..];
    let candidates = [
        nearest_rust_item_import_targets(head, module_path, bindings)
            .map(|(scope, targets)| (scope, targets, 0u8)),
        nearest_rust_item_import_targets(
            &rust_local_module_shadow_key(head),
            module_path,
            bindings,
        )
        .map(|(scope, targets)| (scope, targets, 1u8)),
        nearest_rust_item_import_targets(
            &rust_local_trait_shadow_key(head),
            module_path,
            bindings,
        )
        .map(|(scope, targets)| (scope, targets, 2u8)),
    ];
    let best_depth = candidates
        .iter()
        .flatten()
        .map(|(scope, _, _)| scope.len())
        .max();
    let glob_depth = nearest_rust_item_binding_depth(
        &rust_glob_import_key(),
        module_path,
        bindings,
    );
    let Some(best_depth) = best_depth else {
        if glob_depth.is_some() {
            return None;
        }
        return Some(ResolvedRustItemPath::External(segments.to_vec()));
    };
    // A nearer glob can supply the same name. At the same scope an explicit
    // item or import wins over a glob, matching rustc's lexical precedence.
    if glob_depth.is_some_and(|depth| depth > best_depth) {
        return None;
    }
    let mut best = candidates
        .into_iter()
        .flatten()
        .filter(|(scope, _, _)| scope.len() == best_depth);
    let selected = best.next()?;
    if best.next().is_some() || selected.1.len() != 1 {
        return None;
    }
    let (scope, targets, kind) = selected;
    let resolved = match kind {
        0 => {
            let visit_key = (scope.join("::"), head.clone());
            if !visiting.insert(visit_key.clone()) {
                return None;
            }
            let resolved = resolve_rust_import_target(
                targets.iter().next()?,
                &scope,
                declared_trait_paths,
                bindings,
                visiting,
            );
            visiting.remove(&visit_key);
            resolved?
        }
        1 => ResolvedRustItemPath::LocalModule(
            targets
                .iter()
                .next()?
                .split("::")
                .map(str::to_string)
                .collect(),
        ),
        2 => {
            let target = targets.iter().next()?.clone();
            if !declared_trait_paths.contains(&target) {
                return None;
            }
            ResolvedRustItemPath::LocalTrait(target)
        }
        _ => unreachable!(),
    };
    append_resolved_rust_path(resolved, tail, declared_trait_paths, bindings, visiting)
}

fn resolve_rust_item_path(
    path: &syn::Path,
    module_path: &[String],
    declared_trait_paths: &std::collections::HashSet<String>,
    bindings: &RustItemImportBindings,
) -> Option<ResolvedRustItemPath> {
    let segments = path
        .segments
        .iter()
        .map(|segment| segment.ident.to_string())
        .collect::<Vec<_>>();
    if segments.is_empty() {
        return None;
    }
    let mut visiting = std::collections::HashSet::new();
    if path.leading_colon.is_some() {
        return Some(ResolvedRustItemPath::External(segments));
    }
    match segments.first().map(String::as_str) {
        Some("crate") => resolve_local_rust_path(
            &[],
            &segments[1..],
            declared_trait_paths,
            bindings,
            &mut visiting,
        ),
        Some("self" | "super") => {
            let mut scope = module_path.to_vec();
            let mut index = 0usize;
            while index < segments.len() {
                match segments[index].as_str() {
                    "self" => index += 1,
                    "super" => {
                        scope.pop();
                        index += 1;
                    }
                    _ => break,
                }
            }
            resolve_local_rust_path(
                &scope,
                &segments[index..],
                declared_trait_paths,
                bindings,
                &mut visiting,
            )
        }
        _ => resolve_relative_rust_path(
            &segments,
            module_path,
            declared_trait_paths,
            bindings,
            &mut visiting,
        ),
    }
}

pub(crate) fn resolve_external_rust_item_path(
    path: &syn::Path,
    module_path: &[String],
    declared_trait_paths: &std::collections::HashSet<String>,
    bindings: &RustItemImportBindings,
) -> Option<String> {
    match resolve_rust_item_path(path, module_path, declared_trait_paths, bindings)? {
        ResolvedRustItemPath::External(segments) => Some(segments.join("::")),
        ResolvedRustItemPath::LocalModule(_) | ResolvedRustItemPath::LocalTrait(_) => None,
    }
}

pub(crate) fn has_authenticated_cpp_inherit_attr(
    attrs: &[syn::Attribute],
    module_path: &[String],
    bindings: &RustItemImportBindings,
    authenticated_roots: &std::collections::HashSet<String>,
) -> bool {
    attrs.iter().any(|attribute| {
        // Route 1: a LIVE `#[cpp_inherit]` is honored only when it resolves
        // through the exact Rust import graph to the authenticated
        // `rusty::cpp_inherit` marker — a live attribute may be a proc macro
        // and is not authenticated by syntax alone.
        if attribute.path().is_ident("cpp_inherit") {
            return authenticated_roots.contains("rusty")
                && resolve_external_rust_item_path(
                    attribute.path(),
                    module_path,
                    &std::collections::HashSet::new(),
                    bindings,
                )
                .is_some_and(|path| path == "rusty::cpp_inherit");
        }
        // Route 2: the exact INERT spelling `#[cfg_attr(any(), cpp_inherit)]`
        // is compiler-owned by construction — its predicate is permanently
        // false, so rustc never resolves the payload and no proc macro can
        // occupy it. This is the same self-authentication argument the
        // `cpp_ctor` contract's inert form relies on.
        has_exact_inert_cpp_inherit_attr(attribute)
    })
}

/// The exact `#[cfg_attr(any(), cpp_inherit)]` spelling: a two-element
/// cfg_attr whose predicate is an empty `any()` and whose payload is the bare
/// `cpp_inherit` path. Anything qualified, nested, active, or multi-payload
/// is rejected.
fn has_exact_inert_cpp_inherit_attr(attribute: &syn::Attribute) -> bool {
    if !attribute.path().is_ident("cfg_attr") {
        return false;
    }
    let Ok(args) = attribute.parse_args_with(
        syn::punctuated::Punctuated::<syn::Meta, syn::Token![,]>::parse_terminated,
    ) else {
        return false;
    };
    if args.len() != 2 {
        return false;
    }
    let predicate_is_exact_inactive = matches!(args.first(), Some(syn::Meta::List(any))
        if any.path.is_ident("any") && any.tokens.is_empty());
    let payload_is_exact_marker = matches!(args.iter().nth(1), Some(syn::Meta::Path(path))
        if path.is_ident("cpp_inherit"));
    predicate_is_exact_inactive && payload_is_exact_marker
}

pub(crate) fn rust_item_import_name_is_bound(
    local_name: &str,
    module_path: &[String],
    bindings: &RustItemImportBindings,
) -> bool {
    nearest_rust_item_import_targets(local_name, module_path, bindings).is_some()
}

/// Resolve an impl's trait path to a crate-local lexical trait key.  Resolution
/// is intentionally fail-closed: explicit `crate`/`self`/`super` paths must
/// name a declared trait, and bare imports are resolved through their exact
/// binding rather than guessed from a same-leaf declaration.
pub(crate) fn resolve_declared_trait_path_key(
    path: &syn::Path,
    module_path: &[String],
    declared_trait_paths: &std::collections::HashSet<String>,
    import_bindings: &RustItemImportBindings,
) -> Option<String> {
    match resolve_rust_item_path(path, module_path, declared_trait_paths, import_bindings)? {
        ResolvedRustItemPath::LocalTrait(key) if declared_trait_paths.contains(&key) => Some(key),
        ResolvedRustItemPath::LocalModule(_) | ResolvedRustItemPath::External(_)
        | ResolvedRustItemPath::LocalTrait(_) => None,
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

/// Like `collect_declared_trait_methods`, but only methods WITH a default
/// body. Feeds the manifest's `trait_default_methods` so a consumer's
/// static-dispatch fallback (`<Trait>RuntimeHelper::method<Owner>`) is only
/// emitted for methods the helper actually carries — the helper is built
/// from default bodies, so a REQUIRED method (serde's `Error::custom`) has
/// no helper member, and the fallback branch is a hard parse error (the
/// helper type is non-dependent, so clang checks the never-taken branch
/// eagerly).
/// Trait name → ALL associated-type names it declares (bounds not required —
/// `type Ok;` counts). Feeds the manifest's `trait_assoc_type_names`.
pub fn collect_trait_assoc_type_names(
    items: &[syn::Item],
) -> std::collections::BTreeMap<String, Vec<String>> {
    fn walk(
        items: &[syn::Item],
        out: &mut std::collections::BTreeMap<String, Vec<String>>,
    ) {
        for item in items {
            match item {
                syn::Item::Trait(t) => {
                    let entry = out.entry(t.ident.to_string()).or_insert_with(Vec::new);
                    for ti in &t.items {
                        if let syn::TraitItem::Type(at) = ti {
                            let name = at.ident.to_string();
                            if !entry.contains(&name) {
                                entry.push(name);
                            }
                        }
                    }
                }
                syn::Item::Mod(m) => {
                    if let Some((_, nested)) = &m.content {
                        walk(nested, out);
                    }
                }
                _ => {}
            }
        }
    }
    let mut out = std::collections::BTreeMap::new();
    walk(items, &mut out);
    out
}

pub fn collect_trait_default_methods(
    items: &[syn::Item],
) -> std::collections::BTreeMap<String, Vec<String>> {
    let mut out = std::collections::BTreeMap::new();
    collect_trait_default_methods_into(items, &mut out);
    out
}

fn collect_trait_default_methods_into(
    items: &[syn::Item],
    out: &mut std::collections::BTreeMap<String, Vec<String>>,
) {
    for item in items {
        match item {
            syn::Item::Trait(t) => {
                let entry = out.entry(t.ident.to_string()).or_insert_with(Vec::new);
                for ti in &t.items {
                    if let syn::TraitItem::Fn(f) = ti
                        && f.default.is_some()
                    {
                        let name = f.sig.ident.to_string();
                        if !entry.contains(&name) {
                            entry.push(name);
                        }
                    }
                }
            }
            syn::Item::Mod(m) => {
                if let Some((_, nested)) = &m.content {
                    collect_trait_default_methods_into(nested, out);
                }
            }
            _ => {}
        }
    }
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
                if module_is_cfg_disabled(m) {
                    continue;
                }
                if let Some((_, nested)) = &m.content {
                    collect_declared_trait_methods_into(nested, out);
                }
            }
            _ => {}
        }
    }
}

/// `Trait::method` → the first `Self::X` projection ident in the declared
/// return type. Token-level scan: `Result < Self :: SerializeMap , ... >`
/// yields `SerializeMap` (the payload projection comes first in Result/Option
/// spellings, which is the case this feeds).
pub fn collect_trait_method_return_assocs(
    items: &[syn::Item],
) -> std::collections::BTreeMap<String, String> {
    fn first_self_projection(ty: &syn::Type) -> Option<String> {
        // For a `Result<A, B>` return, only the OK position feeds a
        // `let x = call()?` binding — scanning the whole type made
        // `next_entry` (`Result<Option<(K, V)>, Self::Error>`) record
        // `Error`, which then leaked into the call's V template arg
        // (leaf5202). Recurse into A; the Err arg never types the binding.
        if let syn::Type::Path(tp) = ty
            && let Some(seg) = tp.path.segments.last()
            && seg.ident == "Result"
            && let syn::PathArguments::AngleBracketed(ab) = &seg.arguments
            && let Some(syn::GenericArgument::Type(ok_ty)) = ab.args.first()
        {
            return first_self_projection(ok_ty);
        }
        let text = quote::ToTokens::to_token_stream(ty).to_string();
        let idx = text.find("Self :: ")?;
        let rest = &text[idx + "Self :: ".len()..];
        let ident: String = rest
            .chars()
            .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
            .collect();
        if ident.is_empty() { None } else { Some(ident) }
    }
    fn walk(items: &[syn::Item], out: &mut std::collections::BTreeMap<String, String>) {
        for item in items {
            match item {
                syn::Item::Trait(t) => {
                    for ti in &t.items {
                        if let syn::TraitItem::Fn(f) = ti
                            && let syn::ReturnType::Type(_, ret) = &f.sig.output
                            && let Some(assoc) = first_self_projection(ret)
                        {
                            out.entry(format!("{}::{}", t.ident, f.sig.ident))
                                .or_insert(assoc);
                        }
                    }
                }
                syn::Item::Mod(m) => {
                    if let Some((_, nested)) = &m.content {
                        walk(nested, out);
                    }
                }
                _ => {}
            }
        }
    }
    let mut out = std::collections::BTreeMap::new();
    walk(items, &mut out);
    out
}

/// `Trait::Assoc` → short name of the assoc type's first non-marker trait
/// bound, from this crate's trait DECLARATIONS. Marker-ish bounds that never
/// own the methods being routed (Sized/Send/Sync/Clone/Copy/Debug + lifetimes)
/// are skipped so `type Item: Clone + Iterator` still records `Iterator`.
pub fn collect_trait_assoc_type_bounds(
    items: &[syn::Item],
) -> std::collections::BTreeMap<String, String> {
    fn walk(items: &[syn::Item], out: &mut std::collections::BTreeMap<String, String>) {
        for item in items {
            match item {
                syn::Item::Trait(t) => {
                    for ti in &t.items {
                        if let syn::TraitItem::Type(at) = ti {
                            for b in &at.bounds {
                                if let syn::TypeParamBound::Trait(tb) = b
                                    && let Some(seg) = tb.path.segments.last()
                                {
                                    let short = seg.ident.to_string();
                                    if matches!(
                                        short.as_str(),
                                        "Sized" | "Send" | "Sync" | "Clone" | "Copy" | "Debug"
                                    ) {
                                        continue;
                                    }
                                    out.entry(format!("{}::{}", t.ident, at.ident))
                                        .or_insert(short);
                                    break;
                                }
                            }
                        }
                    }
                }
                syn::Item::Mod(m) => {
                    if let Some((_, nested)) = &m.content {
                        walk(nested, out);
                    }
                }
                _ => {}
            }
        }
    }
    let mut out = std::collections::BTreeMap::new();
    walk(items, &mut out);
    out
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
    collect_concrete_trait_impl_method_owners_excluding_traits(
        items,
        declared_traits,
        &std::collections::HashSet::new(),
    )
}

pub fn collect_concrete_trait_impl_method_owners_excluding_traits(
    items: &[syn::Item],
    declared_traits: &std::collections::HashSet<String>,
    excluded_traits: &std::collections::HashSet<String>,
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
    let declared_trait_paths = collect_declared_trait_paths(items);
    let import_bindings = collect_rust_item_import_bindings(items);
    let mut assoc_const_traits = std::collections::HashSet::new();
    collect_assoc_const_trait_names_into(items, &[], &mut assoc_const_traits);
    let mut out: HashMap<String, std::collections::BTreeSet<String>> = HashMap::new();
    collect_concrete_trait_impl_method_owners_into(
        items,
        declared_traits,
        &declared_trait_paths,
        &import_bindings,
        &assoc_const_traits,
        excluded_traits,
        &[],
        &mut out,
    );
    out
}

fn collect_assoc_const_trait_names_into(
    items: &[syn::Item],
    module_path: &[String],
    out: &mut std::collections::HashSet<String>,
) {
    for item in items {
        match item {
            syn::Item::Trait(t) => {
                if t.items.iter().any(|ti| matches!(ti, syn::TraitItem::Const(_))) {
                    let trait_name = t.ident.to_string();
                    out.insert(if module_path.is_empty() {
                        trait_name
                    } else {
                        format!("{}::{}", module_path.join("::"), trait_name)
                    });
                }
            }
            syn::Item::Mod(m) => {
                if module_is_cfg_disabled(m) {
                    continue;
                }
                if let Some((_, nested)) = &m.content {
                    let mut nested_path = module_path.to_vec();
                    nested_path.push(m.ident.to_string());
                    collect_assoc_const_trait_names_into(nested, &nested_path, out);
                }
            }
            _ => {}
        }
    }
}

fn collect_concrete_trait_impl_method_owners_into(
    items: &[syn::Item],
    declared_traits: &std::collections::HashSet<String>,
    declared_trait_paths: &std::collections::HashSet<String>,
    import_bindings: &RustItemImportBindings,
    assoc_const_traits: &std::collections::HashSet<String>,
    excluded_traits: &std::collections::HashSet<String>,
    module_path: &[String],
    out: &mut HashMap<String, std::collections::BTreeSet<String>>,
) {
    for item in items {
        match item {
            syn::Item::Impl(impl_block) => {
                let Some((_, trait_path, _)) = &impl_block.trait_ else {
                    continue;
                };
                let Some(written_trait_name) =
                    trait_path.segments.last().map(|s| s.ident.to_string())
                else {
                    continue;
                };
                let Some(trait_key) = resolve_declared_trait_path_key(
                    trait_path,
                    module_path,
                    declared_trait_paths,
                    import_bindings,
                ) else {
                    continue;
                };
                let trait_name = trait_key
                    .rsplit("::")
                    .next()
                    .unwrap_or(&written_trait_name)
                    .to_string();
                // Only crate-declared traits (foreign-trait impls aren't UFCS-
                // lowered), skip assoc-const (runtime-helper) traits, and only
                // concrete impls (no type-param generics) — generic/blanket
                // impls don't reliably emit an early-declared `<Tr>_`.
                if !declared_traits.contains(&trait_name)
                    || assoc_const_traits.contains(&trait_key)
                    || excluded_traits.contains(&trait_key)
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
                let trait_key = if module_path.is_empty() {
                    trait_name.clone()
                } else {
                    format!("{}::{}", module_path.join("::"), trait_name)
                };
                if !assoc_const_traits.contains(&trait_key)
                    && !excluded_traits.contains(&trait_key)
                {
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
                if module_is_cfg_disabled(m) {
                    continue;
                }
                if let Some((_, nested)) = &m.content {
                    let mut nested_path = module_path.to_vec();
                    nested_path.push(m.ident.to_string());
                    collect_concrete_trait_impl_method_owners_into(
                        nested,
                        declared_traits,
                        declared_trait_paths,
                        import_bindings,
                        assoc_const_traits,
                        excluded_traits,
                        &nested_path,
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
    declared_trait_paths: &std::collections::HashSet<String>,
    import_bindings: &RustItemImportBindings,
    excluded_traits: &std::collections::HashSet<String>,
    module_path: &[String],
    inherent: &mut std::collections::HashSet<String>,
    trait_named: &mut std::collections::HashSet<String>,
) {
    for item in items {
        match item {
            syn::Item::Impl(impl_block) => {
                // A trait impl counts as a *trait* use only when the implemented
                // trait is crate-declared (see `classify_method_names`).
                let impl_trait_key = impl_block.trait_.as_ref().and_then(|(_, path, _)| {
                    resolve_declared_trait_path_key(
                        path,
                        module_path,
                        declared_trait_paths,
                        import_bindings,
                    )
                });
                let is_crate_trait_impl = impl_trait_key
                    .as_ref()
                    .is_some_and(|key| !excluded_traits.contains(key));
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
                let trait_name = t.ident.to_string();
                let trait_key = if module_path.is_empty() {
                    trait_name
                } else {
                    format!("{}::{}", module_path.join("::"), trait_name)
                };
                if excluded_traits.contains(&trait_key) {
                    continue;
                }
                for trait_item in &t.items {
                    if let syn::TraitItem::Fn(method) = trait_item {
                        trait_named.insert(method.sig.ident.to_string());
                    }
                }
            }
            syn::Item::Mod(m) => {
                if module_is_cfg_disabled(m) {
                    continue;
                }
                if let Some((_, nested)) = &m.content {
                    let mut nested_path = module_path.to_vec();
                    nested_path.push(m.ident.to_string());
                    collect_method_name_uses(
                        nested,
                        declared_trait_paths,
                        import_bindings,
                        excluded_traits,
                        &nested_path,
                        inherent,
                        trait_named,
                    );
                }
            }
            _ => {}
        }
    }
}

impl Default for TranspileOptions {
    fn default() -> Self {
        Self {
            crate_namespace_wrap: false,
            // Outside the umbrella's re-export closure = ordinary consumer.
            in_umbrella_closure: false,
            lenient_auto_template_args: false,
            by_value_cycle_breaking_prototype: false,
            is_dependency: false,
            cpp_module_symbol_index: None,
            cpp_module_symbol_index_sources: Vec::new(),
            consumer_module_map: ConsumerModuleMap::default(),
            consumer_rust_module: None,
            external_crate_module_aliases: HashMap::new(),
            authenticated_cpp_inherit_roots: std::collections::HashSet::new(),
            cpp_name_trusted_cpp_inherit_provenance: false,
            authenticated_sysroot_roots: std::collections::HashSet::from([
                "std".to_string(),
                "core".to_string(),
            ]),
            cross_file_rust_item_import_bindings: RustItemImportBindings::new(),
            cpp_type_aliases: HashMap::new(),
            emit_ufcs_trait_manifest_path: None,
            dependency_ufcs_trait_manifests: Vec::new(),
            use_import_std_in_modules: false,
            explicit_gmf_includes: Vec::new(),
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
            cross_file_traits: Vec::new(),
            cross_file_cpp_name_targets: std::collections::BTreeMap::new(),
            cross_file_cpp_inherit: Vec::new(),
            cross_file_impl_blocks: Vec::new(),
            cross_file_structs: Vec::new(),
            cross_file_type_aliases: Vec::new(),
            flat_import_type_authorizations: BTreeSet::new(),
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
    validate_cpp_module_symbol_index_contract(&merged, index_paths)?;
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
        validate_cpp_qualified_name(&module_path, "::", "C++ interop binding path")
            .map_err(|error| {
                format!(
                    "C++ module symbol index {} contains {}",
                    source_path.display(),
                    error
                )
            })?;
        validate_cpp_qualified_name(&module.cpp_module, ".", "C++ module name").map_err(
            |error| {
                format!(
                    "C++ module symbol index {} entry '{}' contains {}",
                    source_path.display(),
                    module_path,
                    error
                )
            },
        )?;
        // The namespace field is validated by canonicalization below, which
        // deliberately accepts (and trims) whitespace-formatted input from
        // heterogeneous index producers before rejecting degenerate forms.
        let namespace = module
            .namespace
            .map(|namespace| {
                canonical_cpp_export_namespace_path(&namespace).map_err(|detail| {
                    format!(
                        "C++ module symbol index {} has invalid namespace for module '{}': {}",
                        source_path.display(),
                        module_path,
                        detail
                    )
                })
            })
            .transpose()?;
        let incoming = CppModuleIndexModule {
            cpp_module: module.cpp_module,
            namespace,
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
    if existing.cpp_module != incoming.cpp_module {
        return Err(format!(
            "C++ module symbol index {} has conflicting C++ module name for binding '{}': '{}' vs '{}'",
            source_path.display(),
            module_path,
            existing.cpp_module,
            incoming.cpp_module
        ));
    }
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

fn canonical_cpp_export_namespace_path(path: &str) -> Result<String, String> {
    let path = path.trim();
    if path.is_empty() {
        return Err("namespace must not be empty".to_string());
    }
    if path.starts_with("::") {
        return Err("leading `::` is not supported".to_string());
    }

    let mut segments = Vec::new();
    for (segment_index, raw_segment) in path.split("::").enumerate() {
        let segment = raw_segment.trim();
        if segment.is_empty() {
            return Err("namespace contains an empty identifier segment".to_string());
        }
        let mut chars = segment.chars();
        let Some(first) = chars.next() else {
            return Err("namespace contains an empty identifier segment".to_string());
        };
        if !(first.is_ascii_alphabetic() || first == '_')
            || !chars.all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
        {
            return Err(format!(
                "namespace segment '{}' is not a C++ identifier",
                segment
            ));
        }
        let reserved_identifier = crate::codegen::escape_cpp_keyword(segment) != segment
            || segment.contains("__")
            || (segment_index == 0 && segment.starts_with('_'))
            || segment
                .strip_prefix('_')
                .and_then(|rest| rest.chars().next())
                .is_some_and(|ch| ch.is_ascii_uppercase());
        if reserved_identifier {
            return Err(format!(
                "namespace segment '{}' is reserved in C++",
                segment
            ));
        }
        segments.push(segment);
    }
    Ok(segments.join("::"))
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

fn collect_cpp_module_namespace_map(index: &CppModuleSymbolIndex) -> HashMap<String, String> {
    index
        .modules
        .iter()
        .filter_map(|(module_path, module)| {
            module
                .namespace
                .as_ref()
                .map(|namespace| (module_path.clone(), namespace.clone()))
        })
        .collect()
}

fn collect_cpp_module_import_name_map(index: &CppModuleSymbolIndex) -> HashMap<String, String> {
    index
        .modules
        .iter()
        .map(|(binding_path, module)| (binding_path.clone(), module.cpp_module.clone()))
        .collect()
}

fn validate_cpp_module_symbol_index_contract(
    index: &CppModuleSymbolIndex,
    index_sources: &[PathBuf],
) -> Result<(), String> {
    let source_label = format_cpp_module_index_sources(index_sources);
    let mut cpp_modules = HashSet::new();
    for (binding_path, module) in &index.modules {
        validate_cpp_qualified_name(binding_path, "::", "C++ interop binding path").map_err(
            |error| format!("C++ module symbol index {source_label} contains {error}"),
        )?;
        validate_cpp_qualified_name(&module.cpp_module, ".", "C++ module name").map_err(
            |error| {
                format!(
                    "C++ module symbol index {source_label} entry '{binding_path}' contains {error}"
                )
            },
        )?;
        if let Some(namespace) = module.namespace.as_deref() {
            validate_cpp_qualified_name(namespace, "::", "C++ namespace").map_err(|error| {
                format!(
                    "C++ module symbol index {source_label} entry '{binding_path}' contains {error}"
                )
            })?;
        }
        if !cpp_modules.insert(module.cpp_module.as_str()) {
            return Err(format!(
                "C++ module symbol index {source_label} repeats C++ module '{}' across binding entries",
                module.cpp_module
            ));
        }
    }
    Ok(())
}

fn collect_cpp_module_export_namespace_map(
    index: &CppModuleSymbolIndex,
) -> Result<HashMap<String, String>, String> {
    let mut by_module = HashMap::new();
    for (module_path, module_entry) in &index.modules {
        let Some(namespace) = module_entry.namespace.as_ref() else {
            continue;
        };
        let namespace = canonical_cpp_export_namespace_path(namespace).map_err(|detail| {
            format!(
                "C++ module symbol index has invalid namespace for module '{}': {}",
                module_path, detail
            )
        })?;
        by_module.insert(module_path.clone(), namespace);
    }
    Ok(by_module)
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
        // Was hard-coded `None`, which is why the `--crate` path never got the
        // requalifying namespace wrap. Opt-in via TranspileOptions so alloc and
        // path (same entry points, unwrapped, matrix-green) are unaffected.
        // On this path the module name IS the crate name (main.rs:253 -> :328).
        if options.crate_namespace_wrap { module_name } else { None },
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
    transpile_full_with_options_impl(
        rust_source,
        module_name,
        type_map,
        extension_method_hints,
        crate_name,
        options,
        None,
    )
}

/// Validate the narrow `extern "Rust"` seam used by named C++ modules whose
/// definitions live in a C++ module implementation unit.
///
/// A Rust-ABI foreign declaration has no literal C++ linkage-specification
/// equivalent: `extern "Rust"` is not a C++ language linkage.  In named-module
/// output we deliberately lower it to an ordinary module-attached C++
/// declaration.  Outside a named module there is no implementation-unit
/// ownership contract to bind that declaration to, so fail before emitting an
/// invalid or silently different ABI surface.
fn validate_rust_abi_foreign_declarations(
    file: &syn::File,
    module_name: Option<&str>,
) -> Result<(), String> {
    struct Validator<'a> {
        module_name: Option<&'a str>,
        block_depth: usize,
        error: Option<String>,
    }

    impl<'ast> syn::visit::Visit<'ast> for Validator<'_> {
        fn visit_block(&mut self, block: &'ast syn::Block) {
            self.block_depth += 1;
            syn::visit::visit_block(self, block);
            self.block_depth -= 1;
        }

        fn visit_item_foreign_mod(&mut self, foreign: &'ast syn::ItemForeignMod) {
            if self.error.is_some()
                || foreign.abi.name.as_ref().map(syn::LitStr::value).as_deref() != Some("Rust")
            {
                syn::visit::visit_item_foreign_mod(self, foreign);
                return;
            }

            if self.module_name.is_none() {
                self.error = Some(
                    "`extern \"Rust\"` declarations require named C++ module output".to_string(),
                );
                return;
            }
            if foreign.unsafety.is_none() {
                self.error = Some(
                    "`extern \"Rust\"` declarations must use an `unsafe extern` block".to_string(),
                );
                return;
            }
            if self.block_depth != 0 {
                self.error = Some(
                    "`extern \"Rust\"` declarations are only supported at module scope".to_string(),
                );
                return;
            }

            for item in &foreign.items {
                let syn::ForeignItem::Fn(function) = item else {
                    self.error = Some(
                        "named-module `extern \"Rust\"` supports function declarations only"
                            .to_string(),
                    );
                    return;
                };
                if function.sig.variadic.is_some() {
                    self.error = Some(format!(
                        "named-module `extern \"Rust\"` function `{}` cannot be variadic",
                        function.sig.ident
                    ));
                    return;
                }
                if function.attrs.iter().any(|attr| {
                    attr.path().is_ident("link_name") || attr.path().is_ident("link_ordinal")
                }) {
                    self.error = Some(format!(
                        "named-module `extern \"Rust\"` function `{}` cannot override its link name",
                        function.sig.ident
                    ));
                    return;
                }
            }

            syn::visit::visit_item_foreign_mod(self, foreign);
        }
    }

    let mut validator = Validator {
        module_name,
        block_depth: 0,
        error: None,
    };
    syn::visit::Visit::visit_file(&mut validator, file);
    validator.error.map_or(Ok(()), Err)
}

/// Render a cpp_abi file that was already collected, globally validated, and
/// lowered by the ordered inline-block preflight.  This seam is intentionally
/// crate-private: ordinary standalone callers must continue through
/// `transpile_full_with_options`, which rejects module-less ABI facades.
pub(crate) fn transpile_prepared_inline_cpp_abi(
    file: syn::File,
    plan: crate::cpp_abi::CppAbiEmissionPlan,
    type_map: &UserTypeMap,
    extension_method_hints: &HashSet<String>,
    options: &TranspileOptions,
) -> Result<String, String> {
    if !options.inline_rust_block {
        return Err("prepared cpp_abi rendering requires inline-rust code generation".to_string());
    }
    transpile_full_with_options_impl(
        "",
        None,
        type_map,
        extension_method_hints,
        None,
        options,
        Some((file, plan)),
    )
}

fn transpile_full_with_options_impl(
    rust_source: &str,
    module_name: Option<&str>,
    type_map: &UserTypeMap,
    extension_method_hints: &HashSet<String>,
    crate_name: Option<&str>,
    options: &TranspileOptions,
    prepared_cpp_abi: Option<(syn::File, crate::cpp_abi::CppAbiEmissionPlan)>,
) -> Result<String, String> {
    validate_explicit_gmf_includes(&options.explicit_gmf_includes)?;
    if module_name.is_none() && !options.explicit_gmf_includes.is_empty() {
        return Err(
            "Explicit GMF includes require module output (provide a C++ module name)".to_string(),
        );
    }

    let consumer_rust_module = match options.consumer_rust_module.as_deref() {
        None => None,
        Some(raw) => {
            if options.consumer_module_map.is_empty() {
                return Err(
                    "--consumer-rust-module requires --consumer-module-map".to_string(),
                );
            }
            let current_cpp_module = module_name.ok_or_else(|| {
                "--consumer-rust-module requires module emission; pass --module-name <name>"
                    .to_string()
            })?;
            let canonical = canonical_consumer_rust_module_path(raw)?;
            if let Some(entry) = options
                .consumer_module_map
                .entry_for_rust_module(&canonical)
                && entry.cpp_module != current_cpp_module
            {
                return Err(format!(
                    "--consumer-rust-module '{}' maps to C++ module '{}', not current module '{}'",
                    raw, entry.cpp_module, current_cpp_module
                ));
            }
            Some(canonical)
        }
    };
    let effective_cxx_namespace = if options.consumer_module_map.is_empty() {
        options.cxx_namespace.clone()
    } else {
        let current_cpp_module = module_name.ok_or_else(|| {
            "--consumer-module-map requires module emission; pass --module-name <name>"
                .to_string()
        })?;
        let entry = options
            .consumer_module_map
            .entry_for_cpp_module(current_cpp_module)
            .ok_or_else(|| {
                format!(
                    "C++ module '{}' has no entry in --consumer-module-map",
                    current_cpp_module
                )
            })?;
        if options.auto_namespace {
            return Err(
                "--consumer-module-map cannot be combined with --auto-namespace; the map supplies the C++ namespace"
                    .to_string(),
            );
        }
        if let Some(explicit) = options.cxx_namespace.as_deref()
            && explicit != entry.cpp_namespace
        {
            return Err(format!(
                "--cxx-namespace '{}' conflicts with namespace '{}' mapped for C++ module '{}'",
                explicit, entry.cpp_namespace, current_cpp_module
            ));
        }
        Some(entry.cpp_namespace.clone())
    };
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
    let is_prepared_inline = prepared_cpp_abi.is_some();
    let validate_cpp_defaults = |file: &syn::File| {
        if options.crate_module_names.is_empty() {
            crate::cpp_default_args::validate_file(file, type_map)
        } else {
            crate::cpp_default_args::validate_file_after_crate_preflight(file, type_map)
        }
    };
    let (mut file, cpp_abi_plan, has_cpp_defaults) = if let Some((file, plan)) = prepared_cpp_abi {
        let has_cpp_defaults = validate_cpp_defaults(&file)?;
        (file, plan, has_cpp_defaults)
    } else {
        let file: syn::File = parse_with_expand_hygiene_fallback(rust_source)
            .map_err(|e| format!("Parse error: {}", e))?;
        log_profile("parse_with_expand_hygiene_fallback");
        let has_cpp_defaults = validate_cpp_defaults(&file)?;
        match crate::cpp_abi::lower(&file)? {
            Some((lowered, plan)) => (lowered, plan, has_cpp_defaults),
            None => (
                file,
                crate::cpp_abi::CppAbiEmissionPlan::default(),
                has_cpp_defaults,
            ),
        }
    };
    if has_cpp_defaults && is_prepared_inline {
        return Err(
            "cpp_default_argument is supported only by source files in named-module crate mode, not inline Rust blocks"
                .to_string(),
        );
    }
    if has_cpp_defaults && module_name.is_none() {
        return Err(
            "cpp_default_argument requires named C++ module output so the default belongs to an exported declaration"
                .to_string(),
        );
    }
    if has_cpp_defaults {
        crate::cpp_default_args::validate_required_gmf_includes(
            &file,
            &options.explicit_gmf_includes,
        )?;
    }
    let cpp_name_plan = if options.crate_module_names.is_empty() {
        crate::cpp_name::collect(&file)?
    } else {
        crate::cpp_name::collect_with_crate_provenance(
            &file,
            options.cpp_name_trusted_cpp_inherit_provenance,
        )?
    };
    if !cpp_name_plan.is_empty()
        && (module_name.is_none() || is_prepared_inline || options.inline_rust_block)
    {
        return Err(
            "cpp_name requires named-module or crate-mode output and is not supported in inline/module-less transpilation"
                .to_string(),
        );
    }
    if !is_prepared_inline {
        cpp_abi_plan.validate_flat_import_namespace(
            options.cxx_namespace.as_deref(),
            "crate/module transpilation",
        )?;
    }
    validate_rust_abi_foreign_declarations(&file, module_name)?;
    if cpp_abi_plan.has_flat_imports()
        && !is_prepared_inline
        && options.crate_module_names.is_empty()
    {
        return Err(
            "cpp_import_namespace requires prepared crate mode or prepared inline-rust mode; direct named-module transpilation cannot prove the physical sibling import"
                .to_string(),
        );
    }
    if !cpp_abi_plan.is_empty() && module_name.is_none() && !is_prepared_inline {
        return Err(
            "cpp_abi adapters require named C++ module output; standalone output is unsupported"
                .to_string(),
        );
    }
    log_profile("cpp_abi_lower");
    validate_cpp_declaration_markers(&file)?;
    log_profile("validate_cpp_declaration_markers");
    validate_reserved_cpp_marker_names(&file)?;
    log_profile("validate_reserved_cpp_marker_names");
    let has_cpp_module_imports = file_contains_cpp_module_imports(&file);
    log_profile("file_contains_cpp_module_imports");
    if has_cpp_module_imports {
        // `cpp` is a reserved interop root once this file contains a
        // `use cpp::...` binding.  Native Rust fixtures commonly carry a
        // top-level inline `mod cpp { ... }` solely to make cargo type-check;
        // it is not part of the C++ surface.  Require both conditions so an
        // ordinary Rust module named `cpp` remains an ordinary emitted module.
        file.items.retain(|item| {
            !matches!(item, syn::Item::Mod(module)
                if module.ident == "cpp" && module.content.is_some())
        });
    }
    if has_cpp_module_imports {
        match options.cpp_module_symbol_index.as_ref() {
            Some(index) if !index.modules.is_empty() => {
                validate_cpp_module_symbol_index_contract(
                    index,
                    &options.cpp_module_symbol_index_sources,
                )?;
            }
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
    codegen.set_authenticated_cpp_inherit_roots(
        options.authenticated_cpp_inherit_roots.clone(),
    );
    codegen.set_authenticated_sysroot_roots(options.authenticated_sysroot_roots.clone());
    codegen.set_cross_file_rust_item_import_bindings(
        options.cross_file_rust_item_import_bindings.clone(),
    );
    codegen.set_cpp_type_aliases(options.cpp_type_aliases.clone());
    codegen.set_use_import_std_in_modules(options.use_import_std_in_modules);
    codegen.set_explicit_module_gmf_includes(
        options
            .explicit_gmf_includes
            .iter()
            .map(GmfIncludeSpec::render)
            .collect(),
    );
    codegen.set_in_umbrella_closure(options.in_umbrella_closure);
    codegen.lenient_auto_template_args = options.lenient_auto_template_args;
    codegen.set_cxx_namespace(effective_cxx_namespace);
    codegen.set_auto_namespace(options.auto_namespace);
    codegen.set_prefer_rusty_unit_alias(options.prefer_rusty_unit_alias);
    codegen.set_prefer_rusty_view_aliases(options.prefer_rusty_view_aliases);
    codegen.set_interface_traits(options.interface_traits);
    codegen.inline_rust_block = options.inline_rust_block;
    codegen.set_cross_file_enums(options.cross_file_enums.clone());
    codegen.set_cross_file_traits(&options.cross_file_traits);
    codegen.set_cross_file_cpp_name_targets(options.cross_file_cpp_name_targets.clone());
    codegen.set_cross_file_cpp_inherit(options.cross_file_cpp_inherit.clone());
    codegen.set_cross_file_impl_blocks(options.cross_file_impl_blocks.clone());
    codegen.set_cross_file_structs(options.cross_file_structs.clone());
    codegen.set_cross_file_type_aliases(options.cross_file_type_aliases.clone());
    codegen.set_flat_import_type_authorizations(
        options.flat_import_type_authorizations.clone(),
    );
    codegen.set_crate_module_names(options.crate_module_names.clone());
    codegen.set_consumer_module_map(
        options.consumer_module_map.clone(),
        module_name,
        consumer_rust_module.as_deref(),
    );
    codegen.set_cpp_abi_plan(cpp_abi_plan);
    codegen.set_cpp_name_plan(cpp_name_plan);
    if let Some(index) = options.cpp_module_symbol_index.as_ref() {
        let member_symbols = collect_cpp_module_member_symbol_map(index);
        codegen.set_cpp_module_member_symbols(member_symbols);
        codegen.set_cpp_module_namespaces(collect_cpp_module_namespace_map(index));
        codegen.set_cpp_module_import_names(collect_cpp_module_import_name_map(index));
        let export_namespaces = collect_cpp_module_export_namespace_map(index)?;
        codegen.set_cpp_module_export_namespaces(export_namespaces);
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
    if let Some(error) = codegen.take_codegen_error() {
        return Err(error);
    }
    // UFCS cross-crate: emit this crate's trait manifest (declared traits +
    // actually-emitted `<Tr>_::m` owner map) for dependents to consume.
    if let Some(path) = options.emit_ufcs_trait_manifest_path.as_ref() {
        let manifest = codegen.build_ufcs_trait_manifest(module_name.unwrap_or(""));
        if let Ok(json) = serde_json::to_string_pretty(&manifest) {
            let _ = fs::write(path, json);
        }
    }
    let mut output_str = codegen.into_output();
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
            if is_defaulted_declarator && prev_trimmed.as_ref().is_some_and(|prev| prev == &trimmed)
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

/// Validate the deliberately narrow declaration-only ownership marker before
/// codegen. A marked body remains native Rust; C++ receives only the ordinary
/// declaration emitted by the existing forward-declaration pass.
/// Every reserved `cpp_*` marker name this compiler honors, in any of its
/// spellings. The inert `#[cfg_attr(any(), cpp_*)]` carrier exists precisely
/// so contracts survive rustc unseen — which also means a MISSPELLED or
/// not-yet-ported contract would otherwise vanish silently. Fail closed: an
/// unknown `cpp_*` name inside an inert carrier is a hard error, never a
/// silent no-op.
const KNOWN_CPP_MARKER_NAMES: &[&str] = &[
    "cpp_abi",
    "cpp_abi_alias",
    "cpp_abi_core",
    "cpp_ctor",
    "cpp_declaration",
    "cpp_default_argument",
    "cpp_explicit",
    "cpp_import_namespace",
    "cpp_inherit",
    "cpp_internal",
    "cpp_marker_impl",
    "cpp_marker_trait",
    "cpp_name",
    "cpp_namespace",
    "cpp_no_auto_traits",
    "cpp_noexcept",
    "cpp_no_fieldwise_ctor",
    "cpp_trait_member_dispatch",
];

/// Reject `#[cfg_attr(any(), <payload>)]` carriers whose payload names an
/// UNKNOWN `cpp_*` marker. Only the permanently-inactive `any()` predicate is
/// policed: an active `cfg_attr` payload is ordinary Rust that rustc itself
/// resolves, while the inert spelling is compiler-owned and has exactly one
/// legitimate use — carrying a contract this compiler knows.
fn validate_reserved_cpp_marker_names(file: &syn::File) -> Result<(), String> {
    struct Validator {
        error: Option<String>,
    }

    impl Validator {
        fn check_attribute(&mut self, attribute: &syn::Attribute) {
            if self.error.is_some() || !attribute.path().is_ident("cfg_attr") {
                return;
            }
            let Ok(args) = attribute.parse_args_with(
                syn::punctuated::Punctuated::<syn::Meta, syn::Token![,]>::parse_terminated,
            ) else {
                return;
            };
            let predicate_is_exact_inactive = matches!(args.first(), Some(syn::Meta::List(any))
                if any.path.is_ident("any") && any.tokens.is_empty());
            if !predicate_is_exact_inactive {
                return;
            }
            for payload in args.iter().skip(1) {
                let Some(name) = payload.path().segments.last().map(|s| s.ident.to_string())
                else {
                    continue;
                };
                if name.starts_with("cpp_") && !KNOWN_CPP_MARKER_NAMES.contains(&name.as_str())
                {
                    self.error = Some(format!(
                        "unknown reserved marker `{name}` in `#[cfg_attr(any(), {name})]`: \
                         this compiler does not implement such a contract, and the inert \
                         spelling would otherwise discard it silently. Known cpp_* markers: \
                         {}",
                        KNOWN_CPP_MARKER_NAMES.join(", ")
                    ));
                    return;
                }
            }
        }
    }

    impl<'ast> syn::visit::Visit<'ast> for Validator {
        fn visit_attribute(&mut self, attribute: &'ast syn::Attribute) {
            self.check_attribute(attribute);
        }
    }

    let mut validator = Validator { error: None };
    syn::visit::Visit::visit_file(&mut validator, file);
    validator.error.map_or(Ok(()), Err)
}

fn validate_cpp_declaration_markers(file: &syn::File) -> Result<(), String> {
    use syn::visit::Visit;

    #[derive(Default)]
    struct MarkerFinder(bool);

    impl<'ast> Visit<'ast> for MarkerFinder {
        fn visit_attribute(&mut self, attr: &'ast syn::Attribute) {
            self.0 |= crate::codegen::CodeGen::mentions_cpp_declaration_attr(attr);
        }
    }

    #[derive(Default)]
    struct UnsupportedSignatureFinder(bool);

    impl<'ast> Visit<'ast> for UnsupportedSignatureFinder {
        fn visit_type_impl_trait(&mut self, _ty: &'ast syn::TypeImplTrait) {
            self.0 = true;
        }

        fn visit_type_infer(&mut self, _ty: &'ast syn::TypeInfer) {
            self.0 = true;
        }

        fn visit_expr_infer(&mut self, _expr: &'ast syn::ExprInfer) {
            // Covers inferred const positions such as `[u8; _]`, which are
            // expressions rather than `Type::Infer` nodes in syn's AST.
            self.0 = true;
        }

        fn visit_type_macro(&mut self, _ty: &'ast syn::TypeMacro) {
            self.0 = true;
        }

        fn visit_macro(&mut self, _mac: &'ast syn::Macro) {
            // Includes macros embedded in const expressions such as
            // `[u8; count!()]`, not just `Type::Macro` nodes.
            self.0 = true;
        }
    }

    fn contains_marker<T>(node: &T, visit: impl FnOnce(&mut MarkerFinder, &T)) -> bool {
        let mut finder = MarkerFinder::default();
        visit(&mut finder, node);
        finder.0
    }

    fn validate_items(items: &[syn::Item], scope: &mut Vec<String>) -> Result<(), String> {
        for item in items {
            match item {
                syn::Item::Fn(function) => {
                    let name = function.sig.ident.to_string();
                    let qualified = if scope.is_empty() {
                        name
                    } else {
                        format!("{}::{name}", scope.join("::"))
                    };
                    let attempts = function
                        .attrs
                        .iter()
                        .filter(|attr| {
                            crate::codegen::CodeGen::mentions_cpp_declaration_attr(attr)
                        })
                        .count();
                    let markers = function
                        .attrs
                        .iter()
                        .filter(|attr| crate::codegen::CodeGen::is_cpp_declaration_attr(attr))
                        .count();
                    if attempts != markers || markers > 1 {
                        return Err(format!(
                            "cpp_declaration on '{qualified}' must use exactly one #[cfg_attr(any(), cpp_declaration)] attribute"
                        ));
                    }
                    if markers == 1 {
                        let mut unsupported_signature = UnsupportedSignatureFinder::default();
                        unsupported_signature.visit_signature(&function.sig);
                        let unsupported = if !matches!(
                            function.vis,
                            syn::Visibility::Public(_)
                        ) {
                            Some("non-public functions")
                        } else if !function.sig.generics.params.is_empty()
                            || function.sig.generics.where_clause.is_some()
                        {
                            Some("generic functions")
                        } else if function.sig.constness.is_some() {
                            Some("const functions")
                        } else if function.sig.asyncness.is_some() {
                            Some("async functions")
                        } else if function.sig.abi.is_some() {
                            Some("functions with an explicit ABI")
                        } else if function.sig.variadic.is_some() {
                            Some("variadic functions")
                        } else if unsupported_signature.0 {
                            Some("opaque, inferred, or macro-generated signature types or expressions")
                        } else if function.attrs.iter().any(|attr| {
                            (attr.path().is_ident("cfg") || attr.path().is_ident("cfg_attr"))
                                && !crate::codegen::CodeGen::is_cpp_declaration_attr(attr)
                        }) {
                            Some("conditionally compiled functions")
                        } else if function
                            .attrs
                            .iter()
                            .any(|attr| attr.path().is_ident("test"))
                        {
                            Some("test functions")
                        } else {
                            None
                        };
                        if let Some(kind) = unsupported {
                            return Err(format!(
                                "#[cfg_attr(any(), cpp_declaration)] on '{qualified}' is unsupported: {kind} cannot use a separate C++ definition"
                            ));
                        }
                    }

                    let marker_in_signature = contains_marker(&function.sig, |finder, sig| {
                        finder.visit_signature(sig)
                    });
                    let marker_in_body = contains_marker(&function.block, |finder, block| {
                        finder.visit_block(block)
                    });
                    if marker_in_signature || marker_in_body {
                        return Err(format!(
                            "cpp_declaration is only supported on module-scope free functions (found inside '{qualified}')"
                        ));
                    }
                }
                syn::Item::Mod(module) => {
                    if module.attrs.iter().any(|attr| {
                        crate::codegen::CodeGen::mentions_cpp_declaration_attr(attr)
                    }) {
                        return Err(format!(
                            "cpp_declaration is only supported on module-scope free functions (found on module '{}')",
                            module.ident
                        ));
                    }
                    if let Some((_, nested)) = &module.content {
                        scope.push(module.ident.to_string());
                        validate_items(nested, scope)?;
                        scope.pop();
                    }
                }
                other => {
                    if contains_marker(other, |finder, item| finder.visit_item(item)) {
                        return Err(
                            "cpp_declaration is only supported on module-scope free functions"
                                .to_string(),
                        );
                    }
                }
            }
        }
        Ok(())
    }

    if file.attrs.iter().any(|attr| {
        crate::codegen::CodeGen::mentions_cpp_declaration_attr(attr)
    }) {
        return Err(
            "cpp_declaration is only supported on module-scope free functions (found at crate scope)"
                .to_string(),
        );
    }
    validate_items(&file.items, &mut Vec::new())
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

    fn visit_item(&mut self, item: &'ast syn::Item) {
        // Rust item bodies establish their own safety context. In particular,
        // a function, module, const, static, impl, or trait declared inside an
        // `unsafe` block does not inherit that block's permission to perform
        // unsafe operations. Expression bodies such as closures are not items
        // and continue to inherit their enclosing lexical safety context.
        let enclosing_unsafe_context = std::mem::replace(&mut self.unsafe_context_depth, 0);
        visit::visit_item(self, item);
        self.unsafe_context_depth = enclosing_unsafe_context;
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
        let enclosing_unsafe_context = std::mem::replace(&mut self.unsafe_context_depth, 0);
        visit::visit_signature(self, &function.sig);
        self.unsafe_context_depth = usize::from(function.sig.unsafety.is_some());
        visit::visit_block(self, &function.block);
        self.unsafe_context_depth = enclosing_unsafe_context;
        self.context_stack.pop();
    }

    fn visit_impl_item_fn(&mut self, method: &'ast syn::ImplItemFn) {
        self.context_stack.push(method.sig.ident.to_string());
        let enclosing_unsafe_context = std::mem::replace(&mut self.unsafe_context_depth, 0);
        visit::visit_signature(self, &method.sig);
        self.unsafe_context_depth = usize::from(method.sig.unsafety.is_some());
        visit::visit_block(self, &method.block);
        self.unsafe_context_depth = enclosing_unsafe_context;
        self.context_stack.pop();
    }

    fn visit_trait_item_fn(&mut self, method: &'ast syn::TraitItemFn) {
        self.context_stack.push(method.sig.ident.to_string());
        let enclosing_unsafe_context = std::mem::replace(&mut self.unsafe_context_depth, 0);
        visit::visit_signature(self, &method.sig);
        self.unsafe_context_depth = usize::from(method.sig.unsafety.is_some());
        if let Some(block) = &method.default {
            visit::visit_block(self, block);
        }
        self.unsafe_context_depth = enclosing_unsafe_context;
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

    fn visit_type_array(&mut self, array: &'ast syn::TypeArray) {
        self.visit_type(&array.elem);
        // Array/repeat lengths and const generic arguments are lowered as
        // anonymous const items, so they do not inherit lexical unsafety.
        let enclosing_unsafe_context = std::mem::replace(&mut self.unsafe_context_depth, 0);
        self.visit_expr(&array.len);
        self.unsafe_context_depth = enclosing_unsafe_context;
    }

    fn visit_expr_repeat(&mut self, repeat: &'ast syn::ExprRepeat) {
        for attribute in &repeat.attrs {
            self.visit_attribute(attribute);
        }
        self.visit_expr(&repeat.expr);
        let enclosing_unsafe_context = std::mem::replace(&mut self.unsafe_context_depth, 0);
        self.visit_expr(&repeat.len);
        self.unsafe_context_depth = enclosing_unsafe_context;
    }

    fn visit_generic_argument(&mut self, argument: &'ast syn::GenericArgument) {
        match argument {
            syn::GenericArgument::Const(expression) => {
                let enclosing_unsafe_context = std::mem::replace(&mut self.unsafe_context_depth, 0);
                self.visit_expr(expression);
                self.unsafe_context_depth = enclosing_unsafe_context;
            }
            syn::GenericArgument::AssocConst(assoc_const) => {
                let enclosing_unsafe_context = std::mem::replace(&mut self.unsafe_context_depth, 0);
                visit::visit_assoc_const(self, assoc_const);
                self.unsafe_context_depth = enclosing_unsafe_context;
            }
            _ => visit::visit_generic_argument(self, argument),
        }
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
        module.symbols.get(symbol_name)
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
        .then(|| call_arity - 1);
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

    fn validate_cpp_type_symbol(&mut self, type_path: &syn::TypePath) {
        if type_path.qself.is_some() {
            return;
        }
        let Some((module_path, symbol_name)) =
            self.resolve_cpp_symbol_for_path(&type_path.path)
        else {
            return;
        };
        let type_site = type_path.to_token_stream().to_string();
        let _ = self.validate_cpp_module_symbol_access(
            &type_site,
            &module_path,
            &symbol_name,
        );
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
        visit::visit_signature(self, &function.sig);
        visit::visit_block(self, &function.block);
        self.context_stack.pop();
    }

    fn visit_impl_item_fn(&mut self, method: &'ast syn::ImplItemFn) {
        self.context_stack.push(method.sig.ident.to_string());
        visit::visit_signature(self, &method.sig);
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

    fn visit_type_path(&mut self, type_path: &'ast syn::TypePath) {
        self.validate_cpp_type_symbol(type_path);
        visit::visit_type_path(self, type_path);
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

/// Whether a nested module is compiled OUT of production output.
///
/// The emitter omits `#[cfg(test)]` modules wholesale, so the cross-file
/// collectors must not descend into them either: gathering declarations
/// from a module that will never be emitted makes the registries
/// describe types that do not exist. Concretely, a test-only
/// `impl Trait for LocalType` was emitted into the module body while the
/// `LocalType` definition beside it had been correctly dropped.
///
/// Uses the emitter's own predicate so the two cannot drift apart.
fn module_is_cfg_disabled(m: &syn::ItemMod) -> bool {
    crate::codegen::CodeGen::should_skip_cfg_attrs(&m.attrs)
}

fn collect_enum_decls_recursive(items: &[syn::Item], out: &mut Vec<syn::ItemEnum>) {
    for item in items {
        match item {
            syn::Item::Enum(e) => out.push(e.clone()),
            syn::Item::Mod(m) => {
                if module_is_cfg_disabled(m) {
                    continue;
                }
                if let Some((_, nested)) = &m.content {
                    collect_enum_decls_recursive(nested, out);
                }
            }
            _ => {}
        }
    }
}

/// Walk a Rust source file and collect every top-level / nested `Item::Trait`
/// declaration. Threaded across files in crate-mode transpilation so a module
/// that only IMPORTS a crate trait still knows the trait has a real C++
/// interface class in a sibling module (C9 / checkpoint contract 9: an owning
/// `Box<dyn CrateTrait>` must not erase to `void*`).
pub fn collect_crate_trait_decls(rust_source: &str) -> Vec<syn::ItemTrait> {
    let Ok(file) = syn::parse_str::<syn::File>(rust_source) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    collect_trait_decls_recursive(&file.items, &mut out);
    out
}

fn collect_trait_decls_recursive(items: &[syn::Item], out: &mut Vec<syn::ItemTrait>) {
    for item in items {
        match item {
            syn::Item::Trait(t) => out.push(t.clone()),
            syn::Item::Mod(m) => {
                if module_is_cfg_disabled(m) {
                    continue;
                }
                if let Some((_, nested)) = &m.content {
                    collect_trait_decls_recursive(nested, out);
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
                if module_is_cfg_disabled(m) {
                    continue;
                }
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
                if module_is_cfg_disabled(m) {
                    continue;
                }
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
                if module_is_cfg_disabled(m) {
                    continue;
                }
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
                if module_is_cfg_disabled(m) {
                    continue;
                }
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
                if module_is_cfg_disabled(m) {
                    continue;
                }
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

    fn cpp_default_argument_type_map() -> UserTypeMap {
        let mut type_map = UserTypeMap::default();
        type_map.mappings.insert(
            "rusty::SourceLocation".to_string(),
            "std::source_location".to_string(),
        );
        type_map
            .mappings
            .insert("rusty::CFile".to_string(), "FILE".to_string());
        type_map
    }

    #[test]
    fn cpp_default_arguments_emit_only_on_named_module_forward_declarations() {
        let source = r#"
            pub fn verify<Expr>(
                expr: &Expr,
                #[cfg_attr(any(), cpp_default_argument(source_location))]
                location: &::rusty::SourceLocation,
            ) where Expr: Copy {}

            pub unsafe fn print_stack_trace(
                #[cfg_attr(any(), cpp_default_argument(stderr))]
                stream: *mut ::rusty::CFile,
            ) {}
        "#;
        let options = TranspileOptions {
            explicit_gmf_includes: vec![
                GmfIncludeSpec {
                    path: "stdio.h".to_string(),
                    form: GmfIncludeForm::Angle,
                },
                GmfIncludeSpec {
                    path: "source_location".to_string(),
                    form: GmfIncludeForm::Angle,
                },
            ],
            ..TranspileOptions::default()
        };
        let output = transpile_full_with_options(
            source,
            Some("rrr.debugging"),
            &cpp_default_argument_type_map(),
            &HashSet::new(),
            None,
            &options,
        )
        .expect("typed defaults should transpile");
        assert_eq!(
            output.matches(" = std::source_location::current()").count(),
            1,
            "source-location default must occur on one declaration only:\n{output}"
        );
        assert_eq!(
            output.matches(" = stderr").count(),
            1,
            "stderr default must occur on one declaration only:\n{output}"
        );
        assert!(
            output
                .contains("const std::source_location& location = std::source_location::current()")
        );
        assert!(output.contains("FILE* stream = stderr"));
        assert!(output.contains("const std::source_location& location)"));
        assert!(output.contains("FILE* stream)"));

        let error = transpile_with_type_map(source, None, &cpp_default_argument_type_map())
            .expect_err("moduleless defaults must fail closed");
        assert!(
            error.contains("requires named C++ module output"),
            "{error}"
        );

        let mut inline_options = TranspileOptions::default();
        inline_options.inline_rust_block = true;
        let inline_error = transpile_prepared_inline_cpp_abi(
            syn::parse_file(source).expect("parse inline fixture"),
            crate::cpp_abi::CppAbiEmissionPlan::default(),
            &cpp_default_argument_type_map(),
            &HashSet::new(),
            &inline_options,
        )
        .expect_err("inline defaults must fail closed");
        assert!(
            inline_error.contains("not inline Rust blocks"),
            "{inline_error}"
        );
    }
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
    fn test_explicit_gmf_includes_are_ordered_before_module_declaration() {
        let source = "pub fn answer() -> i32 { 42 }";
        let baseline = transpile(source, Some("demo.preamble")).unwrap();
        let empty_options = transpile_full_with_options(
            source,
            Some("demo.preamble"),
            &UserTypeMap::default(),
            &HashSet::new(),
            None,
            &TranspileOptions::default(),
        )
        .unwrap();
        assert_eq!(
            baseline, empty_options,
            "empty preamble changed legacy bytes"
        );
        use sha2::{Digest, Sha256};
        assert_eq!(
            format!("{:x}", Sha256::digest(baseline.as_bytes())),
            "7ba59e308ba31cf408c7b3a3f83c856d4a5ab888f5e4fa7361437708fc24cd86",
            "default module output drifted from the ba70 no-preamble baseline"
        );

        let options = TranspileOptions {
            explicit_gmf_includes: vec![
                GmfIncludeSpec {
                    path: "demo/first.hpp".to_string(),
                    form: GmfIncludeForm::Quote,
                },
                GmfIncludeSpec {
                    path: "sys/types.h".to_string(),
                    form: GmfIncludeForm::Angle,
                },
            ],
            ..TranspileOptions::default()
        };
        let output = transpile_full_with_options(
            source,
            Some("demo.preamble"),
            &UserTypeMap::default(),
            &HashSet::new(),
            None,
            &options,
        )
        .unwrap();
        let repeated = transpile_full_with_options(
            source,
            Some("demo.preamble"),
            &UserTypeMap::default(),
            &HashSet::new(),
            None,
            &options,
        )
        .unwrap();
        assert_eq!(
            output, repeated,
            "explicit preamble output is not deterministic"
        );
        let module_fragment = output.find("\nmodule;\n").unwrap();
        let first = output.find("#include \"demo/first.hpp\"").unwrap();
        let second = output.find("#include <sys/types.h>").unwrap();
        let fixed = output.find("#include <cstdint>").unwrap();
        let declaration = output.find("export module demo.preamble;").unwrap();
        assert!(module_fragment < first && first < second && second < fixed && fixed < declaration);
    }

    #[test]
    fn test_explicit_gmf_includes_require_module_output() {
        let options = TranspileOptions {
            explicit_gmf_includes: vec![GmfIncludeSpec {
                path: "demo/header.hpp".to_string(),
                form: GmfIncludeForm::Quote,
            }],
            ..TranspileOptions::default()
        };
        let error = transpile_full_with_options(
            "fn f() {}",
            None,
            &UserTypeMap::default(),
            &HashSet::new(),
            None,
            &options,
        )
        .unwrap_err();
        assert!(error.contains("require module output"), "{error}");
    }

    #[test]
    fn test_explicit_gmf_include_validation_rejects_injection_and_collisions() {
        for invalid in [
            "/absolute.hpp",
            "../escape.hpp",
            "dir/../escape.hpp",
            "dir//header.hpp",
            "dir/./header.hpp",
            "dir\\header.hpp",
            "header.hpp\n#define BAD 1",
            "header.hpp\"",
            "<header.hpp>",
            "header.hpp;bad",
            "header with spaces.hpp",
        ] {
            let error = validate_explicit_gmf_includes(&[GmfIncludeSpec {
                path: invalid.to_string(),
                form: GmfIncludeForm::Quote,
            }])
            .unwrap_err();
            assert!(error.contains("GMF include"), "path={invalid:?}: {error}");
        }

        let duplicate = vec![
            GmfIncludeSpec {
                path: "demo/header.hpp".to_string(),
                form: GmfIncludeForm::Quote,
            },
            GmfIncludeSpec {
                path: "demo/header.hpp".to_string(),
                form: GmfIncludeForm::Quote,
            },
        ];
        assert!(
            validate_explicit_gmf_includes(&duplicate)
                .unwrap_err()
                .contains("duplicate")
        );

        let conflict = vec![
            GmfIncludeSpec {
                path: "demo/header.hpp".to_string(),
                form: GmfIncludeForm::Quote,
            },
            GmfIncludeSpec {
                path: "demo/header.hpp".to_string(),
                form: GmfIncludeForm::Angle,
            },
        ];
        assert!(
            validate_explicit_gmf_includes(&conflict)
                .unwrap_err()
                .contains("conflicting")
        );
    }

    #[test]
    fn test_module_preamble_sidecar_filters_target_and_rejects_stale_rows() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("module-preamble.toml");
        std::fs::write(
            &path,
            r#"
version = 1

[[module]]
name = "demo.net"
includes = [
    { path = "sys/epoll.h", form = "angle", when = { target_os = ["linux", "android"] } },
    { path = "demo/net.hpp", form = "quote" },
]
"#,
        )
        .unwrap();

        let missing_target = load_module_preamble_file(&path, None).unwrap_err();
        assert!(missing_target.contains("--preamble-target-os"));

        let linux = load_module_preamble_file(&path, Some("linux")).unwrap();
        let selected = linux.select_for_modules(["demo", "demo.net"]).unwrap();
        assert!(
            !selected.contains_key("demo"),
            "an emitted module without a sidecar row must remain valid and empty"
        );
        assert_eq!(
            selected["demo.net"],
            vec![
                GmfIncludeSpec {
                    path: "sys/epoll.h".to_string(),
                    form: GmfIncludeForm::Angle,
                },
                GmfIncludeSpec {
                    path: "demo/net.hpp".to_string(),
                    form: GmfIncludeForm::Quote,
                },
            ]
        );

        let windows = load_module_preamble_file(&path, Some("windows")).unwrap();
        assert_eq!(
            windows.select_for_modules(["demo.net"]).unwrap()["demo.net"],
            vec![GmfIncludeSpec {
                path: "demo/net.hpp".to_string(),
                form: GmfIncludeForm::Quote,
            }]
        );

        let stale = linux.select_for_modules(["demo"]).unwrap_err();
        assert!(stale.contains("stale/uncollected"), "{stale}");
        assert!(stale.contains("demo.net"), "{stale}");
    }

    #[test]
    fn test_module_preamble_sidecar_denies_unknown_fields_at_every_level() {
        let cases = [
            "version = 1\nunknown = true\n[[module]]\nname = \"demo\"\nincludes = [{ path = \"x.h\", form = \"angle\" }]\n",
            "version = 1\n[[module]]\nname = \"demo\"\nunknown = true\nincludes = [{ path = \"x.h\", form = \"angle\" }]\n",
            "version = 1\n[[module]]\nname = \"demo\"\nincludes = [{ path = \"x.h\", form = \"angle\", unknown = true }]\n",
            "version = 1\n[[module]]\nname = \"demo\"\nincludes = [{ path = \"x.h\", form = \"angle\", when = { target_os = [\"linux\"], feature = [\"x\"] } }]\n",
        ];
        for (index, content) in cases.into_iter().enumerate() {
            let dir = tempdir().unwrap();
            let path = dir.path().join(format!("unknown-{index}.toml"));
            std::fs::write(&path, content).unwrap();
            let error = load_module_preamble_file(&path, Some("linux")).unwrap_err();
            assert!(error.contains("unknown field"), "case {index}: {error}");
        }
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
                || output.contains("::add(static_cast<int32_t>(1), static_cast<int32_t>(2))")
                || output.contains("add(static_cast<int32_t>(1), static_cast<int32_t>(2))"),
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
    fn test_cpp_declaration_emits_only_declaration_and_preserves_alias_lookup() {
        let options = TranspileOptions {
            cxx_namespace: Some("rrr".to_string()),
            ..TranspileOptions::default()
        };
        let output = transpile_full_with_options(
            r#"
                #[cfg_attr(any(), cpp_declaration)]
                pub fn platform_open() -> i32 {
                    crate::native_only::sentinel_987654321()
                }
                use crate::platform_open as open_alias;
                pub fn through_alias() -> i32 { open_alias() }
            "#,
            Some("rrr.epoll_wrapper"),
            &UserTypeMap::default(),
            &HashSet::new(),
            None,
            &options,
        )
        .expect("valid declaration-only owner");

        assert!(output.contains("export int32_t platform_open();"), "{output}");
        assert!(!output.contains("platform_open() {"), "{output}");
        assert!(!output.contains("sentinel_987654321"), "{output}");
        assert!(output.contains("return ::rrr::platform_open();"), "{output}");
    }

    #[test]
    fn test_cpp_declaration_keeps_signature_imports_and_drops_only_its_body_imports() {
        let entry = |rust_module: &str, cpp_module: &str, cpp_namespace: &str| {
            ConsumerModuleEntry {
                rust_module: rust_module.to_string(),
                cpp_module: cpp_module.to_string(),
                cpp_namespace: cpp_namespace.to_string(),
            }
        };
        let options = TranspileOptions {
            consumer_module_map: ConsumerModuleMap {
                modules: BTreeMap::from([
                    ("runtime::iface".into(), entry("runtime::iface", "rrr.iface", "rrr")),
                    ("signature".into(), entry("signature", "rrr.signature", "rusty")),
                    ("native_body".into(), entry("native_body", "rrr.native_body", "native")),
                    ("ordinary_body".into(), entry("ordinary_body", "rrr.ordinary_body", "ordinary")),
                ]),
            },
            consumer_rust_module: Some("crate::runtime::iface".to_string()),
            ..TranspileOptions::default()
        };
        let output = transpile_full_with_options(
            r#"
                #[cfg_attr(any(), cpp_declaration)]
                pub fn convert(value: crate::signature::Unit) -> crate::signature::Unit {
                    crate::native_body::convert(value)
                }
                pub fn ordinary() -> i32 { crate::ordinary_body::run() }
            "#,
            Some("rrr.iface"),
            &UserTypeMap::default(),
            &HashSet::new(),
            Some("srpc"),
            &options,
        )
        .expect("signature dependency should remain lowerable");

        assert!(output.contains("import rrr.signature;"), "{output}");
        assert!(!output.contains("import rrr.native_body;"), "{output}");
        assert!(!output.contains("native::convert"), "{output}");
        assert!(output.contains("import rrr.ordinary_body;"), "{output}");
        assert!(output.contains("ordinary::run"), "{output}");
    }

    #[test]
    fn test_cpp_declaration_accepts_concrete_array_const_expressions() {
        let output = transpile(
            r#"
                #[cfg_attr(any(), cpp_declaration)]
                pub fn literal(value: [u8; 4]) -> [u8; 4] { value }
                #[cfg_attr(any(), cpp_declaration)]
                pub fn expression(value: [u8; 2 + 2]) -> [u8; 2 + 2] { value }
            "#,
            Some("marker.arrays"),
        )
        .expect("ordinary array lengths are concrete declarations");

        assert!(output.contains("std::array<uint8_t, 4> literal"), "{output}");
        assert!(output.contains("std::array<uint8_t, 2 + 2> expression"), "{output}");
        assert!(!output.contains("literal(std::array<uint8_t, 4> value) {"), "{output}");
        assert!(!output.contains("expression(std::array<uint8_t, 2 + 2> value) {"), "{output}");
    }

    #[test]
    fn test_cpp_declaration_rejects_signature_const_expression_macros() {
        let error = transpile(
            r#"
                macro_rules! count { () => { 4 } }
                #[cfg_attr(any(), cpp_declaration)]
                pub fn marked(value: [u8; count!()]) -> [u8; count!()] { value }
            "#,
            Some("marker.invalid"),
        )
        .expect_err("signature macro must fail before forward declaration emission");
        assert!(
            error.contains("macro-generated signature types or expressions"),
            "{error}"
        );
    }

    #[test]
    fn test_cpp_declaration_rejects_other_unsupported_forms() {
        let cases = [
            ("#[cfg_attr(any(), cpp_declaration)] pub fn f<T>(x: T) {}", "generic functions"),
            ("#[cfg_attr(any(), cpp_declaration)] pub const fn f() {}", "const functions"),
            ("#[cfg_attr(any(), cpp_declaration)] pub async fn f() {}", "async functions"),
            ("#[cfg_attr(any(), cpp_declaration)] pub extern \"C\" fn f() {}", "explicit ABI"),
            ("#[cfg_attr(any(), cpp_declaration)] fn f() {}", "non-public functions"),
            ("#[cfg_attr(any(), cpp_declaration)] pub fn f() -> impl Iterator<Item=i32> { [1].into_iter() }", "opaque, inferred, or macro-generated"),
            ("#[cfg_attr(any(), cpp_declaration)] pub fn f(x: [u8; _]) {}", "opaque, inferred, or macro-generated"),
            ("#[test] #[cfg_attr(any(), cpp_declaration)] pub fn f() {}", "test functions"),
            ("#[cfg(unix)] #[cfg_attr(any(), cpp_declaration)] pub fn f() {}", "conditionally compiled"),
            ("struct S; impl S { #[cfg_attr(any(), cpp_declaration)] pub fn f() {} }", "module-scope free functions"),
            ("pub fn outer() { #[cfg_attr(any(), cpp_declaration)] pub fn f() {} }", "module-scope free functions"),
            ("#[cpp_declaration] pub fn f() {}", "must use exactly one"),
            ("#[cfg_attr(all(), cpp_declaration)] pub fn f() {}", "must use exactly one"),
            ("#[cfg_attr(any(), cfg_attr(any(), cpp_declaration))] pub fn f() {}", "must use exactly one"),
            ("#[cpp_declaration] mod nested {}", "module-scope free functions"),
            ("#![cfg_attr(any(), cpp_declaration)] pub fn f() {}", "crate scope"),
        ];
        for (source, expected) in cases {
            let error = transpile(source, Some("marker.invalid"))
                .expect_err("unsupported marker form must fail closed");
            assert!(error.contains(expected), "{source}: {error}");
        }
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
                || (output.contains("rusty_ext::tap_err(") && output.contains("})(result")),
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
            on.contains(".hello(); }") || on.contains(".hello() ; }") || on.contains(").hello();"),
            "flag-on shim must end in a member-call fallback `deref(__self).hello()`\nGot: {on}"
        );
        // The member branch comes FIRST (rustc resolves inherent methods
        // before trait methods; for a dyn receiver the virtual member routes
        // through the adapter override to the same static impl), with the
        // qualified free-call branches behind it: one guarded
        // `requires { Greet_::hello(` tier plus the unconditional deref
        // free-call tail (`hello` is owned by exactly one trait, so the free
        // call is qualified).
        let guard_count = on.matches("requires { Greet_::hello(").count();
        assert!(
            guard_count >= 1,
            "flag-on shim must keep a qualified free-call guard (got {guard_count})\nGot: {on}"
        );
        let free_call_count = on.matches("Greet_::hello(").count();
        assert!(
            free_call_count >= 2,
            "flag-on shim must keep both free-call branches (got {free_call_count})\nGot: {on}"
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
        let manifest: UfcsTraitManifest = serde_json::from_str(&text).expect("manifest must parse");
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
            declared_trait_modules: std::collections::BTreeMap::new(),
            version: 1,
            module: "depmod".to_string(),
            declared_traits: vec!["Greet".to_string()],
            declared_trait_methods: std::collections::BTreeMap::from([(
                "Greet".to_string(),
                vec!["hello".to_string()],
            )]),
            trait_method_has_receiver: std::collections::BTreeMap::from([(
                "Greet::hello".to_string(),
                true,
            )]),
            trait_method_receiver_kind: std::collections::BTreeMap::new(),
            trait_method_bare_template_prefix_len: std::collections::BTreeMap::new(),
            method_owners: std::collections::BTreeMap::from([(
                "hello".to_string(),
                vec!["Greet".to_string()],
            )]),
            declared_types: Vec::new(),
            hygiene_aliases: std::collections::BTreeMap::new(),
            declared_macros: Vec::new(),
            root_exported_names: Vec::new(),
            declared_modules: Vec::new(),
            function_arg_pass_styles: std::collections::BTreeMap::new(),
            rusty_ext_methods_by_module: std::collections::BTreeMap::new(),
            c_like_enum_variants: std::collections::BTreeMap::new(),
            trait_assoc_type_bounds: std::collections::BTreeMap::new(),
            trait_assoc_type_names: std::collections::BTreeMap::new(),
            trait_default_methods: std::collections::BTreeMap::new(),
            preserved_collapse_methods: Vec::new(),
            trait_method_return_assoc: std::collections::BTreeMap::new(),
            cross_crate_reexports: std::collections::BTreeMap::new(),
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
    fn test_ufcs_by_value_receiver_crate_qualified_trait_path_call() {
        // #33 part 8: `depcrate::Trait::method(recv, arg)` where the RECEIVER IS
        // BY VALUE (no `&recv`). The reference-form detector bails on the
        // non-reference first argument, so the by-value handler must route the
        // call through the `<Tr>_::method` shim — its receiver shape comes ONLY
        // from the dependency manifest's `trait_method_has_receiver` (there is
        // no local trait declaration). Regression shape: itertools'
        // `::itertools::Itertools::cartesian_product(0..6, 0..9)`.
        let manifest = UfcsTraitManifest {
            declared_trait_modules: std::collections::BTreeMap::new(),
            version: 1,
            module: "depmod".to_string(),
            declared_traits: vec!["Greet".to_string()],
            declared_trait_methods: std::collections::BTreeMap::from([(
                "Greet".to_string(),
                vec!["consume".to_string()],
            )]),
            trait_method_has_receiver: std::collections::BTreeMap::from([(
                "Greet::consume".to_string(),
                true,
            )]),
            trait_method_receiver_kind: std::collections::BTreeMap::new(),
            trait_method_bare_template_prefix_len: std::collections::BTreeMap::new(),
            method_owners: std::collections::BTreeMap::from([(
                "consume".to_string(),
                vec!["Greet".to_string()],
            )]),
            declared_types: Vec::new(),
            hygiene_aliases: std::collections::BTreeMap::new(),
            declared_macros: Vec::new(),
            root_exported_names: Vec::new(),
            declared_modules: Vec::new(),
            function_arg_pass_styles: std::collections::BTreeMap::new(),
            rusty_ext_methods_by_module: std::collections::BTreeMap::new(),
            c_like_enum_variants: std::collections::BTreeMap::new(),
            trait_assoc_type_bounds: std::collections::BTreeMap::new(),
            trait_assoc_type_names: std::collections::BTreeMap::new(),
            trait_default_methods: std::collections::BTreeMap::new(),
            preserved_collapse_methods: Vec::new(),
            trait_method_return_assoc: std::collections::BTreeMap::new(),
            cross_crate_reexports: std::collections::BTreeMap::new(),
        };
        let path = std::env::temp_dir().join("rusty_ufcs_manifest_byvalue_test.json");
        std::fs::write(&path, serde_json::to_string(&manifest).unwrap()).unwrap();

        let src = r#"
            struct Local { id: i32 }
            fn use_it(x: Local) -> i32 { depmod::Greet::consume(x, 1) }
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
            on.contains("Greet_::consume("),
            "by-value `depmod::Greet::consume(x, 1)` must route to the Greet_::consume shim\nGot: {on}"
        );
        assert!(
            !on.contains("Greet::consume("),
            "the verbatim associated-call form must not survive\nGot: {on}"
        );
    }

    #[test]
    fn test_cross_crate_c_like_enum_variants_roundtrip_and_crate_rename_alias() {
        // Producer: a C-like enum's variants land in the manifest with the
        // enum-qualified crate-relative path (Rust variant glob re-exports
        // make bare variants crate-visible; C++ enum classes don't).
        let dep_src = r#"
            pub mod yaml {
                pub enum yaml_event_type_t {
                    YAML_NO_EVENT,
                    YAML_GO_EVENT,
                }
                pub use self::yaml_event_type_t::*;
            }
            pub use crate::yaml::*;
        "#;
        let manifest_path =
            std::env::temp_dir().join("rusty_ufcs_manifest_variant_roundtrip_test.json");
        let _ = std::fs::remove_file(&manifest_path);
        let options = TranspileOptions {
            emit_ufcs_trait_manifest_path: Some(manifest_path.clone()),
            ..TranspileOptions::default()
        };
        let _ = transpile_full_with_options(
            dep_src,
            Some("sysdep"),
            &UserTypeMap::default(),
            &HashSet::new(),
            None,
            &options,
        )
        .expect("dep transpile should succeed");
        let manifest: UfcsTraitManifest = serde_json::from_str(
            &std::fs::read_to_string(&manifest_path).expect("manifest must be written"),
        )
        .expect("manifest must parse");
        assert_eq!(
            manifest.c_like_enum_variants.get("YAML_GO_EVENT"),
            Some(&"yaml::yaml_event_type_t".to_string()),
            "producer must record the variant's enum-qualified path\nGot: {manifest:?}"
        );

        // Consumer: `use sysdep as sys;` is a CRATE rename → a namespace
        // alias (not `using sys = sysdep;`), and `sys::YAML_GO_EVENT`
        // qualifies through the manifest to the enum-scoped C++ path.
        let consumer_src = r#"
            use sysdep as sys;
            pub fn check(t: u32) -> bool {
                t == sys::YAML_GO_EVENT as u32
            }
        "#;
        let consume_options = TranspileOptions {
            dependency_ufcs_trait_manifests: vec![manifest_path.clone()],
            ..TranspileOptions::default()
        };
        let out = transpile_full_with_options(
            consumer_src,
            Some("consumer"),
            &UserTypeMap::default(),
            &HashSet::new(),
            None,
            &consume_options,
        )
        .expect("consumer transpile should succeed");
        let _ = std::fs::remove_file(&manifest_path);
        assert!(
            out.contains("namespace sys = ::sysdep;"),
            "crate rename import must emit an absolute namespace alias\nGot: {out}"
        );
        assert!(
            !out.contains("using sys = sysdep;"),
            "crate rename import must not emit a type alias\nGot: {out}"
        );
        assert!(
            out.contains("::sysdep::yaml::yaml_event_type_t::YAML_GO_EVENT"),
            "cross-crate variant reference must qualify to the enum-scoped path\nGot: {out}"
        );
    }

    #[test]
    fn test_cross_crate_module_reexport_requalifies_facade_paths() {
        // serde-facade shape: the FACADE crate re-exports a dependency's
        // module at its root (`pub use core_crate::de;`), so a consumer's
        // `facade::de::Visitor` only resolves through the dependency --
        // the facade's own C++ `namespace de` holds just its additions.
        let core_manifest_path =
            std::env::temp_dir().join("rusty_ufcs_manifest_reexport_core_test.json");
        let facade_manifest_path =
            std::env::temp_dir().join("rusty_ufcs_manifest_reexport_facade_test.json");
        let _ = std::fs::remove_file(&core_manifest_path);
        let _ = std::fs::remove_file(&facade_manifest_path);

        let core_src = r#"
            pub mod de {
                pub struct Visitor {
                    pub id: u64,
                }
            }
        "#;
        let core_options = TranspileOptions {
            emit_ufcs_trait_manifest_path: Some(core_manifest_path.clone()),
            ..TranspileOptions::default()
        };
        let _ = transpile_full_with_options(
            core_src,
            Some("core_crate"),
            &UserTypeMap::default(),
            &HashSet::new(),
            None,
            &core_options,
        )
        .expect("core transpile should succeed");

        let facade_src = r#"
            pub use core_crate::de;
        "#;
        let facade_options = TranspileOptions {
            emit_ufcs_trait_manifest_path: Some(facade_manifest_path.clone()),
            dependency_ufcs_trait_manifests: vec![core_manifest_path.clone()],
            ..TranspileOptions::default()
        };
        let _ = transpile_full_with_options(
            facade_src,
            Some("facade"),
            &UserTypeMap::default(),
            &HashSet::new(),
            None,
            &facade_options,
        )
        .expect("facade transpile should succeed");
        let facade_manifest: UfcsTraitManifest = serde_json::from_str(
            &std::fs::read_to_string(&facade_manifest_path).expect("facade manifest written"),
        )
        .expect("facade manifest parses");
        assert_eq!(
            facade_manifest.cross_crate_reexports.get("de"),
            Some(&"core_crate::de".to_string()),
            "facade must record the cross-crate module re-export\nGot: {facade_manifest:?}"
        );

        let consumer_src = r#"
            pub fn probe(v: facade::de::Visitor) -> u64 {
                v.id
            }
        "#;
        let consumer_options = TranspileOptions {
            dependency_ufcs_trait_manifests: vec![
                core_manifest_path.clone(),
                facade_manifest_path.clone(),
            ],
            ..TranspileOptions::default()
        };
        let out = transpile_full_with_options(
            consumer_src,
            Some("consumer"),
            &UserTypeMap::default(),
            &HashSet::new(),
            None,
            &consumer_options,
        )
        .expect("consumer transpile should succeed");
        let _ = std::fs::remove_file(&core_manifest_path);
        let _ = std::fs::remove_file(&facade_manifest_path);
        assert!(
            out.contains("::core_crate::de::Visitor"),
            "facade module path must requalify to the dependency\nGot: {out}"
        );
        assert!(
            !out.contains("facade::de::Visitor"),
            "facade-relative spelling must not survive\nGot: {out}"
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
      "cpp_module": "std",
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
        assert_eq!(std_module.cpp_module, "std");
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
cpp_module = "std"
namespace = "std"

[modules.std.symbols.max]
kind = "function"
callable_signatures = ["int(int,int)"]
"#,
        )
        .expect("write toml index");

        let index = load_cpp_module_symbol_index_files(&[index_path]).expect("load toml index");
        let std_module = index.modules.get("std").expect("std module");
        assert_eq!(std_module.cpp_module, "std");
        assert_eq!(std_module.namespace.as_deref(), Some("std"));
        let max = std_module.symbols.get("max").expect("max symbol");
        assert_eq!(max.kind.as_deref(), Some("function"));
        assert_eq!(max.callable_signatures, vec!["int(int,int)".to_string()]);
    }

    #[test]
    fn test_cpp_module_symbol_index_requires_explicit_cpp_module() {
        let dir = tempdir().expect("tempdir");
        let index_path = dir.path().join("missing_cpp_module.toml");
        std::fs::write(
            &index_path,
            r#"
version = 1
[modules."legacy::serde"]
namespace = "rrr"
"#,
        )
        .expect("write incomplete index");

        let error = load_cpp_module_symbol_index_files(&[index_path])
            .expect_err("missing cpp_module must fail closed");
        assert!(error.contains("cpp_module"), "{error}");
    }

    #[test]
    fn test_cpp_module_symbol_index_rejects_malformed_cpp_module() {
        let dir = tempdir().expect("tempdir");
        let index_path = dir.path().join("malformed_cpp_module.toml");
        std::fs::write(
            &index_path,
            r#"
version = 1
[modules."legacy::serde"]
cpp_module = "rrr.serializable;import evil"
namespace = "rrr"
"#,
        )
        .expect("write malformed index");

        let error = load_cpp_module_symbol_index_files(&[index_path])
            .expect_err("malformed cpp_module must fail closed");
        assert!(error.contains("invalid C++ module name"), "{error}");
        assert!(error.contains("rrr.serializable;import evil"), "{error}");
    }

    #[test]
    fn test_cpp_module_symbol_index_rejects_unknown_fields_at_every_level() {
        let dir = tempdir().expect("tempdir");
        let cases = [
            (
                "top",
                r#"{"version":1,"future":true,"modules":{}}"#,
                "future",
            ),
            (
                "module",
                r#"{"version":1,"modules":{"legacy::serde":{"cpp_module":"rrr.serializable","namespace":"rrr","future":true,"symbols":{}}}}"#,
                "future",
            ),
            (
                "symbol",
                r#"{"version":1,"modules":{"legacy::serde":{"cpp_module":"rrr.serializable","namespace":"rrr","symbols":{"Archive":{"kind":"type","future":true}}}}}"#,
                "future",
            ),
        ];

        for (name, contents, unknown_field) in cases {
            let index_path = dir.path().join(format!("unknown_{name}.json"));
            std::fs::write(&index_path, contents).expect("write unknown-field index");
            let error = load_cpp_module_symbol_index_files(&[index_path])
                .expect_err("unknown index field must fail closed");
            assert!(error.contains("unknown field"), "{name}: {error}");
            assert!(error.contains(unknown_field), "{name}: {error}");
        }
    }

    #[test]
    fn test_load_consumer_module_map_toml() {
        let dir = tempdir().expect("tempdir");
        let map_path = dir.path().join("consumer-modules.toml");
        std::fs::write(
            &map_path,
            r#"
version = 1

[[module]]
rust_module = "crate::base::sync"
cpp_module = "rrr.basetypes"
cpp_namespace = "rrr"

[[module]]
rust_module = "crate::rpc::client"
cpp_module = "rrr.client"
cpp_namespace = "rrr"
"#,
        )
        .expect("write consumer map");

        let map = load_consumer_module_map(&map_path).expect("load consumer map");
        assert_eq!(map.modules.len(), 2);
        let sync = map.entry_for_rust_module("base::sync").unwrap();
        assert_eq!(sync.cpp_module, "rrr.basetypes");
        assert_eq!(sync.cpp_namespace, "rrr");
        assert_eq!(
            map.entry_for_cpp_module("rrr.client")
                .map(|entry| entry.rust_module.as_str()),
            Some("rpc::client")
        );
    }

    #[test]
    fn test_consumer_module_map_rejects_duplicate_cpp_module() {
        let dir = tempdir().expect("tempdir");
        let map_path = dir.path().join("consumer-modules.json");
        std::fs::write(
            &map_path,
            r#"{
  "version": 1,
  "module": [
    {"rust_module":"crate::base::sync","cpp_module":"rrr.shared","cpp_namespace":"rrr"},
    {"rust_module":"crate::rpc::client","cpp_module":"rrr.shared","cpp_namespace":"rrr"}
  ]
}"#,
        )
        .expect("write consumer map");

        let error = load_consumer_module_map(&map_path).unwrap_err();
        assert!(error.contains("repeats C++ module 'rrr.shared'"), "{error}");
    }

    fn grouped_epoll_consumer_map() -> ConsumerModuleMap {
        ConsumerModuleMap {
            modules: BTreeMap::from([(
                "runtime::epoll".to_string(),
                ConsumerModuleEntry {
                    rust_module: "runtime::epoll".to_string(),
                    cpp_module: "rrr.epoll_wrapper".to_string(),
                    cpp_namespace: "rrr".to_string(),
                },
            )]),
        }
    }

    #[test]
    fn test_consumer_rust_module_requires_consumer_map() {
        let options = TranspileOptions {
            consumer_rust_module: Some("crate::runtime::epoll_linux".to_string()),
            ..TranspileOptions::default()
        };
        let error = transpile_full_with_options(
            "pub fn platform_mask() -> i32 { 0 }",
            Some("rrr.epoll_wrapper"),
            &UserTypeMap::default(),
            &HashSet::new(),
            None,
            &options,
        )
        .expect_err("consumer Rust module without map must fail");
        assert_eq!(
            error,
            "--consumer-rust-module requires --consumer-module-map"
        );
    }

    #[test]
    fn test_consumer_rust_module_requires_module_emission() {
        let options = TranspileOptions {
            consumer_module_map: grouped_epoll_consumer_map(),
            consumer_rust_module: Some("crate::runtime::epoll_linux".to_string()),
            ..TranspileOptions::default()
        };
        let error = transpile_full_with_options(
            "pub fn platform_mask() -> i32 { 0 }",
            None,
            &UserTypeMap::default(),
            &HashSet::new(),
            None,
            &options,
        )
        .expect_err("consumer Rust module without module emission must fail");
        assert_eq!(
            error,
            "--consumer-rust-module requires module emission; pass --module-name <name>"
        );
    }

    #[test]
    fn test_consumer_rust_module_requires_canonical_crate_path() {
        for (path, expected) in [
            ("runtime::epoll_linux", "must begin with 'crate'"),
            (
                "::crate::runtime::epoll_linux",
                "must be an unparameterized crate path",
            ),
            (
                "crate::runtime::epoll_linux::<i32>",
                "must be an unparameterized crate path",
            ),
        ] {
            let options = TranspileOptions {
                consumer_module_map: grouped_epoll_consumer_map(),
                consumer_rust_module: Some(path.to_string()),
                ..TranspileOptions::default()
            };
            let error = transpile_full_with_options(
                "pub fn platform_mask() -> i32 { 0 }",
                Some("rrr.epoll_wrapper"),
                &UserTypeMap::default(),
                &HashSet::new(),
                None,
                &options,
            )
            .expect_err("non-canonical consumer Rust path must fail");
            assert!(error.contains(expected), "{path}: {error}");
        }
    }

    #[test]
    fn test_consumer_rust_module_rejects_mapped_cpp_module_mismatch() {
        let mut map = grouped_epoll_consumer_map();
        map.modules.insert(
            "rpc::client".to_string(),
            ConsumerModuleEntry {
                rust_module: "rpc::client".to_string(),
                cpp_module: "rrr.client".to_string(),
                cpp_namespace: "rrr".to_string(),
            },
        );
        let options = TranspileOptions {
            consumer_module_map: map,
            consumer_rust_module: Some("crate::rpc::client".to_string()),
            ..TranspileOptions::default()
        };
        let error = transpile_full_with_options(
            "pub fn platform_mask() -> i32 { 0 }",
            Some("rrr.epoll_wrapper"),
            &UserTypeMap::default(),
            &HashSet::new(),
            None,
            &options,
        )
        .expect_err("mapped consumer Rust path owned by another C++ module must fail");
        assert_eq!(
            error,
            "--consumer-rust-module 'crate::rpc::client' maps to C++ module 'rrr.client', not current module 'rrr.epoll_wrapper'"
        );
    }

    fn serializable_cpp_module_index() -> CppModuleSymbolIndex {
        let mut symbols = BTreeMap::new();
        for archive in ["BinaryWriteArchive", "BinaryReadArchive"] {
            symbols.insert(
                archive.to_string(),
                CppModuleIndexSymbol {
                    kind: Some("type".to_string()),
                    callable_signatures: Vec::new(),
                },
            );
        }
        for operation in ["Serialize_::serialize", "Deserialize_::deserialize"] {
            symbols.insert(
                operation.to_string(),
                CppModuleIndexSymbol {
                    kind: Some("function_template".to_string()),
                    callable_signatures: vec!["void(T,Archive)".to_string()],
                },
            );
        }
        CppModuleSymbolIndex {
            modules: BTreeMap::from([(
                "legacy::serde".to_string(),
                CppModuleIndexModule {
                    cpp_module: "rrr.serializable".to_string(),
                    namespace: Some("rrr".to_string()),
                    symbols,
                },
            )]),
        }
    }

    fn transpile_with_serializable_cpp_index(source: &str) -> Result<String, String> {
        let options = TranspileOptions {
            cpp_module_symbol_index: Some(serializable_cpp_module_index()),
            cpp_module_symbol_index_sources: vec![PathBuf::from("/tmp/rrr-cpp-index.toml")],
            ..TranspileOptions::default()
        };
        transpile_full_with_options(
            source,
            Some("srpc.consumer"),
            &UserTypeMap::default(),
            &HashSet::new(),
            None,
            &options,
        )
    }

    #[test]
    fn test_cpp_module_declared_namespace_projects_types_and_expressions_absolutely() {
        let output = transpile_with_serializable_cpp_index(
            r#"
use cpp::legacy::serde;

pub unsafe fn roundtrip(
    value: i32,
    write: &mut serde::BinaryWriteArchive,
    read: &mut serde::BinaryReadArchive,
) {
    serde::Serialize_::serialize(value, write);
    serde::Deserialize_::deserialize(value, read);
}

mod cpp {
    pub mod legacy {
        pub mod serde {
            pub struct BinaryWriteArchive;
            pub struct BinaryReadArchive;
            pub mod Serialize_ {}
            pub mod Deserialize_ {}
        }
    }
}
"#,
        )
        .expect("indexed serializable surface should transpile");

        assert_eq!(output.matches("import rrr.serializable;").count(), 1, "{output}");
        assert!(!output.contains("import legacy.serde;"), "{output}");
        assert!(output.contains("::rrr::BinaryWriteArchive& write"), "{output}");
        assert!(output.contains("::rrr::BinaryReadArchive& read"), "{output}");
        assert!(output.contains("::rrr::Serialize_::serialize("), "{output}");
        assert!(output.contains("::rrr::Deserialize_::deserialize("), "{output}");
        assert!(!output.contains("rrr::serializable::Serialize_"), "{output}");
        assert!(!output.contains("namespace cpp"), "{output}");
    }

    #[test]
    fn test_cpp_module_declared_namespace_projects_alias_binding() {
        let output = transpile_with_serializable_cpp_index(
            r#"
use cpp::legacy::serde as legacy;
pub unsafe fn encode(value: i32, ar: &mut legacy::BinaryWriteArchive) {
    legacy::Serialize_::serialize(value, ar);
}
"#,
        )
        .expect("indexed aliased serializable surface should transpile");

        assert!(output.contains("import rrr.serializable;"), "{output}");
        assert!(!output.contains("import legacy.serde;"), "{output}");
        assert!(output.contains("::rrr::BinaryWriteArchive& ar"), "{output}");
        assert!(output.contains("::rrr::Serialize_::serialize("), "{output}");
        assert!(!output.contains("legacy::BinaryWriteArchive"), "{output}");
        assert!(!output.contains("legacy::Serialize_"), "{output}");
    }

    #[test]
    fn test_ordinary_inline_cpp_module_is_not_suppressed_without_reserved_import() {
        let output = transpile(
            "mod cpp { pub struct Native { pub value: i32 } }",
            None,
        )
        .expect("ordinary cpp module should transpile");
        assert!(output.contains("namespace cpp"), "{output}");
        assert!(output.contains("struct Native"), "{output}");
    }

    #[test]
    fn test_cpp_module_type_errors_when_module_path_is_unindexed() {
        let options = TranspileOptions {
            cpp_module_symbol_index: Some(serializable_cpp_module_index()),
            cpp_module_symbol_index_sources: vec![PathBuf::from("/tmp/rrr-cpp-index.toml")],
            ..TranspileOptions::default()
        };
        let error = transpile_full_with_options(
            "use cpp::rrr::missing; fn f(_: missing::Archive) {}",
            None,
            &UserTypeMap::default(),
            &HashSet::new(),
            None,
            &options,
        )
        .expect_err("unindexed type module must fail closed");
        assert!(error.contains("module path is not present"), "{error}");
        assert!(error.contains("module `rrr::missing`"), "{error}");
        assert!(error.contains("symbol `Archive`"), "{error}");
    }

    #[test]
    fn test_cpp_module_type_errors_when_symbol_is_unindexed() {
        let error = transpile_with_serializable_cpp_index(
            "use cpp::legacy::serde; fn f(_: serde::UnknownArchive) {}",
        )
        .expect_err("unindexed type symbol must fail closed");
        assert!(error.contains("symbol is not present"), "{error}");
        assert!(error.contains("module `legacy::serde`"), "{error}");
        assert!(error.contains("symbol `UnknownArchive`"), "{error}");
    }

    #[test]
    fn test_load_cpp_module_symbol_index_rejects_removed_safe_field() {
        let dir = tempdir().expect("tempdir");
        let index_path = dir.path().join("cpp_index.toml");
        std::fs::write(
            &index_path,
            r#"
version = 1
[modules.std.symbols.max]
kind = "function"
callable_signatures = ["int(int,int)"]
safe = true
"#,
        )
        .expect("write toml index");

        let err = load_cpp_module_symbol_index_files(&[index_path])
            .expect_err("removed safe metadata must be rejected");
        assert!(err.contains("Invalid TOML C++ module symbol index"));
        assert!(err.contains("unknown field `safe`"));
    }

    #[test]
    fn test_load_cpp_module_symbol_index_canonicalizes_and_merges_namespace() {
        let dir = tempdir().expect("tempdir");
        let first = dir.path().join("first.toml");
        let second = dir.path().join("second.toml");
        std::fs::write(
            &first,
            r#"
version = 1
[modules."rrr::logging"]
cpp_module = "rrr.logging"
namespace = " rrr :: logging_api "
[modules."rrr::logging".symbols.first]
kind = "function"
callable_signatures = ["void()"]
"#,
        )
        .expect("write first index");
        std::fs::write(
            &second,
            r#"
version = 1
[modules."rrr::logging"]
cpp_module = "rrr.logging"
namespace = "rrr::logging_api"
[modules."rrr::logging".symbols.second]
kind = "function"
callable_signatures = ["void()"]
"#,
        )
        .expect("write second index");

        let index =
            load_cpp_module_symbol_index_files(&[first, second]).expect("merge canonical paths");
        let module = index.modules.get("rrr::logging").expect("merged module");
        assert_eq!(module.namespace.as_deref(), Some("rrr::logging_api"));
        assert!(module.symbols.contains_key("first"));
        assert!(module.symbols.contains_key("second"));
    }

    #[test]
    fn test_load_cpp_module_symbol_index_rejects_conflicting_namespace() {
        let dir = tempdir().expect("tempdir");
        let first = dir.path().join("first.toml");
        let second = dir.path().join("second.toml");
        std::fs::write(&first, "version = 1\n[modules.demo]\ncpp_module = \"demo\"\nnamespace = \"one\"\n")
            .expect("write first index");
        std::fs::write(
            &second,
            "version = 1\n[modules.demo]\ncpp_module = \"demo\"\nnamespace = \"two\"\n",
        )
        .expect("write second index");

        let err = load_cpp_module_symbol_index_files(&[first, second])
            .expect_err("conflicting namespace must be rejected");
        assert!(err.contains("conflicting namespace"));
        assert!(err.contains("'one' vs 'two'"));
    }

    #[test]
    fn test_load_cpp_module_symbol_index_rejects_invalid_namespace() {
        for invalid in [
            "",
            "::rrr",
            "rrr::",
            "rrr::::logging",
            "rrr.logging",
            "rrr::logging<int>",
            "rrr::class",
            "_rrr::logging",
            "rrr::_Logging",
            "rrr::log__detail",
            "rrr; injected",
        ] {
            let dir = tempdir().expect("tempdir");
            let index_path = dir.path().join("cpp_index.toml");
            std::fs::write(
                &index_path,
                format!(
                    "version = 1\n[modules.demo]\ncpp_module = \"demo\"\nnamespace = {:?}\n",
                    invalid
                ),
            )
            .expect("write invalid index");

            let err = load_cpp_module_symbol_index_files(&[index_path])
                .expect_err("invalid namespace must be rejected");
            assert!(
                err.contains("invalid namespace for module 'demo'"),
                "invalid={invalid:?}, err={err}"
            );
        }
    }

    #[test]
    fn test_unknown_inert_cpp_marker_name_is_a_hard_error() {
        // The inert carrier exists so contracts survive rustc unseen — a
        // misspelled or unported contract must fail loudly, not vanish.
        for source in [
            "#[cfg_attr(any(), cpp_nmae(\"x\"))] pub fn f() {}",
            "#[cfg_attr(any(), cpp_frobnicate)] pub struct S;",
            "pub struct T; #[cfg_attr(any(), cpp_virtual_dispatch)] impl T { pub fn m(&self) {} }",
        ] {
            let err = transpile(source, None)
                .expect_err("unknown inert cpp_* marker must be rejected");
            assert!(
                err.contains("unknown reserved marker"),
                "source={source:?}, err={err}"
            );
            assert!(
                err.contains("Known cpp_* markers"),
                "the diagnostic must list the known roster: {err}"
            );
        }
    }

    #[test]
    fn test_known_inert_cpp_markers_and_foreign_payloads_still_pass() {
        // Known contracts keep working; and an inert payload that is not a
        // cpp_* name at all (thread_local, allow) is not this validator's
        // business.
        for source in [
            "pub struct S { v: i32 }\nimpl S {\n    #[cfg_attr(any(), cpp_ctor)]\n    fn new(v: i32) -> S { S { v } }\n}",
            "#[cfg_attr(any(), thread_local)] static X: i32 = 0;",
            "#[cfg_attr(any(), allow(dead_code))] pub fn g() {}",
            // ACTIVE cfg_attr predicates are rustc's own surface, not ours.
            "#[cfg_attr(test, cpp_this_is_rustcs_problem)] pub fn h() {}",
        ] {
            assert!(
                transpile(source, None).is_ok(),
                "source={source:?} must transpile"
            );
        }
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
                cpp_module: "std".to_string(),
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
                cpp_module: "std".to_string(),
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
                cpp_module: "std".to_string(),
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
    fn test_cpp_module_unsafe_context_does_not_cross_nested_item_boundaries() {
        let file = syn::parse_str::<syn::File>(
            r#"
use cpp::std as cpp_std;
fn outer() {
    unsafe {
        fn nested_fn() -> i32 { cpp_std::max(1, 2) }

        struct Local;
        impl Local {
            fn nested_method() -> i32 { cpp_std::max(3, 4) }
            const NESTED_ASSOC_CONST: i32 = cpp_std::max(5, 6);
        }
        trait LocalTrait {
            fn nested_default() -> i32 { cpp_std::max(7, 8) }
            const NESTED_ASSOC_DEFAULT: i32 = cpp_std::max(9, 10);
        }

        const NESTED_CONST: i32 = cpp_std::max(11, 12);
        static NESTED_STATIC: i32 = cpp_std::max(13, 14);

        mod nested_module {
            use cpp::std as cpp_std;
            pub fn nested_module_fn() -> i32 { cpp_std::max(15, 16) }
        }
    }
}

fn safe_closure() {
    let _call_later = || cpp_std::max(17, 18);
}
"#,
        )
        .expect("nested item safety fixture should parse");

        let diagnostics = collect_cpp_foreign_call_unsafe_violations(&file);
        assert_eq!(
            diagnostics.len(),
            9,
            "each safe nested item and safe closure call must be rejected: {diagnostics:#?}"
        );
        for context in [
            "outer::nested_fn",
            "outer::nested_method",
            "outer::nested_default",
            "outer::nested_module::nested_module_fn",
            "safe_closure",
        ] {
            assert!(
                diagnostics
                    .iter()
                    .any(|diagnostic| diagnostic.contains(context)),
                "missing violation in {context}: {diagnostics:#?}"
            );
        }
    }

    #[test]
    fn test_cpp_module_unsafe_context_keeps_unsafe_items_blocks_and_closures() {
        let file = syn::parse_str::<syn::File>(
            r#"
use cpp::std as cpp_std;

unsafe fn unsafe_function() -> i32 { cpp_std::max(1, 2) }

struct Host;
impl Host {
    unsafe fn unsafe_method() -> i32 { cpp_std::max(3, 4) }
}
trait HostTrait {
    unsafe fn unsafe_default() -> i32 { cpp_std::max(5, 6) }
}

fn outer() {
    unsafe {
        fn nested_explicit_block() -> i32 { unsafe { cpp_std::max(7, 8) } }
        unsafe fn nested_unsafe_fn() -> i32 { cpp_std::max(9, 10) }

        struct Local;
        impl Local {
            unsafe fn nested_unsafe_method() -> i32 { cpp_std::max(11, 12) }
            fn nested_explicit_method() -> i32 { unsafe { cpp_std::max(13, 14) } }
        }
        trait LocalTrait {
            unsafe fn nested_unsafe_default() -> i32 { cpp_std::max(15, 16) }
            fn nested_explicit_default() -> i32 { unsafe { cpp_std::max(17, 18) } }
        }

        let _inherits_unsafe_block = || cpp_std::max(19, 20);
    }
}
"#,
        )
        .expect("positive item safety fixture should parse");

        let diagnostics = collect_cpp_foreign_call_unsafe_violations(&file);
        assert!(
            diagnostics.is_empty(),
            "unsafe functions/methods, explicit unsafe blocks, and closures inside an unsafe block remain allowed: {diagnostics:#?}"
        );
    }

    #[test]
    fn test_cpp_module_callable_signatures_never_inherit_unsafe_context() {
        let file = syn::parse_str::<syn::File>(
            r#"
use cpp::std as cpp_std;
fn outer() {
    unsafe {
        fn safe_free(_: [u8; cpp_std::max(1, 2) as usize]) {}
        unsafe fn unsafe_free(_: [u8; cpp_std::max(3, 4) as usize]) {}

        struct Local;
        impl Local {
            fn safe_method(_: [u8; cpp_std::max(5, 6) as usize]) {}
            unsafe fn unsafe_method(_: [u8; cpp_std::max(7, 8) as usize]) {}
        }
        trait LocalTrait {
            fn safe_trait_method(_: [u8; cpp_std::max(9, 10) as usize]);
            unsafe fn unsafe_trait_method(_: [u8; cpp_std::max(11, 12) as usize]);
        }
    }
}
"#,
        )
        .expect("signature safety fixture should parse");

        let diagnostics = collect_cpp_foreign_call_unsafe_violations(&file);
        assert_eq!(
            diagnostics.len(),
            6,
            "safe and unsafe callable signatures must both reject implicit unsafe context: {diagnostics:#?}"
        );
        for context in [
            "outer::safe_free",
            "outer::unsafe_free",
            "outer::safe_method",
            "outer::unsafe_method",
            "outer::safe_trait_method",
            "outer::unsafe_trait_method",
        ] {
            assert!(
                diagnostics
                    .iter()
                    .any(|diagnostic| diagnostic.contains(context)),
                "missing signature violation in {context}: {diagnostics:#?}"
            );
        }
    }

    #[test]
    fn test_cpp_module_callable_signatures_allow_explicit_unsafe_blocks() {
        let file = syn::parse_str::<syn::File>(
            r#"
use cpp::std as cpp_std;

fn safe_free(_: [u8; (unsafe { cpp_std::max(1, 2) }) as usize]) {}
unsafe fn unsafe_free(_: [u8; (unsafe { cpp_std::max(3, 4) }) as usize]) {}

struct Host;
impl Host {
    fn safe_method(_: [u8; (unsafe { cpp_std::max(5, 6) }) as usize]) {}
    unsafe fn unsafe_method(_: [u8; (unsafe { cpp_std::max(7, 8) }) as usize]) {}
}
trait HostTrait {
    fn safe_trait_method(_: [u8; (unsafe { cpp_std::max(9, 10) }) as usize]);
    unsafe fn unsafe_trait_method(_: [u8; (unsafe { cpp_std::max(11, 12) }) as usize]);
}
"#,
        )
        .expect("explicitly unsafe signature fixture should parse");

        let diagnostics = collect_cpp_foreign_call_unsafe_violations(&file);
        assert!(
            diagnostics.is_empty(),
            "explicit unsafe blocks in callable signatures must remain allowed: {diagnostics:#?}"
        );
    }

    #[test]
    fn test_cpp_module_anonymous_consts_do_not_inherit_unsafe_context() {
        let file = syn::parse_str::<syn::File>(
            r#"
use cpp::std as cpp_std;

struct Generic<const N: usize>;
trait HasConst { const N: usize; }
fn takes<const N: usize>() {}

fn outer() {
    unsafe {
        let _repeat = [0; { cpp_std::max(1, 2) as usize }];
        let _: [i32; { cpp_std::max(3, 4) as usize }];
        let _: Generic<{ cpp_std::max(5, 6) as usize }>;
        takes::<{ cpp_std::max(7, 8) as usize }>();
        let _: &dyn HasConst<N = { cpp_std::max(9, 10) as usize }>;
    }
}
"#,
        )
        .expect("anonymous const safety fixture should parse");

        let diagnostics = collect_cpp_foreign_call_unsafe_violations(&file);
        assert_eq!(
            diagnostics.len(),
            5,
            "repeat/array lengths and positional/associated const arguments must establish fresh safe contexts: {diagnostics:#?}"
        );
        assert!(
            diagnostics
                .iter()
                .all(|diagnostic| diagnostic.contains("outer")),
            "all anonymous-const violations should retain their lexical diagnostic label: {diagnostics:#?}"
        );
    }

    #[test]
    fn test_cpp_module_anonymous_consts_allow_only_explicit_unsafe_context() {
        let file = syn::parse_str::<syn::File>(
            r#"
use cpp::std as cpp_std;

struct Generic<const N: usize>;
trait HasConst { const N: usize; }
fn takes<const N: usize>() {}

fn explicit() {
    let _repeat = [0; { (unsafe { cpp_std::max(1, 2) }) as usize }];
    let _: [i32; { (unsafe { cpp_std::max(3, 4) }) as usize }];
    let _: Generic<{ (unsafe { cpp_std::max(5, 6) }) as usize }>;
    takes::<{ (unsafe { cpp_std::max(7, 8) }) as usize }>();
    let _: &dyn HasConst<N = { (unsafe { cpp_std::max(9, 10) }) as usize }>;
}

fn lexical_expressions_still_inherit() {
    unsafe {
        let _repeat_element = [cpp_std::max(11, 12); 1];
        let _closure = || cpp_std::max(13, 14);
        let _async_block = async { cpp_std::max(15, 16) };
        let _inline_const = const { cpp_std::max(17, 18) };
    }
}
"#,
        )
        .expect("explicitly unsafe anonymous const fixture should parse");

        let diagnostics = collect_cpp_foreign_call_unsafe_violations(&file);
        assert!(
            diagnostics.is_empty(),
            "explicit unsafe blocks in anonymous consts and lexical closure/async/inline-const inheritance must remain allowed: {diagnostics:#?}"
        );
    }

    #[test]
    fn test_cpp_module_nested_symbol_identity_matches_exact_index_entries() {
        let options = TranspileOptions {
            cpp_module_symbol_index: Some(CppModuleSymbolIndex {
                modules: BTreeMap::from([(
                    "host".to_string(),
                    CppModuleIndexModule {
                        cpp_module: "host".to_string(),
                        namespace: Some("host_api".to_string()),
                        symbols: BTreeMap::from([
                            (
                                "other::increment".to_string(),
                                CppModuleIndexSymbol {
                                    kind: Some("function".to_string()),
                                    callable_signatures: vec!["int(int)".to_string()],
                                },
                            ),
                            (
                                "Counter::add".to_string(),
                                CppModuleIndexSymbol {
                                    kind: Some("method".to_string()),
                                    callable_signatures: vec!["int(int)".to_string()],
                                },
                            ),
                        ]),
                    },
                )]),
            }),
            ..TranspileOptions::default()
        };

        let output = transpile_full_with_options(
            r#"
use cpp::host;
fn nested(v: i32) -> i32 {
    unsafe { host::other::increment(v) }
}
fn member<T>(counter: &mut T, v: i32) -> i32 {
    unsafe { host::Counter::add(counter, v) }
}
"#,
            Some("consumer"),
            &UserTypeMap::default(),
            &HashSet::new(),
            None,
            &options,
        )
        .expect("exact nested function and member identities should transpile");

        assert!(
            output.contains("host_api::other::increment("),
            "Got: {output}"
        );
        assert!(output.contains("rusty::deref_call("), "Got: {output}");
        assert!(output.contains("__mdisp_add"), "Got: {output}");
        assert!(!output.contains("host_api::Counter::add("), "Got: {output}");
    }

    #[test]
    fn test_cpp_module_nested_symbol_identity_rejects_tail_only_index_entries() {
        let options = TranspileOptions {
            cpp_module_symbol_index: Some(CppModuleSymbolIndex {
                modules: BTreeMap::from([(
                    "host".to_string(),
                    CppModuleIndexModule {
                        cpp_module: "host".to_string(),
                        namespace: Some("host_api".to_string()),
                        symbols: BTreeMap::from([
                            (
                                "increment".to_string(),
                                CppModuleIndexSymbol {
                                    kind: Some("function".to_string()),
                                    callable_signatures: vec!["int(int)".to_string()],
                                },
                            ),
                            (
                                "add".to_string(),
                                CppModuleIndexSymbol {
                                    kind: Some("method".to_string()),
                                    callable_signatures: vec!["int(int)".to_string()],
                                },
                            ),
                        ]),
                    },
                )]),
            }),
            ..TranspileOptions::default()
        };

        let nested_err = transpile_full_with_options(
            "use cpp::host; fn f(v: i32) -> i32 { unsafe { host::other::increment(v) } }",
            Some("consumer"),
            &UserTypeMap::default(),
            &HashSet::new(),
            None,
            &options,
        )
        .expect_err("tail-only function entry must not match a nested symbol");
        assert!(nested_err.contains("symbol is not present"));
        assert!(nested_err.contains("symbol `other::increment`"));

        let member_err = transpile_full_with_options(
            "use cpp::host; fn f<T>(c: &mut T, v: i32) -> i32 { unsafe { host::Counter::add(c, v) } }",
            Some("consumer"),
            &UserTypeMap::default(),
            &HashSet::new(),
            None,
            &options,
        )
        .expect_err("tail-only method entry must not match a nested symbol");
        assert!(member_err.contains("symbol is not present"));
        assert!(member_err.contains("symbol `Counter::add`"));
    }

    #[test]
    fn test_cpp_module_import_identity_is_separate_from_export_namespace() {
        let mut modules = BTreeMap::new();
        let mut symbols = BTreeMap::new();
        symbols.insert(
            "log_line".to_string(),
            CppModuleIndexSymbol {
                kind: Some("function".to_string()),
                callable_signatures: vec![
                    "void(int,int,const int8_t*,const std::string&)".to_string(),
                ],
            },
        );
        modules.insert(
            "rrr::logging".to_string(),
            CppModuleIndexModule {
                cpp_module: "rrr.logging".to_string(),
                namespace: Some("rrr".to_string()),
                symbols,
            },
        );
        let options = TranspileOptions {
            cpp_module_symbol_index: Some(CppModuleSymbolIndex { modules }),
            ..TranspileOptions::default()
        };

        let output = transpile_full_with_options(
            r#"
use cpp::rrr::logging as cpp_logging;
fn write(message: &String) {
    unsafe { cpp_logging::log_line(3, 0, core::ptr::null(), message) }
}
"#,
            Some("consumer"),
            &UserTypeMap::default(),
            &HashSet::new(),
            None,
            &options,
        )
        .expect("indexed namespace should transpile");

        assert!(output.contains("import rrr.logging;"));
        assert!(output.contains("rrr::log_line("));
        assert!(!output.contains("rrr::logging::log_line("));
    }

    #[test]
    fn test_cpp_module_zero_argument_template_call_does_not_underflow() {
        let options = TranspileOptions {
            cpp_module_symbol_index: Some(CppModuleSymbolIndex {
                modules: BTreeMap::from([(
                    "rrr::reactor".to_string(),
                    CppModuleIndexModule {
                        cpp_module: "rrr.reactor".to_string(),
                        namespace: Some("rrr".to_string()),
                        symbols: BTreeMap::from([(
                            "create_sp_box_event".to_string(),
                            CppModuleIndexSymbol {
                                kind: Some("function_template".to_string()),
                                callable_signatures: vec!["BoxEvent<T>()".to_string()],
                            },
                        )]),
                    },
                )]),
            }),
            ..TranspileOptions::default()
        };

        let output = transpile_full_with_options(
            r#"
use cpp::rrr::reactor as cpp_reactor;
fn make<T>() {
    unsafe { cpp_reactor::create_sp_box_event::<T>(); }
}
"#,
            Some("consumer"),
            &UserTypeMap::default(),
            &HashSet::new(),
            None,
            &options,
        )
        .expect("zero-argument indexed template call should transpile");

        assert!(output.contains("import rrr.reactor;"));
        assert!(output.contains("rrr::create_sp_box_event<T>()"));
    }

    #[test]
    fn test_cpp_module_export_namespace_keeps_index_resolution_fail_closed() {
        let source = r#"
use cpp::rrr::logging as cpp_logging;
fn write(message: &String) {
    unsafe { cpp_logging::log_line(3, 0, core::ptr::null(), message) }
}
"#;

        let symbol = CppModuleIndexSymbol {
            kind: Some("function".to_string()),
            callable_signatures: vec!["void(int,int,const int8_t*,const std::string&)".to_string()],
        };
        let wrong_module_options = TranspileOptions {
            cpp_module_symbol_index: Some(CppModuleSymbolIndex {
                modules: BTreeMap::from([(
                    "rrr".to_string(),
                    CppModuleIndexModule {
                        cpp_module: "rrr".to_string(),
                        namespace: Some("rrr".to_string()),
                        symbols: BTreeMap::from([("log_line".to_string(), symbol.clone())]),
                    },
                )]),
            }),
            ..TranspileOptions::default()
        };
        let err = transpile_full_with_options(
            source,
            Some("consumer"),
            &UserTypeMap::default(),
            &HashSet::new(),
            None,
            &wrong_module_options,
        )
        .expect_err("matching namespace must not substitute for module identity");
        assert!(err.contains("module path is not present"));
        assert!(err.contains("module `rrr::logging`"));

        let wrong_symbol_options = TranspileOptions {
            cpp_module_symbol_index: Some(CppModuleSymbolIndex {
                modules: BTreeMap::from([(
                    "rrr::logging".to_string(),
                    CppModuleIndexModule {
                        cpp_module: "rrr.logging".to_string(),
                        namespace: Some("rrr".to_string()),
                        symbols: BTreeMap::new(),
                    },
                )]),
            }),
            ..TranspileOptions::default()
        };
        let err = transpile_full_with_options(
            source,
            Some("consumer"),
            &UserTypeMap::default(),
            &HashSet::new(),
            None,
            &wrong_symbol_options,
        )
        .expect_err("export namespace must not bypass indexed symbol lookup");
        assert!(err.contains("symbol is not present"));
        assert!(err.contains("symbol `log_line`"));

        let wrong_signature_options = TranspileOptions {
            cpp_module_symbol_index: Some(CppModuleSymbolIndex {
                modules: BTreeMap::from([(
                    "rrr::logging".to_string(),
                    CppModuleIndexModule {
                        cpp_module: "rrr.logging".to_string(),
                        namespace: Some("rrr".to_string()),
                        symbols: BTreeMap::from([(
                            "log_line".to_string(),
                            CppModuleIndexSymbol {
                                kind: Some("function".to_string()),
                                callable_signatures: vec!["void(int,int)".to_string()],
                            },
                        )]),
                    },
                )]),
            }),
            ..TranspileOptions::default()
        };
        let err = transpile_full_with_options(
            source,
            Some("consumer"),
            &UserTypeMap::default(),
            &HashSet::new(),
            None,
            &wrong_signature_options,
        )
        .expect_err("export namespace must not bypass indexed signature matching");
        assert!(err.contains("call cannot be matched to indexed callable family"));
        assert!(err.contains("arity 4"));
        assert!(err.contains("void(int,int)"));
    }

    #[test]
    fn test_cpp_module_call_errors_when_module_path_missing_from_index() {
        let mut modules = BTreeMap::new();
        modules.insert(
            "std".to_string(),
            CppModuleIndexModule {
                cpp_module: "std".to_string(),
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
                cpp_module: "std".to_string(),
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
                cpp_module: "std".to_string(),
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
                cpp_module: "std".to_string(),
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
                cpp_module: "std".to_string(),
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
                cpp_module: "std".to_string(),
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
                cpp_module: "std".to_string(),
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
                cpp_module: "std".to_string(),
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
