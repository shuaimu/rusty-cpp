use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

/// Apply the graph-shaping portion of a Cargo build/test invocation to
/// `cargo metadata`.
///
/// Cargo spells the target selector differently for metadata
/// (`--filter-platform`) than it does for build commands (`--target`).  Keeping
/// this translation in one checked helper prevents provenance/authentication
/// queries from silently resolving a different feature or target graph.
fn apply_resolution_flags_to_metadata(
    command: &mut std::process::Command,
    cargo_flags: &[String],
) -> Result<(), String> {
    let mut index = 0usize;
    while index < cargo_flags.len() {
        let flag = cargo_flags[index].as_str();
        match flag {
            "--features" | "-F" | "--config" => {
                let value = cargo_flags.get(index + 1).ok_or_else(|| {
                    format!("Cargo resolution flag '{flag}' requires a value")
                })?;
                command.arg(flag).arg(value);
                index += 2;
            }
            "--target" => {
                let value = cargo_flags.get(index + 1).ok_or_else(|| {
                    "Cargo resolution flag '--target' requires a value".to_string()
                })?;
                command.arg("--filter-platform").arg(value);
                index += 2;
            }
            "--filter-platform" => {
                let value = cargo_flags.get(index + 1).ok_or_else(|| {
                    "Cargo resolution flag '--filter-platform' requires a value".to_string()
                })?;
                command.arg("--filter-platform").arg(value);
                index += 2;
            }
            "--all-features" | "--no-default-features" | "--locked" | "--offline"
            | "--frozen" => {
                command.arg(flag);
                index += 1;
            }
            _ if flag.starts_with("--features=")
                || flag.starts_with("--config=")
                || flag.starts_with("--filter-platform=") =>
            {
                command.arg(flag);
                index += 1;
            }
            _ if flag.starts_with("--target=") => {
                command.arg(format!(
                    "--filter-platform={}",
                    flag.trim_start_matches("--target=")
                ));
                index += 1;
            }
            _ => {
                return Err(format!(
                    "unsupported Cargo resolution flag '{flag}'; refusing to authenticate with a potentially different Cargo graph"
                ));
            }
        }
    }
    Ok(())
}

/// Stable fingerprint of Cargo's complete resolved metadata graph under the
/// supplied invocation context.  Parity reuse records this so changing a
/// manifest, lockfile, patch/config resolution, or selected feature graph
/// cannot reuse expanded/transpiled artifacts authenticated under another
/// graph.
pub fn cargo_resolution_graph_fingerprint(
    manifest_path: &Path,
    package_filter: Option<&str>,
    cargo_flags: &[String],
) -> Result<String, String> {
    let project_dir = manifest_path.parent().unwrap_or(Path::new("."));
    let mut command = std::process::Command::new("cargo");
    command
        .arg("metadata")
        .arg("--format-version")
        .arg("1")
        .arg("--manifest-path")
        .arg(manifest_path);
    apply_resolution_flags_to_metadata(&mut command, cargo_flags)?;
    let output = command
        .current_dir(project_dir)
        .output()
        .map_err(|error| format!("Failed to run cargo metadata: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "cargo metadata failed:\n{}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    let metadata: CargoMetadataResolved = serde_json::from_slice(&output.stdout)
        .map_err(|error| format!("Failed to parse cargo metadata: {error}"))?;
    let selected = select_resolved_package(&metadata, manifest_path, package_filter)?;
    let canonical = serde_json::to_vec(&serde_json::from_slice::<serde_json::Value>(
        &output.stdout,
    )
    .map_err(|error| format!("Failed to canonicalize cargo metadata: {error}"))?)
    .map_err(|error| format!("Failed to canonicalize cargo metadata: {error}"))?;
    let mut digest = Sha256::new();
    digest.update(selected.id.as_bytes());
    digest.update([0]);
    digest.update(canonical);
    Ok(format!("{:x}", digest.finalize()))
}

/// A discovered crate target from `cargo metadata`.
#[derive(Debug, Clone)]
pub struct CrateTarget {
    pub name: String,
    pub kind: TargetKind,
    pub src_path: PathBuf,
    /// C++20 module name derived from target name
    pub module_name: String,
}

/// A local path dependency package discovered from the resolved dependency graph.
#[derive(Debug, Clone)]
pub struct LocalDependencyPackage {
    pub name: String,
    pub manifest_path: PathBuf,
    /// Features resolved by Cargo for this dependency package.
    pub resolved_features: Vec<String>,
    /// Crate roots that may reference this package in expanded source
    /// (including renamed dependency aliases, e.g. `serde` for package `serde_core`).
    pub extern_crate_roots: Vec<String>,
}

/// Cargo-resolved view of one direct dependency from a package manifest.
///
/// Unlike the raw TOML dependency table, this includes values inherited from
/// `[workspace.dependencies]` and therefore preserves the selected package
/// identity for renamed dependencies.
#[derive(Debug, Clone)]
pub struct ManifestDependency {
    pub dependency_key: String,
    pub package_name: String,
    pub source: Option<String>,
    pub path: Option<PathBuf>,
    pub kind: Option<String>,
    pub target: Option<String>,
    pub optional: bool,
}

/// Cargo-resolved target identity for one package manifest.
#[derive(Debug, Clone)]
pub struct ManifestTarget {
    pub name: String,
    pub kind: Vec<String>,
    pub crate_types: Vec<String>,
    pub src_path: PathBuf,
}

/// Exact Cargo package, dependency, and target identities for one manifest.
#[derive(Debug, Clone)]
pub struct ManifestIdentity {
    pub package_name: String,
    pub edition: String,
    pub rust_version: Option<String>,
    pub workspace_root: PathBuf,
    pub feature_names: Vec<String>,
    pub dependencies: Vec<ManifestDependency>,
    pub targets: Vec<ManifestTarget>,
}

/// One Cargo-selected normal local dependency edge.
///
/// `dependency_key` is the crate name visible to Rust source (and therefore
/// preserves `package = ...` renames); `package_name` is the selected Cargo
/// package identity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EffectiveLocalNormalDependency {
    pub dependency_key: String,
    pub package_name: String,
    pub manifest_path: PathBuf,
    pub resolved_features: Vec<String>,
}

/// Cargo's effective normal local-dependency graph for one concrete target.
///
/// The graph is deliberately keyed by canonical manifest path so all crate
/// mode phases can consume the same selection result without re-evaluating
/// target cfg expressions or workspace inheritance independently.
#[derive(Debug, Clone)]
pub struct EffectiveLocalNormalDependencyGraph {
    pub target_triple: String,
    root_manifest: PathBuf,
    direct_dependencies: HashMap<PathBuf, Vec<EffectiveLocalNormalDependency>>,
    /// Cargo's normal-unit feature set for the root package itself (the
    /// selected features of the probe's edge to it).
    root_features: Option<Vec<String>>,
    /// Every selected normal dependency edge (registry ones included) of each
    /// local package, as the extern-prelude names it occupies.
    extern_dependencies: HashMap<PathBuf, Vec<ResolvedExternDependency>>,
}

impl EffectiveLocalNormalDependencyGraph {
    pub fn root_manifest(&self) -> &Path {
        &self.root_manifest
    }

    pub fn direct_dependencies(
        &self,
        manifest_path: &Path,
    ) -> Option<&[EffectiveLocalNormalDependency]> {
        self.direct_dependencies
            .get(&canonicalized_path(manifest_path))
            .map(Vec::as_slice)
    }

    /// Cargo's normal-unit feature set for the root package.
    pub fn root_resolved_features(&self) -> Option<&[String]> {
        self.root_features.as_deref()
    }

    /// The extern-prelude names a local package's selected normal
    /// dependencies occupy in this graph (registry dependencies included).
    pub fn extern_dependencies(&self, manifest_path: &Path) -> &[ResolvedExternDependency] {
        self.extern_dependencies
            .get(&canonicalized_path(manifest_path))
            .map(Vec::as_slice)
            .unwrap_or_default()
    }

    /// Return Cargo's exact normal-unit feature set for a selected local
    /// package. Feature sets are attached to incoming edges because Cargo's
    /// metadata node feature list is a build/dev/proc-macro union. The graph
    /// resolver rejects divergent incoming normal-unit feature witnesses, so
    /// any matching edge is authoritative here.
    pub fn resolved_features_for_manifest(&self, manifest_path: &Path) -> Option<&[String]> {
        let manifest_path = canonicalized_path(manifest_path);
        self.direct_dependencies
            .values()
            .flat_map(|dependencies| dependencies.iter())
            .find(|dependency| dependency.manifest_path == manifest_path)
            .map(|dependency| dependency.resolved_features.as_slice())
    }
}

/// One direct dependency as Cargo exposes it to rustc's extern prelude.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ResolvedExternDependency {
    pub extern_crate_root: String,
    pub package_name: String,
}

/// Dependency-kind provenance for the Rust compilation unit being
/// transpiled. Cargo's resolved package node contains normal, development,
/// and build edges at the same time; which edges can occupy an extern-prelude
/// name depends on the selected target invocation.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum CargoDependencyContext {
    /// Library/binary compilation: ordinary dependencies only.
    Normal,
    /// Test/example/bench (and `--tests`) compilation: ordinary plus
    /// development dependencies.
    Development,
    /// Build-script compilation: build dependencies only.
    Build,
    /// The caller cannot prove which Cargo target produced the source. Every
    /// dependency kind is considered occupied so compiler-owned identities
    /// fail closed.
    Unknown,
}

impl CargoDependencyContext {
    fn includes(self, kind: Option<&str>) -> bool {
        match self {
            Self::Normal => kind.is_none(),
            Self::Development => kind.is_none() || kind == Some("dev"),
            Self::Build => kind == Some("build"),
            Self::Unknown => true,
        }
    }
}

/// Exact Cargo target provenance when available, plus the dependency kinds
/// made visible by the actual Cargo invocation. A missing target is deliberate
/// conservative provenance, not an implicit library/default-target guess.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct CargoCompilationContext {
    target_name: Option<String>,
    target_kind: Option<TargetKind>,
    target_src_path: Option<PathBuf>,
    dependency_context: CargoDependencyContext,
}

impl CargoCompilationContext {
    pub fn exact(target: &CrateTarget, include_dev_dependencies: bool) -> Self {
        let dependency_context = match target.kind {
            TargetKind::Test | TargetKind::Example | TargetKind::Bench => {
                CargoDependencyContext::Development
            }
            TargetKind::Other(ref kind) if kind == "custom-build" => {
                CargoDependencyContext::Build
            }
            _ if include_dev_dependencies => CargoDependencyContext::Development,
            _ => CargoDependencyContext::Normal,
        };
        Self {
            target_name: Some(target.name.clone()),
            target_kind: Some(target.kind.clone()),
            target_src_path: Some(canonicalized_path(&target.src_path)),
            dependency_context,
        }
    }

    pub fn normal_package() -> Self {
        Self {
            target_name: None,
            target_kind: None,
            target_src_path: None,
            dependency_context: CargoDependencyContext::Normal,
        }
    }

    pub fn conservative() -> Self {
        Self {
            target_name: None,
            target_kind: None,
            target_src_path: None,
            dependency_context: CargoDependencyContext::Unknown,
        }
    }

    pub fn dependency_context(&self) -> CargoDependencyContext {
        self.dependency_context
    }

    pub fn target_src_path(&self) -> Option<&Path> {
        self.target_src_path.as_deref()
    }

    /// Cargo target selector matching this exact compilation root. Unknown
    /// source ownership must fail before invoking Cargo: omitting a selector
    /// would silently expand Cargo's default target instead of the supplied
    /// source.
    pub fn cargo_target_args(&self) -> Result<Vec<String>, String> {
        let (Some(name), Some(kind)) = (&self.target_name, &self.target_kind) else {
            return Err(
                "cannot faithfully cargo expand a source that is not one exact Cargo target root"
                    .to_string(),
            );
        };
        Ok(match kind {
            TargetKind::Lib => vec!["--lib".to_string()],
            TargetKind::Bin => vec!["--bin".to_string(), name.clone()],
            TargetKind::Test => vec!["--test".to_string(), name.clone()],
            TargetKind::Example => vec!["--example".to_string(), name.clone()],
            TargetKind::Bench => vec!["--bench".to_string(), name.clone()],
            TargetKind::Other(kind) => {
                return Err(format!(
                    "cargo expand target '{}' ({kind}) has no faithful target selector; refusing to expand a different Cargo target",
                    name
                ));
            }
        })
    }

    pub fn describe(&self) -> String {
        match (&self.target_name, &self.target_kind) {
            (Some(name), Some(kind)) => format!("target '{name}' ({kind:?})"),
            _ => "unknown Cargo target (all dependency kinds)".to_string(),
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum TargetKind {
    Lib,
    Bin,
    Test,
    Example,
    Bench,
    Other(String),
}

impl TargetKind {
    fn from_cargo(kinds: &[String]) -> Self {
        for k in kinds {
            match k.as_str() {
                "lib" | "rlib" | "dylib" | "cdylib" | "staticlib" | "proc-macro" => {
                    return TargetKind::Lib;
                }
                "bin" => return TargetKind::Bin,
                "test" => return TargetKind::Test,
                "example" => return TargetKind::Example,
                "bench" => return TargetKind::Bench,
                _ => {}
            }
        }
        TargetKind::Other(kinds.join(","))
    }

    pub fn is_test_capable(&self) -> bool {
        matches!(self, TargetKind::Lib | TargetKind::Bin | TargetKind::Test)
    }

    pub fn cargo_expand_flag(&self) -> Option<&'static str> {
        match self {
            TargetKind::Lib => Some("--lib"),
            TargetKind::Bin => Some("--bin"),
            TargetKind::Test => Some("--test"),
            TargetKind::Example => Some("--example"),
            _ => None,
        }
    }

    fn module_collision_suffix(&self) -> &'static str {
        match self {
            TargetKind::Lib => "lib",
            TargetKind::Bin => "bin",
            TargetKind::Test => "test",
            TargetKind::Example => "example",
            TargetKind::Bench => "bench",
            TargetKind::Other(_) => "target",
        }
    }

    fn sort_rank(&self) -> u8 {
        match self {
            TargetKind::Lib => 0,
            TargetKind::Bin => 1,
            TargetKind::Test => 2,
            TargetKind::Example => 3,
            TargetKind::Bench => 4,
            TargetKind::Other(_) => 5,
        }
    }
}

/// Raw cargo metadata JSON structures (subset).
#[derive(Deserialize)]
struct CargoMetadata {
    packages: Vec<Package>,
    workspace_root: PathBuf,
}

#[derive(Deserialize)]
struct Package {
    name: String,
    #[allow(dead_code)]
    version: String,
    edition: String,
    rust_version: Option<String>,
    targets: Vec<Target>,
    manifest_path: PathBuf,
    #[serde(default)]
    dependencies: Vec<PackageDependency>,
    #[serde(default)]
    features: HashMap<String, Vec<String>>,
}

#[derive(Deserialize)]
struct PackageDependency {
    name: String,
    source: Option<String>,
    #[serde(default)]
    rename: Option<String>,
    #[serde(default)]
    optional: bool,
    kind: Option<String>,
    target: Option<String>,
    path: Option<PathBuf>,
}

#[derive(Deserialize)]
struct CargoMetadataResolved {
    packages: Vec<ResolvedPackage>,
    resolve: Option<ResolveGraph>,
}

#[derive(Deserialize)]
struct ResolvedPackage {
    id: String,
    name: String,
    version: String,
    manifest_path: PathBuf,
    source: Option<String>,
    targets: Vec<Target>,
}

/// `--crate-graph-git <URL>`: the git repositories whose packages join
/// `--crate-graph` as graph crates, exactly like local path packages.
///
/// The effective graph holds only packages without a Cargo source by default;
/// a registry or git package is an external crate the graph neither descends
/// into nor emits. A package whose source is `git+<URL>[?<reference>]#<commit>`
/// joins when `<URL>` names one of these repositories, and is read from the
/// checkout `cargo metadata` reports for it. Naming a repository rather than
/// packages means every package Cargo selects from it joins, so a consumer
/// cannot name part of a crate family.
///
/// `--crate-graph` itself refuses a graph crate's target-normal dependency on
/// a package from a git repository no one named
/// ([`Self::refusing_unnamed`]). Left external, such a package is silently
/// missing from the output, and its dependents are transpiled against a
/// crate nothing emits: SRPC, with Lion's repository unnamed, exited 0 with
/// three mis-generated providers. Verus's repository is exempt under
/// `--verus-exec`, which erases or lowers every use of `vstd` itself
/// (`crate::verus_lower`). Outside `--crate-graph`, crate mode's
/// source-contract preflight and `--expand` keep an unnamed git package
/// external, as before.
///
/// Repository URLs compare as Cargo identifies a git source (its
/// `CanonicalUrl`): scheme and host ignore case, one trailing `/` and then a
/// trailing `.git` are dropped, and a `github.com` URL compares as `https`
/// with its path lowercased. The reference and the commit play no part;
/// `Cargo.lock` pins the commit.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct CrateGraphGitRepositories {
    /// Canonical repository URL -> the URL as the caller spelled it.
    named: BTreeMap<String, String>,
    /// `--crate-graph`: a graph crate's dependency on a package from a git
    /// repository that is neither named nor in `exempt` is an error.
    refuse_unnamed: bool,
    /// Canonical URLs of the repositories `refuse_unnamed` lets stay external.
    exempt: BTreeSet<String>,
}

impl CrateGraphGitRepositories {
    /// Validate and canonicalize the `--crate-graph-git` values.
    pub fn from_urls(urls: &[String]) -> Result<Self, String> {
        let mut named = BTreeMap::new();
        for url in urls {
            if url.is_empty() {
                return Err("--crate-graph-git: empty repository URL".to_string());
            }
            if url.starts_with("git+") {
                return Err(format!(
                    "--crate-graph-git {url}: name the repository as Cargo.toml's `git = \"...\"` spells it, without Cargo's `git+` source prefix"
                ));
            }
            if url.contains(['?', '#']) {
                return Err(format!(
                    "--crate-graph-git {url}: name a repository, not a reference or a commit; Cargo.lock pins the commit"
                ));
            }
            if !url.contains("://") {
                return Err(format!(
                    "--crate-graph-git {url}: not a `scheme://` URL, the only form Cargo accepts for a git dependency"
                ));
            }
            named
                .entry(canonical_git_repository_url(url))
                .or_insert_with(|| url.clone());
        }
        Ok(Self {
            named,
            ..Self::default()
        })
    }

    /// `--crate-graph`'s policy: refuse a graph crate's dependency on a
    /// package from an unnamed git repository, except Verus's under
    /// `--verus-exec` (`verus_exec`).
    pub fn refusing_unnamed(mut self, verus_exec: bool) -> Self {
        self.refuse_unnamed = true;
        if verus_exec {
            self.exempt
                .insert(canonical_git_repository_url(crate::verus_exec::VERUS_GIT_URL));
        }
        self
    }

    /// Whether no repository is named.
    pub fn is_empty(&self) -> bool {
        self.named.is_empty()
    }

    /// Whether a package with this Cargo source comes from a git repository
    /// that is neither named nor exempt, under `--crate-graph`'s policy.
    fn refuses(&self, source: Option<&str>) -> bool {
        let Some(repository) = source.and_then(git_source_repository) else {
            return false;
        };
        let canonical = canonical_git_repository_url(repository);
        self.refuse_unnamed
            && !self.named.contains_key(&canonical)
            && !self.exempt.contains(&canonical)
    }

    /// The canonical URL of the named repository a Cargo source comes from.
    fn repository_of(&self, source: &str) -> Option<&str> {
        let canonical = canonical_git_repository_url(git_source_repository(source)?);
        self.named
            .get_key_value(&canonical)
            .map(|(canonical, _)| canonical.as_str())
    }

    /// Whether a package with this Cargo source is a graph crate: a local
    /// path package, or a package from a named git repository.
    fn admits(&self, source: Option<&str>) -> bool {
        match source {
            None => true,
            Some(_) if self.named.is_empty() => false,
            Some(source) => self.repository_of(source).is_some(),
        }
    }
}

/// The repository URL of a `git+<URL>[?<reference>][#<commit>]` Cargo source
/// as `cargo metadata` spells it; `None` for any other source.
fn git_source_repository(source: &str) -> Option<&str> {
    let url = source.strip_prefix("git+")?;
    Some(&url[..url.find(['?', '#']).unwrap_or(url.len())])
}

/// Cargo's identity of a git repository URL (`CanonicalUrl::new`), on the
/// URL's text: scheme and host lowercased, one trailing `/` dropped, a
/// `github.com` URL turned to `https` with its path lowercased, and then a
/// trailing `.git` dropped.
fn canonical_git_repository_url(url: &str) -> String {
    let Some((scheme, rest)) = url.split_once("://") else {
        return url.to_string();
    };
    let (authority, path) = rest.split_at(rest.find('/').unwrap_or(rest.len()));
    // The host ignores case; user information does not.
    let (user, host) = match authority.rsplit_once('@') {
        Some((user, host)) => (Some(user), host.to_ascii_lowercase()),
        None => (None, authority.to_ascii_lowercase()),
    };
    let host_name = host.split(':').next().unwrap_or_default();
    let mut path = path.strip_suffix('/').unwrap_or(path).to_string();
    let scheme = if host_name == "github.com" {
        path = path.to_lowercase();
        "https".to_string()
    } else {
        scheme.to_ascii_lowercase()
    };
    if let Some(stripped) = path.strip_suffix(".git") {
        path.truncate(stripped.len());
    }
    match user {
        Some(user) => format!("{scheme}://{user}@{host}{path}"),
        None => format!("{scheme}://{host}{path}"),
    }
}

/// `cargo tree`'s `{p}` spelling of a git source (Cargo's `SourceId`
/// Display): the URL as written, `?branch=`/`?tag=`/`?rev=` with the value
/// form-urlencoded, then `#` and the locked commit cut to 8 characters.
/// `cargo metadata` spells the same source with the raw value and the full
/// commit. Measured with Cargo 1.91, 1.95, 1.97 and 1.99:
/// `git+file:///r/?branch=feat/x+y#419d705b73fb…` prints as
/// `file:///r/?branch=feat%2Fx%2By#419d705b`.
fn cargo_tree_git_source(source: &str) -> Option<String> {
    let url = source.strip_prefix("git+")?;
    let (url, commit) = match url.rsplit_once('#') {
        Some((url, commit)) => (url, Some(commit)),
        None => (url, None),
    };
    let (url, reference) = match url.split_once('?') {
        Some((url, reference)) => (url, Some(reference)),
        None => (url, None),
    };
    let mut rendered = url.to_string();
    if let Some(reference) = reference {
        let (kind, value) = reference.split_once('=')?;
        if !matches!(kind, "branch" | "tag" | "rev") {
            return None;
        }
        rendered.push('?');
        rendered.push_str(kind);
        rendered.push('=');
        for byte in value.bytes() {
            // The `url` crate's form_urlencoded::byte_serialize.
            match byte {
                b'*' | b'-' | b'.' | b'_' | b'0'..=b'9' | b'A'..=b'Z' | b'a'..=b'z' => {
                    rendered.push(char::from(byte))
                }
                b' ' => rendered.push('+'),
                _ => rendered.push_str(&format!("%{byte:02X}")),
            }
        }
    }
    if let Some(commit) = commit {
        rendered.push('#');
        rendered.push_str(commit.get(..8).unwrap_or(commit));
    }
    Some(rendered)
}

#[derive(Deserialize)]
struct ResolveGraph {
    nodes: Vec<ResolveNode>,
}

#[derive(Deserialize)]
struct ResolveNode {
    id: String,
    #[serde(default)]
    deps: Vec<ResolveDep>,
    #[serde(default)]
    features: Vec<String>,
}

#[derive(Deserialize)]
struct ResolveDep {
    #[serde(default)]
    name: String,
    pkg: String,
    #[serde(default)]
    dep_kinds: Vec<ResolveDepKind>,
}

#[derive(Deserialize)]
struct ResolveDepKind {
    kind: Option<String>,
    target: Option<String>,
}

#[derive(Deserialize)]
struct Target {
    name: String,
    kind: Vec<String>,
    #[serde(default)]
    crate_types: Vec<String>,
    src_path: String,
}

fn target_has_kind(target: &Target, needle: &str) -> bool {
    target.kind.iter().any(|kind| kind == needle)
}

fn target_is_proc_macro(target: &Target) -> bool {
    target_has_kind(target, "proc-macro")
}

#[cfg(test)]
fn target_is_library_like(target: &Target) -> bool {
    if target_is_proc_macro(target) {
        return false;
    }
    target.kind.iter().any(|kind| {
        matches!(
            kind.as_str(),
            "lib" | "rlib" | "dylib" | "cdylib" | "staticlib"
        )
    })
}

fn target_is_compiletest_harness(target: &Target) -> bool {
    target_has_kind(target, "test") && target.name == "compiletest"
}

#[derive(Debug, Clone)]
struct RawTarget {
    name: String,
    kind: TargetKind,
    src_path: PathBuf,
}

fn canonicalized_path(path: &Path) -> PathBuf {
    std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

fn select_target_package<'a>(
    metadata: &'a CargoMetadata,
    manifest_path: &Path,
    package_filter: Option<&str>,
) -> Result<&'a Package, String> {
    if let Some(filter) = package_filter {
        return metadata
            .packages
            .iter()
            .find(|p| p.name == filter)
            .ok_or_else(|| format!("Package '{}' not found in metadata", filter));
    }

    let requested_manifest = canonicalized_path(manifest_path);
    if let Some(pkg) = metadata
        .packages
        .iter()
        .find(|p| canonicalized_path(&p.manifest_path) == requested_manifest)
    {
        return Ok(pkg);
    }

    metadata
        .packages
        .first()
        .ok_or_else(|| "No packages found in cargo metadata".to_string())
}

fn select_resolved_package<'a>(
    metadata: &'a CargoMetadataResolved,
    manifest_path: &Path,
    package_filter: Option<&str>,
) -> Result<&'a ResolvedPackage, String> {
    if let Some(filter) = package_filter {
        return metadata
            .packages
            .iter()
            .find(|p| p.name == filter)
            .ok_or_else(|| format!("Package '{}' not found in metadata", filter));
    }

    let requested_manifest = canonicalized_path(manifest_path);
    if let Some(pkg) = metadata
        .packages
        .iter()
        .find(|p| canonicalized_path(&p.manifest_path) == requested_manifest)
    {
        return Ok(pkg);
    }

    metadata
        .packages
        .first()
        .ok_or_else(|| "No packages found in cargo metadata".to_string())
}

fn normalize_module_base(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    for ch in name.chars() {
        if ch.is_ascii_alphanumeric() || ch == '_' {
            out.push(ch);
        } else {
            out.push('_');
        }
    }

    if out.is_empty() {
        out.push_str("target");
    }

    if out
        .as_bytes()
        .first()
        .is_some_and(|first| first.is_ascii_digit())
    {
        out.insert(0, '_');
    }

    out
}

fn assign_module_names(mut raw_targets: Vec<RawTarget>) -> Vec<CrateTarget> {
    // Keep target processing deterministic so module naming and downstream artifact
    // generation are stable across reruns and environments.
    raw_targets.sort_by(|a, b| {
        a.kind
            .sort_rank()
            .cmp(&b.kind.sort_rank())
            .then_with(|| a.name.cmp(&b.name))
            .then_with(|| a.src_path.cmp(&b.src_path))
    });

    let mut used_module_names: HashSet<String> = HashSet::new();
    let mut targets = Vec::with_capacity(raw_targets.len());

    for raw in raw_targets {
        let base = normalize_module_base(&raw.name);
        let mut module_name = base.clone();

        if used_module_names.contains(&module_name) {
            module_name = format!("{}_{}", base, raw.kind.module_collision_suffix());
        }

        if used_module_names.contains(&module_name) {
            let stem = module_name.clone();
            let mut index = 2u32;
            loop {
                let candidate = format!("{}_{}", stem, index);
                if !used_module_names.contains(&candidate) {
                    module_name = candidate;
                    break;
                }
                index += 1;
            }
        }

        used_module_names.insert(module_name.clone());
        targets.push(CrateTarget {
            name: raw.name,
            kind: raw.kind,
            src_path: raw.src_path,
            module_name,
        });
    }

    targets
}

/// Discover crate targets by running `cargo metadata`.
/// Returns the package name and a list of discovered targets.
#[cfg(test)]
pub fn discover_targets(
    manifest_path: &Path,
    package_filter: Option<&str>,
) -> Result<(String, Vec<CrateTarget>), String> {
    discover_targets_with_context(manifest_path, package_filter, &[])
}

/// Context-preserving form of [`discover_targets`].
pub fn discover_targets_with_context(
    manifest_path: &Path,
    package_filter: Option<&str>,
    cargo_flags: &[String],
) -> Result<(String, Vec<CrateTarget>), String> {
    let project_dir = manifest_path.parent().unwrap_or(Path::new("."));

    let mut command = std::process::Command::new("cargo");
    command
        .arg("metadata")
        .arg("--format-version")
        .arg("1")
        .arg("--no-deps")
        .arg("--manifest-path")
        .arg(manifest_path);
    apply_resolution_flags_to_metadata(&mut command, cargo_flags)?;
    let output = command
        .current_dir(project_dir)
        .output()
        .map_err(|e| format!("Failed to run cargo metadata: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("cargo metadata failed:\n{}", stderr));
    }

    let metadata: CargoMetadata = serde_json::from_slice(&output.stdout)
        .map_err(|e| format!("Failed to parse cargo metadata: {}", e))?;

    // Select target package. Without an explicit package filter, prefer the package
    // whose Cargo.toml matches the requested manifest path instead of metadata order.
    let pkg = select_target_package(&metadata, manifest_path, package_filter)?;

    let mut raw_targets = Vec::new();
    let mut skipped = Vec::new();

    for target in &pkg.targets {
        if target_is_proc_macro(target) {
            skipped.push((
                target.name.clone(),
                TargetKind::Other("proc-macro".to_string()),
            ));
            continue;
        }
        if target_is_compiletest_harness(target) {
            skipped.push((
                target.name.clone(),
                TargetKind::Other("compiletest-harness".to_string()),
            ));
            continue;
        }
        let kind = TargetKind::from_cargo(&target.kind);

        if kind.is_test_capable() {
            raw_targets.push(RawTarget {
                name: target.name.clone(),
                kind,
                src_path: PathBuf::from(&target.src_path),
            });
        } else {
            skipped.push((target.name.clone(), kind));
        }
    }

    let targets = assign_module_names(raw_targets);

    // Report skipped targets
    for (name, kind) in &skipped {
        eprintln!(
            "  Skipping target '{}' ({:?}): not test-capable",
            name, kind
        );
    }

    Ok((pkg.name.clone(), targets))
}

/// Ask Cargo for the exact package/dependency/target identities represented by
/// a manifest without resolving or downloading the dependency graph.
///
/// `--no-deps` is sufficient here: Cargo still expands workspace-inherited
/// dependency declarations and reports the package's own targets, while the
/// command remains independent of registry availability.
/// Inspect manifest identity under an explicit Cargo resolution context.
pub fn inspect_manifest_identity_with_context(
    manifest_path: &Path,
    package_filter: Option<&str>,
    cargo_flags: &[String],
) -> Result<ManifestIdentity, String> {
    let project_dir = manifest_path.parent().unwrap_or(Path::new("."));
    let mut command = std::process::Command::new("cargo");
    command
        .arg("metadata")
        .arg("--format-version")
        .arg("1")
        .arg("--no-deps")
        .arg("--manifest-path")
        .arg(manifest_path);
    apply_resolution_flags_to_metadata(&mut command, cargo_flags)?;
    let output = command
        .current_dir(project_dir)
        .output()
        .map_err(|error| format!("Failed to run cargo metadata: {error}"))?;

    if !output.status.success() {
        return Err(format!(
            "cargo metadata failed:\n{}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    let metadata: CargoMetadata = serde_json::from_slice(&output.stdout)
        .map_err(|error| format!("Failed to parse cargo metadata: {error}"))?;
    let package = select_target_package(&metadata, manifest_path, package_filter)?;
    let dependencies = package
        .dependencies
        .iter()
        .map(|dependency| ManifestDependency {
            dependency_key: dependency
                .rename
                .clone()
                .unwrap_or_else(|| dependency.name.clone()),
            package_name: dependency.name.clone(),
            source: dependency.source.clone(),
            path: dependency.path.clone(),
            kind: dependency.kind.clone(),
            target: dependency.target.clone(),
            optional: dependency.optional,
        })
        .collect();
    let targets = package
        .targets
        .iter()
        .map(|target| ManifestTarget {
            name: target.name.clone(),
            kind: target.kind.clone(),
            crate_types: target.crate_types.clone(),
            src_path: PathBuf::from(&target.src_path),
        })
        .collect();

    Ok(ManifestIdentity {
        package_name: package.name.clone(),
        edition: package.edition.clone(),
        rust_version: package.rust_version.clone(),
        workspace_root: canonicalized_path(&metadata.workspace_root),
        feature_names: {
            let mut names = package.features.keys().cloned().collect::<Vec<_>>();
            names.sort();
            names
        },
        dependencies,
        targets,
    })
}

/// Inspect the package which owns `manifest_path` under Cargo's default
/// resolution context.
pub fn inspect_manifest_identity(manifest_path: &Path) -> Result<ManifestIdentity, String> {
    inspect_manifest_identity_with_context(manifest_path, None, &[])
}

/// Ask Cargo's resolved graph for the direct `--extern` names of a package.
/// This is intentionally not reconstructed from dependency-table spelling:
/// a package may expose a library target with a different crate name, while a
/// renamed dependency may choose yet another extern-prelude root.
#[cfg(test)]
pub fn inspect_resolved_extern_dependencies(
    manifest_path: &Path,
) -> Result<Vec<ResolvedExternDependency>, String> {
    inspect_resolved_extern_dependencies_with_context(manifest_path, None, &[])
}

/// Context-preserving form of [`inspect_resolved_extern_dependencies`].
pub fn inspect_resolved_extern_dependencies_with_context(
    manifest_path: &Path,
    package_filter: Option<&str>,
    cargo_flags: &[String],
) -> Result<Vec<ResolvedExternDependency>, String> {
    inspect_resolved_extern_dependencies_for_compilation(
        manifest_path,
        package_filter,
        cargo_flags,
        &CargoCompilationContext::normal_package(),
    )
}

/// Context-preserving direct-extern inspection for one actual compilation
/// target. Cargo metadata deliberately reports all dependency kinds on the
/// package node, so authentication must filter those edges using the same
/// target class as the Cargo command that produced the Rust source.
pub fn inspect_resolved_extern_dependencies_for_compilation(
    manifest_path: &Path,
    package_filter: Option<&str>,
    cargo_flags: &[String],
    compilation: &CargoCompilationContext,
) -> Result<Vec<ResolvedExternDependency>, String> {
    let project_dir = manifest_path.parent().unwrap_or(Path::new("."));
    let mut command = std::process::Command::new("cargo");
    command
        .arg("metadata")
        .arg("--format-version")
        .arg("1")
        .arg("--manifest-path")
        .arg(manifest_path);
    apply_resolution_flags_to_metadata(&mut command, cargo_flags)?;
    let output = command
        .current_dir(project_dir)
        .output()
        .map_err(|error| format!("Failed to run cargo metadata: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "cargo metadata failed:\n{}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    let metadata: CargoMetadataResolved = serde_json::from_slice(&output.stdout)
        .map_err(|error| format!("Failed to parse cargo metadata: {error}"))?;
    let selected = select_resolved_package(&metadata, manifest_path, package_filter)?;
    let resolve = metadata
        .resolve
        .as_ref()
        .ok_or_else(|| "cargo metadata did not return a resolved dependency graph".to_string())?;
    let node = resolve
        .nodes
        .iter()
        .find(|node| node.id == selected.id)
        .ok_or_else(|| {
            format!(
                "cargo metadata did not return the resolved node for {}",
                manifest_path.display()
            )
        })?;
    let packages = metadata
        .packages
        .iter()
        .map(|package| (package.id.as_str(), package.name.as_str()))
        .collect::<HashMap<_, _>>();
    let mut dependencies = node
        .deps
        .iter()
        .filter(|dependency| {
            dependency.dep_kinds.is_empty()
                || dependency.dep_kinds.iter().any(|kind| {
                    compilation
                        .dependency_context()
                        .includes(kind.kind.as_deref())
                })
        })
        .filter_map(|dependency| {
            let package_name = packages.get(dependency.pkg.as_str())?;
            Some(ResolvedExternDependency {
                extern_crate_root: dependency.name.replace('-', "_"),
                package_name: (*package_name).to_string(),
            })
        })
        .collect::<Vec<_>>();
    dependencies.sort_by(|left, right| {
        left.extern_crate_root
            .cmp(&right.extern_crate_root)
            .then_with(|| left.package_name.cmp(&right.package_name))
    });
    dependencies.dedup();
    Ok(dependencies)
}

/// Resolve the exact Cargo target root represented by a direct source path.
/// Nested module files are not independently selected Cargo targets; callers
/// receive conservative provenance for those paths instead of guessing which
/// target/module graph owns them.
pub fn compilation_context_for_source(
    manifest_path: &Path,
    package_filter: Option<&str>,
    cargo_flags: &[String],
    source_path: &Path,
    include_dev_dependencies: bool,
) -> Result<CargoCompilationContext, String> {
    let (_, targets) =
        discover_targets_with_context(manifest_path, package_filter, cargo_flags)?;
    let source_path = canonicalized_path(source_path);
    let mut matches = targets
        .iter()
        .filter(|target| canonicalized_path(&target.src_path) == source_path);
    if let Some(target) = matches.next() {
        if matches.next().is_some() {
            return Ok(CargoCompilationContext::conservative());
        }
        return Ok(CargoCompilationContext::exact(
            target,
            include_dev_dependencies,
        ));
    }

    // Target discovery intentionally omits build scripts and other targets the
    // parity transpiler does not process. Direct source authentication still
    // has to recognize those exact roots so `build.rs` receives build-only
    // dependency provenance instead of the unknown-source union.
    let identity =
        inspect_manifest_identity_with_context(manifest_path, package_filter, cargo_flags)?;
    let mut raw_matches = identity
        .targets
        .iter()
        .filter(|target| canonicalized_path(&target.src_path) == source_path);
    let Some(target) = raw_matches.next() else {
        return Ok(CargoCompilationContext::conservative());
    };
    if raw_matches.next().is_some() {
        return Ok(CargoCompilationContext::conservative());
    }
    let target = CrateTarget {
        name: target.name.clone(),
        kind: TargetKind::from_cargo(&target.kind),
        src_path: target.src_path.clone(),
        module_name: normalize_module_base(&target.name),
    };
    Ok(CargoCompilationContext::exact(
        &target,
        include_dev_dependencies,
    ))
}

/// Cargo's global configuration directory for a command launched from
/// `project_dir`.
///
/// When `CARGO_HOME` is absent, Cargo does not rely on the `HOME` environment
/// variable alone: its platform home lookup can fall back to the account
/// database. `std::env::home_dir` intentionally provides the same fallback,
/// so callers cannot miss a selected patch merely because both variables are
/// unset.
pub(crate) fn effective_cargo_home(project_dir: &Path) -> Option<PathBuf> {
    if let Some(cargo_home) = std::env::var_os("CARGO_HOME") {
        // Cargo treats an explicitly-empty CARGO_HOME exactly like an absent
        // one and falls back to its platform home lookup.  Do the same: using
        // `project_dir.join("")` here would miss the account-home
        // `.cargo/config.toml` that the Cargo subprocess still consumes.
        if !cargo_home.is_empty() {
            let cargo_home = PathBuf::from(cargo_home);
            return Some(if cargo_home.is_absolute() {
                cargo_home
            } else {
                project_dir.join(cargo_home)
            });
        }
    }
    #[allow(deprecated)]
    std::env::home_dir().map(|home| {
        let home = if home.is_absolute() {
            home
        } else {
            project_dir.join(home)
        };
        home.join(".cargo")
    })
}

fn configured_cargo_build_target(project_dir: &Path) -> Result<Option<String>, String> {
    fn config_build_target(
        config_path: &Path,
        active: &mut HashSet<PathBuf>,
    ) -> Result<Option<String>, String> {
        let key = canonicalized_path(config_path);
        if !active.insert(key.clone()) {
            return Err(format!(
                "Cargo configuration include cycle reaches {}",
                config_path.display()
            ));
        }
        let source = std::fs::read_to_string(config_path).map_err(|error| {
            format!(
                "could not read Cargo configuration {} while resolving build.target: {error}",
                config_path.display()
            )
        })?;
        let config = toml::from_str::<toml::Value>(&source).map_err(|error| {
            format!(
                "could not parse Cargo configuration {} while resolving build.target: {error}",
                config_path.display()
            )
        })?;
        let table = config.as_table().ok_or_else(|| {
            format!(
                "Cargo configuration {} is not a TOML table",
                config_path.display()
            )
        })?;

        let mut selected = None;
        if let Some(includes) = table.get("include") {
            let includes = includes.as_array().ok_or_else(|| {
                format!(
                    "Cargo configuration {} has a non-array include value",
                    config_path.display()
                )
            })?;
            for include in includes {
                let (relative, optional) = match include {
                    toml::Value::String(path) => (path.as_str(), false),
                    toml::Value::Table(table) => {
                        let path = table
                            .get("path")
                            .and_then(toml::Value::as_str)
                            .ok_or_else(|| {
                                format!(
                                    "Cargo configuration {} has an included table without a string path",
                                    config_path.display()
                                )
                            })?;
                        let optional = table
                            .get("optional")
                            .map(|value| {
                                value.as_bool().ok_or_else(|| {
                                    format!(
                                        "Cargo configuration {} has a non-boolean optional include flag",
                                        config_path.display()
                                    )
                                })
                            })
                            .transpose()?
                            .unwrap_or(false);
                        (path, optional)
                    }
                    _ => {
                        return Err(format!(
                            "Cargo configuration {} has a malformed include entry",
                            config_path.display()
                        ));
                    }
                };
                let include_path = Path::new(relative);
                let include_path = if include_path.is_absolute() {
                    include_path.to_path_buf()
                } else {
                    config_path
                        .parent()
                        .unwrap_or(Path::new("."))
                        .join(include_path)
                };
                if optional && !include_path.is_file() {
                    continue;
                }
                if let Some(target) = config_build_target(&include_path, active)? {
                    selected = Some(target);
                }
            }
        }

        if let Some(target) = table
            .get("build")
            .and_then(toml::Value::as_table)
            .and_then(|build| build.get("target"))
        {
            let target = target.as_str().ok_or_else(|| {
                format!(
                    "Cargo configuration {} has a non-string build.target; exact target selection is unsupported",
                    config_path.display()
                )
            })?;
            let target = target.trim();
            if target.is_empty() {
                return Err(format!(
                    "Cargo configuration {} has an empty build.target",
                    config_path.display()
                ));
            }
            selected = Some(target.to_string());
        }
        active.remove(&key);
        Ok(selected)
    }

    let mut cargo_directories = Vec::new();
    if let Some(cargo_home) = effective_cargo_home(project_dir) {
        cargo_directories.push(cargo_home);
    }
    let mut ancestors = project_dir.ancestors().collect::<Vec<_>>();
    ancestors.reverse();
    cargo_directories.extend(
        ancestors
            .into_iter()
            .map(|directory| directory.join(".cargo")),
    );

    let mut selected = None;
    for cargo_directory in cargo_directories {
        let modern = cargo_directory.join("config.toml");
        let legacy = cargo_directory.join("config");
        // Cargo intentionally prefers the extensionless legacy file when both
        // spellings exist.
        let config_path = if legacy.is_file() {
            legacy
        } else if modern.is_file() {
            modern
        } else {
            continue;
        };
        if let Some(target) = config_build_target(&config_path, &mut HashSet::new())? {
            selected = Some(target);
        }
    }
    Ok(selected)
}

fn effective_target_triple(project_dir: &Path) -> Result<String, String> {
    if let Some(target) = std::env::var_os("CARGO_BUILD_TARGET") {
        let target = target.to_string_lossy().trim().to_string();
        if target.is_empty() {
            return Err("CARGO_BUILD_TARGET is present but empty".to_string());
        }
        return Ok(target);
    }
    if let Some(target) = configured_cargo_build_target(project_dir)? {
        return Ok(target);
    }

    let rustc = std::env::var_os("RUSTC").unwrap_or_else(|| "rustc".into());
    let output = std::process::Command::new(&rustc)
        .arg("-vV")
        .output()
        .map_err(|error| {
            format!(
                "could not execute {} -vV to determine Cargo's target: {error}",
                Path::new(&rustc).display()
            )
        })?;
    if !output.status.success() {
        return Err(format!(
            "{} -vV failed while determining Cargo's target:\n{}",
            Path::new(&rustc).display(),
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    let stdout = String::from_utf8(output.stdout)
        .map_err(|error| format!("rustc -vV returned non-UTF-8 output: {error}"))?;
    let mut hosts = stdout
        .lines()
        .filter_map(|line| line.strip_prefix("host: "))
        .map(str::trim)
        .filter(|host| !host.is_empty());
    let host = hosts
        .next()
        .ok_or_else(|| "rustc -vV did not report a host target".to_string())?;
    if hosts.next().is_some() {
        return Err("rustc -vV reported more than one host target".to_string());
    }
    Ok(host.to_string())
}

/// A private, process-unique Cargo workspace used to ask Cargo for the
/// dependency graph of exactly one requested package.
///
/// `cargo metadata --manifest-path member/Cargo.toml` still resolves every
/// member of the containing workspace. With resolver v2 that can unify a
/// dependency's features with an unrelated workspace member and make optional
/// edges look selected for the requested package. A one-package wrapper makes
/// the requested package the only dependency root, matching `cargo check
/// --manifest-path member/Cargo.toml` and `cargo tree -p member`.
struct EffectiveGraphProbeDir {
    path: PathBuf,
}

impl EffectiveGraphProbeDir {
    fn create() -> Result<Self, String> {
        static NEXT_PROBE: AtomicU64 = AtomicU64::new(0);

        let base = std::env::temp_dir();
        for _ in 0..1000 {
            let sequence = NEXT_PROBE.fetch_add(1, Ordering::Relaxed);
            let path = base.join(format!(
                "rusty-cpp-effective-graph-{}-{sequence}",
                std::process::id()
            ));
            match fs::create_dir(&path) {
                Ok(()) => return Ok(Self { path }),
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(error) => {
                    return Err(format!(
                        "could not create Cargo effective-graph probe directory {}: {error}",
                        path.display()
                    ));
                }
            }
        }
        Err(format!(
            "could not allocate a unique Cargo effective-graph probe directory under {}",
            base.display()
        ))
    }
}

impl Drop for EffectiveGraphProbeDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

fn resolver_default_for_edition(edition: &str) -> &'static str {
    match edition {
        "2024" => "3",
        "2021" => "2",
        _ => "1",
    }
}

fn workspace_resolver_and_overrides(
    workspace_root: &Path,
) -> Result<(String, Option<toml::Value>, Option<toml::Value>), String> {
    let manifest_path = workspace_root.join("Cargo.toml");
    let source = fs::read_to_string(&manifest_path).map_err(|error| {
        format!(
            "could not read workspace manifest {} while preparing Cargo effective-graph probe: {error}",
            manifest_path.display()
        )
    })?;
    let manifest = toml::from_str::<toml::Value>(&source).map_err(|error| {
        format!(
            "could not parse workspace manifest {} while preparing Cargo effective-graph probe: {error}",
            manifest_path.display()
        )
    })?;
    let workspace = manifest.get("workspace").and_then(toml::Value::as_table);
    let package = manifest.get("package").and_then(toml::Value::as_table);
    let explicit_resolver = workspace
        .and_then(|table| table.get("resolver"))
        .or_else(|| package.and_then(|table| table.get("resolver")));
    let resolver = if let Some(resolver) = explicit_resolver {
        resolver.as_str().ok_or_else(|| {
            format!(
                "workspace resolver in {} is not a string",
                manifest_path.display()
            )
        })?
    } else if let Some(package) = package {
        let edition = match package.get("edition") {
            Some(toml::Value::String(edition)) => edition.as_str(),
            Some(toml::Value::Table(inherited))
                if inherited.get("workspace").and_then(toml::Value::as_bool) == Some(true) =>
            {
                workspace
                    .and_then(|table| table.get("package"))
                    .and_then(toml::Value::as_table)
                    .and_then(|table| table.get("edition"))
                    .and_then(toml::Value::as_str)
                    .ok_or_else(|| {
                        format!(
                            "workspace-inherited package edition in {} has no workspace.package.edition",
                            manifest_path.display()
                        )
                    })?
            }
            Some(_) => {
                return Err(format!(
                    "package edition in {} is neither a string nor workspace-inherited",
                    manifest_path.display()
                ));
            }
            None => "2015",
        };
        resolver_default_for_edition(edition)
    } else {
        // Cargo's historical default for a virtual workspace.
        "1"
    };
    if !matches!(resolver, "1" | "2" | "3") {
        return Err(format!(
            "unsupported Cargo resolver '{resolver}' in {}",
            manifest_path.display()
        ));
    }
    Ok((
        resolver.to_string(),
        manifest.get("patch").cloned(),
        manifest.get("replace").cloned(),
    ))
}

fn absolutize_manifest_path_values(value: &mut toml::Value, base: &Path) {
    match value {
        toml::Value::Table(table) => {
            for (key, value) in table {
                if key == "path"
                    && let toml::Value::String(path) = value
                {
                    let path = Path::new(path);
                    if path.is_relative() {
                        *value =
                            toml::Value::String(base.join(path).to_string_lossy().into_owned());
                    }
                } else {
                    absolutize_manifest_path_values(value, base);
                }
            }
        }
        toml::Value::Array(values) => {
            for value in values {
                absolutize_manifest_path_values(value, base);
            }
        }
        _ => {}
    }
}

#[derive(Debug, Clone)]
struct EffectiveGraphCargoContext {
    dependency_features: Vec<String>,
    default_features: bool,
    non_feature_flags: Vec<String>,
    explicit_target: Option<String>,
}

fn normalize_requested_feature(
    selector: &str,
    package_name: &str,
) -> Result<Option<String>, String> {
    let selector = selector.trim();
    if selector.is_empty() {
        return Ok(None);
    }
    if let Some((package, feature)) = selector.split_once('/') {
        if package != package_name {
            return Err(format!(
                "unsupported Cargo dependency feature selector '{selector}' while selecting cpp_name's effective dependency graph; the isolated package probe cannot exactly project direct or weak dependency feature selectors"
            ));
        }
        if feature.is_empty() {
            return Err(format!("Cargo feature selector '{package}/' has no feature name"));
        }
        return Ok(Some(feature.to_string()));
    }
    Ok(Some(selector.to_string()))
}

fn effective_graph_cargo_context(
    cargo_flags: &[String],
    identity: &ManifestIdentity,
) -> Result<EffectiveGraphCargoContext, String> {
    let mut requested_features = Vec::new();
    let mut all_features = false;
    let mut default_features = true;
    let mut non_feature_flags = Vec::new();
    let mut explicit_target: Option<String> = None;
    let mut index = 0usize;
    while index < cargo_flags.len() {
        let flag = cargo_flags[index].as_str();
        match flag {
            "--features" | "-F" => {
                let value = cargo_flags.get(index + 1).ok_or_else(|| {
                    format!("Cargo feature flag '{flag}' requires a value")
                })?;
                for feature in value.split(|ch: char| ch == ',' || ch.is_whitespace()) {
                    if let Some(feature) =
                        normalize_requested_feature(feature, &identity.package_name)?
                    {
                        requested_features.push(feature);
                    }
                }
                index += 2;
            }
            "--all-features" => {
                all_features = true;
                index += 1;
            }
            "--no-default-features" => {
                default_features = false;
                index += 1;
            }
            "--target" | "--filter-platform" => {
                let value = cargo_flags.get(index + 1).ok_or_else(|| {
                    format!("Cargo resolution flag '{flag}' requires a value")
                })?;
                if let Some(previous) = &explicit_target
                    && previous != value
                {
                    return Err(format!(
                        "conflicting Cargo target selectors '{previous}' and '{value}'"
                    ));
                }
                explicit_target = Some(value.clone());
                non_feature_flags.push("--target".to_string());
                non_feature_flags.push(value.clone());
                index += 2;
            }
            "--config" => {
                let value = cargo_flags.get(index + 1).ok_or_else(|| {
                    "Cargo resolution flag '--config' requires a value".to_string()
                })?;
                non_feature_flags.push(flag.to_string());
                non_feature_flags.push(value.clone());
                index += 2;
            }
            "--locked" | "--offline" | "--frozen" => {
                non_feature_flags.push(flag.to_string());
                index += 1;
            }
            _ if flag.starts_with("--features=") => {
                let value = flag.trim_start_matches("--features=");
                for feature in value.split(|ch: char| ch == ',' || ch.is_whitespace()) {
                    if let Some(feature) =
                        normalize_requested_feature(feature, &identity.package_name)?
                    {
                        requested_features.push(feature);
                    }
                }
                index += 1;
            }
            _ if flag.starts_with("--target=") || flag.starts_with("--filter-platform=") => {
                let value = flag.split_once('=').map(|(_, value)| value).unwrap_or_default();
                if value.is_empty() {
                    return Err(format!("Cargo resolution flag '{flag}' has an empty target"));
                }
                if let Some(previous) = &explicit_target
                    && previous != value
                {
                    return Err(format!(
                        "conflicting Cargo target selectors '{previous}' and '{value}'"
                    ));
                }
                explicit_target = Some(value.to_string());
                non_feature_flags.push(format!("--target={value}"));
                index += 1;
            }
            _ if flag.starts_with("--config=") => {
                non_feature_flags.push(flag.to_string());
                index += 1;
            }
            _ => {
                return Err(format!(
                    "unsupported Cargo resolution flag '{flag}' while selecting cpp_name's effective dependency graph"
                ));
            }
        }
    }
    if all_features {
        requested_features.extend(identity.feature_names.iter().cloned());
    }
    requested_features.sort();
    requested_features.dedup();
    Ok(EffectiveGraphCargoContext {
        dependency_features: requested_features,
        default_features,
        non_feature_flags,
        explicit_target,
    })
}

/// The isolated probe is a new synthetic root package, so Cargo must add that
/// one package to the copied lockfile even when the caller's real workspace is
/// already exactly locked. Validate `--locked`/`--frozen` against the real
/// manifest first, then permit only that temporary lockfile rewrite. Frozen
/// still projects its offline half into the probe; all other graph-shaping
/// flags remain byte-for-byte identical.
fn effective_graph_probe_non_feature_flags(flags: &[String]) -> Vec<String> {
    let mut projected = Vec::new();
    let mut has_offline = false;
    for flag in flags {
        match flag.as_str() {
            "--locked" => {}
            "--frozen" | "--offline" => {
                if !has_offline {
                    projected.push("--offline".to_string());
                    has_offline = true;
                }
            }
            _ => projected.push(flag.clone()),
        }
    }
    projected
}

fn validate_real_manifest_lock_context(
    manifest_path: &Path,
    project_dir: &Path,
    cargo_flags: &[String],
) -> Result<(), String> {
    if !cargo_flags
        .iter()
        .any(|flag| matches!(flag.as_str(), "--locked" | "--frozen"))
    {
        return Ok(());
    }
    let mut command = std::process::Command::new("cargo");
    command
        .arg("metadata")
        .arg("--format-version")
        .arg("1")
        .arg("--manifest-path")
        .arg(manifest_path);
    apply_resolution_flags_to_metadata(&mut command, cargo_flags)?;
    let output = command
        .current_dir(project_dir)
        .output()
        .map_err(|error| format!("Failed to validate Cargo lock context: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "real Cargo manifest failed the requested locked/frozen resolution context before effective-graph probing:\n{}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    Ok(())
}

fn write_effective_graph_probe_manifest(
    requested_manifest: &Path,
    identity: &ManifestIdentity,
    cargo_context: &EffectiveGraphCargoContext,
) -> Result<EffectiveGraphProbeDir, String> {
    let probe = EffectiveGraphProbeDir::create()?;
    let requested_package_dir = requested_manifest.parent().ok_or_else(|| {
        format!(
            "requested Cargo manifest has no parent directory: {}",
            requested_manifest.display()
        )
    })?;

    let mut package = toml::map::Map::new();
    package.insert(
        "name".to_string(),
        toml::Value::String("rusty-cpp-effective-graph-probe".to_string()),
    );
    package.insert(
        "version".to_string(),
        toml::Value::String("0.0.0".to_string()),
    );
    package.insert(
        "edition".to_string(),
        toml::Value::String(identity.edition.clone()),
    );
    if let Some(rust_version) = &identity.rust_version {
        package.insert(
            "rust-version".to_string(),
            toml::Value::String(rust_version.clone()),
        );
    }
    package.insert("publish".to_string(), toml::Value::Boolean(false));

    let mut library = toml::map::Map::new();
    library.insert(
        "path".to_string(),
        toml::Value::String("lib.rs".to_string()),
    );

    let mut requested_dependency = toml::map::Map::new();
    requested_dependency.insert(
        "package".to_string(),
        toml::Value::String(identity.package_name.clone()),
    );
    requested_dependency.insert(
        "path".to_string(),
        toml::Value::String(requested_package_dir.to_string_lossy().into_owned()),
    );
    if !cargo_context.default_features {
        requested_dependency.insert("default-features".to_string(), toml::Value::Boolean(false));
    }
    if !cargo_context.dependency_features.is_empty() {
        requested_dependency.insert(
            "features".to_string(),
            toml::Value::Array(
                cargo_context
                    .dependency_features
                    .iter()
                    .cloned()
                    .map(toml::Value::String)
                    .collect(),
            ),
        );
    }
    let mut dependencies = toml::map::Map::new();
    dependencies.insert(
        "__rusty_cpp_requested_root".to_string(),
        toml::Value::Table(requested_dependency),
    );

    let mut manifest = toml::map::Map::new();
    manifest.insert("package".to_string(), toml::Value::Table(package));
    manifest.insert("lib".to_string(), toml::Value::Table(library));
    manifest.insert("dependencies".to_string(), toml::Value::Table(dependencies));
    let (resolver, mut patch, mut replace) =
        workspace_resolver_and_overrides(&identity.workspace_root)?;
    let workspace_base = &identity.workspace_root;
    if let Some(patch) = patch.as_mut() {
        absolutize_manifest_path_values(patch, workspace_base);
    }
    if let Some(replace) = replace.as_mut() {
        absolutize_manifest_path_values(replace, workspace_base);
    }

    // Explicit isolation prevents an unrelated Cargo.toml above the system
    // temporary directory from adopting the probe into another workspace.
    // Preserve the requested workspace's resolver so target/build feature
    // unification does not change merely because the probe is the graph root.
    let mut workspace = toml::map::Map::new();
    workspace.insert("resolver".to_string(), toml::Value::String(resolver));
    manifest.insert("workspace".to_string(), toml::Value::Table(workspace));
    if let Some(patch) = patch {
        manifest.insert("patch".to_string(), patch);
    }
    if let Some(replace) = replace {
        manifest.insert("replace".to_string(), replace);
    }

    let source = toml::to_string(&toml::Value::Table(manifest))
        .map_err(|error| format!("could not serialize Cargo effective-graph probe: {error}"))?;
    fs::write(probe.path.join("Cargo.toml"), source).map_err(|error| {
        format!(
            "could not write Cargo effective-graph probe manifest under {}: {error}",
            probe.path.display()
        )
    })?;
    fs::write(probe.path.join("lib.rs"), "").map_err(|error| {
        format!(
            "could not write Cargo effective-graph probe library under {}: {error}",
            probe.path.display()
        )
    })?;
    let source_lock = identity.workspace_root.join("Cargo.lock");
    if source_lock.is_file() {
        fs::copy(&source_lock, probe.path.join("Cargo.lock")).map_err(|error| {
            format!(
                "could not copy workspace lockfile {} into Cargo effective-graph probe: {error}",
                source_lock.display()
            )
        })?;
    }
    Ok(probe)
}

/// Ask Cargo's feature-context-aware tree resolver which package-to-package
/// edges belong to the target normal graph.
///
/// `cargo metadata` deliberately reports one feature union per package ID. In
/// resolver v2/v3 that union can contain features enabled only by a build,
/// development, or procedural-macro host unit. Those features must not
/// activate optional dependencies in the package's target-normal unit.
/// `cargo tree -e normal,no-proc-macro` retains the target feature context and
/// prunes every host-only procedural-macro subtree. We use its tree only as an
/// edge-selection witness, while retaining package identities and dependency
/// aliases from metadata.
struct NormalTargetPackageSelection {
    edges: HashSet<(String, String)>,
    selected_features_by_id: HashMap<String, Vec<String>>,
    /// Edges from a graph crate to a package of a git repository that
    /// `--crate-graph` refuses (`CrateGraphGitRepositories::refuses`); empty
    /// unless that policy is on.
    refused_git_edges: HashSet<(String, String)>,
}

fn resolve_normal_target_package_edges(
    probe_manifest: &Path,
    project_dir: &Path,
    target_triple: &str,
    cargo_flags: &[String],
    metadata: &CargoMetadataResolved,
    expected_root_package_id: &str,
    git_repositories: &CrateGraphGitRepositories,
) -> Result<NormalTargetPackageSelection, String> {
    let mut command = std::process::Command::new("cargo");
    command
        .arg("tree")
        .arg("--manifest-path")
        .arg(probe_manifest)
        .arg("--target")
        .arg(target_triple)
        .arg("--edges")
        .arg("normal,no-proc-macro")
        .arg("--prefix")
        .arg("depth")
        .arg("--no-dedupe")
        .arg("--charset")
        .arg("ascii")
        .arg("--format")
        .arg("{p}|{f}")
        .arg("--color")
        .arg("never")
        .arg("--quiet");
    let mut index = 0usize;
    while index < cargo_flags.len() {
        let flag = cargo_flags[index].as_str();
        match flag {
            // `--target` is already supplied from the exact selected context.
            "--target" | "--filter-platform" => {
                if cargo_flags.get(index + 1).is_none() {
                    return Err(format!("Cargo resolution flag '{flag}' requires a value"));
                }
                index += 2;
            }
            "--config" => {
                let value = cargo_flags.get(index + 1).ok_or_else(|| {
                    "Cargo resolution flag '--config' requires a value".to_string()
                })?;
                command.arg(flag).arg(value);
                index += 2;
            }
            "--locked" | "--offline" | "--frozen" => {
                command.arg(flag);
                index += 1;
            }
            _ if flag.starts_with("--target=") || flag.starts_with("--filter-platform=") => {
                index += 1;
            }
            _ if flag.starts_with("--config=") => {
                command.arg(flag);
                index += 1;
            }
            _ => {
                return Err(format!(
                    "feature or unsupported Cargo flag '{flag}' reached the isolated effective-graph tree"
                ));
            }
        }
    }
    let output = command
        .current_dir(project_dir)
        .output()
        .map_err(|error| format!("Failed to run cargo tree: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "cargo tree --edges normal,no-proc-macro --target {target_triple} failed:\n{}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    let stdout = String::from_utf8(output.stdout)
        .map_err(|error| format!("cargo tree returned non-UTF-8 output: {error}"))?;
    select_normal_target_packages(&stdout, metadata, expected_root_package_id, git_repositories)
}

/// Read the `cargo tree --prefix depth --format '{p}|{f}'` output of the
/// probe against the probe's own metadata: the edges between graph crates,
/// each graph crate's target-normal feature set, and the edges from a graph
/// crate to a git package `--crate-graph` refuses.
fn select_normal_target_packages(
    stdout: &str,
    metadata: &CargoMetadataResolved,
    expected_root_package_id: &str,
    git_repositories: &CrateGraphGitRepositories,
) -> Result<NormalTargetPackageSelection, String> {
    // Cargo's documented `{p}` rendering for local path packages includes
    // the exact package name/version and package directory. Construct every
    // accepted rendering from the same metadata invocation, require it to be
    // unique, and fail closed if a local-looking line differs. A package from
    // a `--crate-graph-git` repository is a graph crate too, rendered as its
    // git source. Other registry/git nodes may remain opaque because local
    // traversal stops at them.
    let mut local_by_render = HashMap::<String, String>::new();
    let mut local_prefixes = Vec::<String>::new();
    for package in metadata
        .packages
        .iter()
        .filter(|package| git_repositories.admits(package.source.as_deref()))
    {
        let location = match package.source.as_deref() {
            None => package
                .manifest_path
                .parent()
                .ok_or_else(|| {
                    format!(
                        "Cargo metadata reported a local manifest without a parent: {}",
                        package.manifest_path.display()
                    )
                })?
                .display()
                .to_string(),
            Some(source) => cargo_tree_git_source(source).ok_or_else(|| {
                format!("Cargo metadata reported a git source `cargo tree` cannot render: {source}")
            })?,
        };
        let prefix = format!("{} v{} ", package.name, package.version);
        let renderings = [
            format!("{prefix}({location})"),
            format!("{prefix}(proc-macro) ({location})"),
        ];
        for rendered in renderings {
            if let Some(previous) = local_by_render.insert(rendered.clone(), package.id.clone())
                && previous != package.id
            {
                return Err(format!(
                    "Cargo normal-tree rendering is ambiguous for package ids '{previous}' and '{}': {rendered}",
                    package.id
                ));
            }
        }
        local_prefixes.push(prefix);
    }
    local_prefixes.sort();
    local_prefixes.dedup();

    // Under `--crate-graph`, a package from a git repository that is neither
    // named nor exempt is recognized too, where a graph crate depends on it,
    // so the walk can refuse the edge. It stays opaque: nothing below it is
    // read. Its line must match exactly, as a graph crate's must, or a change
    // in Cargo's rendering would let the edge through unseen.
    let mut refused_by_render = HashMap::<String, String>::new();
    let mut refused_prefixes = Vec::<String>::new();
    for package in metadata
        .packages
        .iter()
        .filter(|package| git_repositories.refuses(package.source.as_deref()))
    {
        let source = package.source.as_deref().unwrap_or_default();
        let location = cargo_tree_git_source(source).ok_or_else(|| {
            format!("Cargo metadata reported a git source `cargo tree` cannot render: {source}")
        })?;
        let prefix = format!("{} v{} ", package.name, package.version);
        for rendered in [
            format!("{prefix}({location})"),
            format!("{prefix}(proc-macro) ({location})"),
        ] {
            if let Some(previous) = refused_by_render.insert(rendered.clone(), package.id.clone())
                && previous != package.id
            {
                return Err(format!(
                    "Cargo normal-tree rendering is ambiguous for package ids '{previous}' and '{}': {rendered}",
                    package.id
                ));
            }
        }
        refused_prefixes.push(prefix);
    }

    let mut stack = Vec::<Option<String>>::new();
    let mut edges = HashSet::new();
    let mut refused_git_edges = HashSet::new();
    let mut selected_features_by_id = HashMap::<String, Vec<String>>::new();
    let mut roots = Vec::new();
    for (line_index, line) in stdout.lines().enumerate() {
        if line.is_empty() {
            continue;
        }
        let depth_end = line.bytes().take_while(u8::is_ascii_digit).count();
        if depth_end == 0 || depth_end == line.len() {
            return Err(format!(
                "could not parse Cargo normal-tree line {}: {line:?}",
                line_index + 1
            ));
        }
        let depth = line[..depth_end].parse::<usize>().map_err(|error| {
            format!(
                "could not parse Cargo normal-tree depth on line {}: {error}",
                line_index + 1
            )
        })?;
        if depth > stack.len() {
            return Err(format!(
                "Cargo normal tree skipped from depth {} to {depth} on line {}",
                stack.len(),
                line_index + 1
            ));
        }
        let rendered_and_features = &line[depth_end..];
        let mut local_match: Option<(&str, &str)> = None;
        for (rendered, package_id) in &local_by_render {
            if let Some(features) = rendered_and_features
                .strip_prefix(rendered)
                .and_then(|suffix| suffix.strip_prefix('|'))
            {
                if local_match.is_some() {
                    return Err(format!(
                        "Cargo normal-tree line {} matched more than one local package identity: {rendered_and_features}",
                        line_index + 1
                    ));
                }
                local_match = Some((package_id.as_str(), features));
            }
        }
        let package_id = local_match.map(|(package_id, _)| package_id.to_string());
        if package_id.is_none()
            && local_prefixes
                .iter()
                .any(|prefix| rendered_and_features.starts_with(prefix))
        {
            return Err(format!(
                "Cargo normal tree reported an inexact local package identity on line {}: {rendered_and_features}",
                line_index + 1
            ));
        }

        if let Some((package_id, feature_text)) = local_match {
            let mut features = feature_text
                .split(',')
                .map(str::trim)
                .filter(|feature| !feature.is_empty())
                .map(ToString::to_string)
                .collect::<Vec<_>>();
            features.sort();
            features.dedup();
            if let Some(previous) = selected_features_by_id.get(package_id)
                && previous != &features
            {
                return Err(format!(
                    "Cargo normal tree reported divergent target feature contexts for package '{package_id}': {previous:?} versus {features:?}"
                ));
            }
            selected_features_by_id.insert(package_id.to_string(), features);
        }

        stack.truncate(depth);
        if depth == 0 {
            roots.push(package_id.clone());
        } else if let Some(Some(parent)) = stack.last() {
            match package_id.as_ref() {
                Some(child) => {
                    edges.insert((parent.clone(), child.clone()));
                }
                None => {
                    let refused = refused_by_render.iter().find_map(|(rendered, child)| {
                        rendered_and_features
                            .strip_prefix(rendered.as_str())
                            .is_some_and(|suffix| suffix.starts_with('|'))
                            .then_some(child)
                    });
                    if let Some(child) = refused {
                        refused_git_edges.insert((parent.clone(), child.clone()));
                    } else if refused_prefixes
                        .iter()
                        .any(|prefix| rendered_and_features.starts_with(prefix))
                    {
                        return Err(format!(
                            "Cargo normal tree reported an inexact git package identity on line {}: {rendered_and_features}",
                            line_index + 1
                        ));
                    }
                }
            }
        }
        stack.push(package_id);
    }
    if roots.len() != 1 || roots[0].as_deref() != Some(expected_root_package_id) {
        return Err(format!(
            "cargo tree normal-graph root did not match probe package '{expected_root_package_id}': {roots:?}"
        ));
    }
    Ok(NormalTargetPackageSelection {
        edges,
        selected_features_by_id,
        refused_git_edges,
    })
}

fn reject_ambiguous_selected_local_aliases(
    package: &ResolvedPackage,
    node: &ResolveNode,
    packages_by_id: &HashMap<&str, &ResolvedPackage>,
    normal_package_edges: &HashSet<(String, String)>,
) -> Result<(), String> {
    // The tree's documented `{p}` rendering identifies packages, not the
    // source-visible alias of an edge. If metadata reports more than one
    // normal alias from this parent to the same selected child package, the
    // package-pair witness cannot prove which alias survived target selection.
    // Never guess: recursive generation uses this key in its output layout and
    // must consume the same exact graph as preflight.
    let mut selected_aliases_by_package = HashMap::<&str, HashSet<&str>>::new();
    for dependency in &node.deps {
        let has_normal_kind = dependency.dep_kinds.is_empty()
            || dependency.dep_kinds.iter().any(|kind| kind.kind.is_none());
        if has_normal_kind
            && normal_package_edges.contains(&(package.id.clone(), dependency.pkg.clone()))
        {
            selected_aliases_by_package
                .entry(dependency.pkg.as_str())
                .or_default()
                .insert(dependency.name.as_str());
        }
    }
    for (dependency_package_id, aliases) in &selected_aliases_by_package {
        if aliases.len() > 1 {
            let dependency_package = packages_by_id.get(dependency_package_id).ok_or_else(|| {
                format!(
                    "effective Cargo resolve graph references unknown dependency id '{dependency_package_id}'"
                )
            })?;
            let mut aliases = aliases.iter().copied().collect::<Vec<_>>();
            aliases.sort_unstable();
            return Err(format!(
                "Cargo target-normal graph cannot distinguish dependency aliases {} from '{}' to package '{}'; refusing to guess",
                aliases.join(", "),
                package.name,
                dependency_package.name
            ));
        }
    }
    Ok(())
}

/// Resolve the exact Cargo-selected target-normal local-dependency graph.
///
/// Cargo itself evaluates target cfg expressions and literal target triples,
/// expands workspace-inherited declarations, and resolves optional/default
/// features. The metadata invocation intentionally remains an unfiltered
/// identity superset. A target-filtered metadata query would incorrectly
/// evaluate every package against the requested target and still merge feature
/// contexts. The normal, non-proc-macro Cargo tree below selects the exact
/// target-context edges from that superset. Callers that own a source-level C++
/// contract must treat any error as a pre-output failure.
///
/// The graph's crates are the local path packages, plus every package from a
/// git repository in `git_repositories` (`--crate-graph-git`); a named
/// repository that contributes none is an error.
pub fn resolve_effective_local_normal_dependency_graph(
    manifest_path: &Path,
) -> Result<EffectiveLocalNormalDependencyGraph, String> {
    resolve_effective_local_normal_dependency_graph_with_context(
        manifest_path,
        None,
        &[],
        &CrateGraphGitRepositories::default(),
    )
}

pub fn resolve_effective_local_normal_dependency_graph_with_context(
    manifest_path: &Path,
    package_filter: Option<&str>,
    cargo_flags: &[String],
    git_repositories: &CrateGraphGitRepositories,
) -> Result<EffectiveLocalNormalDependencyGraph, String> {
    // Normalize before deriving the Cargo working directory. A one-component
    // relative spelling such as `Cargo.toml` has an empty parent, which cannot
    // be passed to `Command::current_dir` even though the manifest is valid.
    let requested_manifest = canonicalized_path(manifest_path);
    let project_dir = requested_manifest.parent().unwrap_or(Path::new("."));
    let requested_identity = inspect_manifest_identity_with_context(
        &requested_manifest,
        package_filter,
        cargo_flags,
    )?;
    validate_real_manifest_lock_context(&requested_manifest, project_dir, cargo_flags)?;
    let cargo_context = effective_graph_cargo_context(cargo_flags, &requested_identity)?;
    let probe_non_feature_flags =
        effective_graph_probe_non_feature_flags(&cargo_context.non_feature_flags);
    let target_triple = cargo_context
        .explicit_target
        .clone()
        .map(Ok)
        .unwrap_or_else(|| effective_target_triple(project_dir))?;
    let probe = write_effective_graph_probe_manifest(
        &requested_manifest,
        &requested_identity,
        &cargo_context,
    )?;
    let mut command = std::process::Command::new("cargo");
    command
        .arg("metadata")
        .arg("--format-version")
        .arg("1")
        .arg("--manifest-path")
        .arg(probe.path.join("Cargo.toml"));
    apply_resolution_flags_to_metadata(&mut command, &probe_non_feature_flags)?;
    let output = command
        .current_dir(project_dir)
        .output()
        .map_err(|error| format!("Failed to run cargo metadata: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "cargo metadata for effective normal graph failed:\n{}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    let metadata: CargoMetadataResolved = serde_json::from_slice(&output.stdout)
        .map_err(|error| format!("Failed to parse effective-graph cargo metadata: {error}"))?;
    let probe_manifest = canonicalized_path(&probe.path.join("Cargo.toml"));
    let probe_package = metadata
        .packages
        .iter()
        .find(|package| canonicalized_path(&package.manifest_path) == probe_manifest)
        .ok_or_else(|| {
            format!(
                "effective Cargo metadata omitted probe package {}",
                probe_manifest.display()
            )
        })?;
    let normal_selection = resolve_normal_target_package_edges(
        &probe.path.join("Cargo.toml"),
        project_dir,
        &target_triple,
        &probe_non_feature_flags,
        &metadata,
        &probe_package.id,
        git_repositories,
    )?;
    let selected = select_resolved_package(&metadata, &requested_manifest, package_filter)?;
    let root_manifest = canonicalized_path(&selected.manifest_path);
    let resolve = metadata.resolve.as_ref().ok_or_else(|| {
        "effective-graph cargo metadata did not report a resolved dependency graph".to_string()
    })?;
    if !resolve.nodes.iter().any(|node| node.id == selected.id) {
        return Err(format!(
            "effective-graph cargo metadata omitted the root resolve node for {}",
            manifest_path.display()
        ));
    }
    let walk = walk_effective_local_graph(
        &metadata.packages,
        resolve,
        &normal_selection,
        selected,
        git_repositories,
    )?;

    let root_features = normal_selection
        .selected_features_by_id
        .get(&selected.id)
        .cloned();
    Ok(EffectiveLocalNormalDependencyGraph {
        target_triple,
        root_manifest,
        direct_dependencies: walk.direct_dependencies,
        root_features,
        extern_dependencies: walk.extern_dependencies,
    })
}

/// The graph crates reachable from the selected root and their edges.
struct EffectiveGraphWalk {
    direct_dependencies: HashMap<PathBuf, Vec<EffectiveLocalNormalDependency>>,
    extern_dependencies: HashMap<PathBuf, Vec<ResolvedExternDependency>>,
}

/// Walk Cargo's resolve graph from the selected root along the target-normal
/// edges the tree selected, descending into every graph crate: a local path
/// package, or a package from a `--crate-graph-git` repository. Every named
/// repository must contribute at least one graph crate, and under
/// `--crate-graph` no graph crate may depend on a package from a git
/// repository that is neither named nor exempt.
fn walk_effective_local_graph(
    packages: &[ResolvedPackage],
    resolve: &ResolveGraph,
    normal_selection: &NormalTargetPackageSelection,
    selected: &ResolvedPackage,
    git_repositories: &CrateGraphGitRepositories,
) -> Result<EffectiveGraphWalk, String> {
    let packages_by_id = packages
        .iter()
        .map(|package| (package.id.as_str(), package))
        .collect::<HashMap<_, _>>();
    let nodes_by_id = resolve
        .nodes
        .iter()
        .map(|node| (node.id.as_str(), node))
        .collect::<HashMap<_, _>>();

    let mut direct_dependencies = HashMap::new();
    let mut extern_dependencies = HashMap::<PathBuf, Vec<ResolvedExternDependency>>::new();
    // `--crate-graph-git`: the named repositories a graph crate came from, and
    // every git repository a graph crate depends on (for the error below).
    let mut joined_repositories = BTreeSet::<&str>::new();
    let mut reached_repositories = BTreeSet::<&str>::new();
    // (graph crate, package, repository) for each edge `--crate-graph` refuses.
    let mut refused_dependencies = BTreeSet::<(&str, String, &str)>::new();
    let mut pending = vec![selected.id.as_str()];
    let mut visited = HashSet::new();
    while let Some(package_id) = pending.pop() {
        if !visited.insert(package_id.to_string()) {
            continue;
        }
        let package = packages_by_id.get(package_id).ok_or_else(|| {
            format!("effective Cargo resolve graph references unknown package id '{package_id}'")
        })?;
        if !git_repositories.admits(package.source.as_deref()) && package_id != selected.id {
            continue;
        }
        let node = nodes_by_id.get(package_id).ok_or_else(|| {
            format!(
                "effective Cargo metadata omitted resolve node for local package '{}'",
                package.name
            )
        })?;

        reject_ambiguous_selected_local_aliases(
            package,
            node,
            &packages_by_id,
            &normal_selection.edges,
        )?;

        let mut edges = Vec::new();
        for dependency in &node.deps {
            let has_normal_kind = dependency.dep_kinds.is_empty()
                || dependency.dep_kinds.iter().any(|kind| kind.kind.is_none());
            if !has_normal_kind {
                continue;
            }
            reached_repositories.extend(
                packages_by_id
                    .get(dependency.pkg.as_str())
                    .and_then(|dependency_package| dependency_package.source.as_deref())
                    .and_then(git_source_repository),
            );
            if normal_selection
                .refused_git_edges
                .contains(&(package.id.clone(), dependency.pkg.clone()))
                && let Some(dependency_package) = packages_by_id.get(dependency.pkg.as_str())
                && let Some(repository) = dependency_package
                    .source
                    .as_deref()
                    .and_then(git_source_repository)
            {
                refused_dependencies.insert((
                    package.name.as_str(),
                    format!("{} v{}", dependency_package.name, dependency_package.version),
                    repository,
                ));
            }
            if !normal_selection
                .edges
                .contains(&(package.id.clone(), dependency.pkg.clone()))
            {
                continue;
            }
            let dependency_package =
                packages_by_id.get(dependency.pkg.as_str()).ok_or_else(|| {
                    format!(
                        "effective Cargo resolve graph references unknown dependency id '{}'",
                        dependency.pkg
                    )
                })?;
            // Every selected normal edge occupies an extern-prelude name,
            // registry dependencies included.
            extern_dependencies
                .entry(canonicalized_path(&package.manifest_path))
                .or_insert_with(Vec::new)
                .push(ResolvedExternDependency {
                    extern_crate_root: dependency.name.replace('-', "_"),
                    package_name: dependency_package.name.clone(),
                });
            if let Some(source) = dependency_package.source.as_deref() {
                match git_repositories.repository_of(source) {
                    Some(repository) => {
                        joined_repositories.insert(repository);
                    }
                    None => continue,
                }
            }
            if dependency.name.trim().is_empty() {
                return Err(format!(
                    "effective Cargo metadata reported an empty dependency key from '{}' to '{}'",
                    package.name, dependency_package.name
                ));
            }
            let dependency_manifest = canonicalized_path(&dependency_package.manifest_path);
            let resolved_features = normal_selection
                .selected_features_by_id
                .get(&dependency.pkg)
                .cloned()
                .ok_or_else(|| {
                    format!(
                        "Cargo normal tree omitted the target feature set for local dependency '{}'",
                        dependency_package.name
                    )
                })?;
            edges.push(EffectiveLocalNormalDependency {
                dependency_key: dependency.name.clone(),
                package_name: dependency_package.name.clone(),
                manifest_path: dependency_manifest,
                resolved_features,
            });
            pending.push(dependency_package.id.as_str());
        }
        edges.sort_by(|left, right| {
            left.dependency_key
                .cmp(&right.dependency_key)
                .then_with(|| left.package_name.cmp(&right.package_name))
                .then_with(|| left.manifest_path.cmp(&right.manifest_path))
        });
        edges.dedup();
        direct_dependencies.insert(canonicalized_path(&package.manifest_path), edges);
    }

    for (canonical, named) in &git_repositories.named {
        if !joined_repositories.contains(canonical.as_str()) {
            let reached = if reached_repositories.is_empty() {
                "none".to_string()
            } else {
                reached_repositories
                    .iter()
                    .copied()
                    .collect::<Vec<_>>()
                    .join(", ")
            };
            return Err(format!(
                "--crate-graph-git {named}: no package of Cargo's target-normal graph comes from this repository; the git repositories its graph crates depend on: {reached}"
            ));
        }
    }
    if !refused_dependencies.is_empty() {
        let edges = refused_dependencies
            .iter()
            .map(|(dependent, package, repository)| format!("\n  {dependent} -> {package} ({repository})"))
            .collect::<String>();
        let mut repositories = refused_dependencies
            .iter()
            .map(|(_, _, repository)| *repository)
            .collect::<Vec<_>>();
        repositories.sort_by_key(|repository| canonical_git_repository_url(repository));
        repositories.dedup_by_key(|repository| canonical_git_repository_url(repository));
        // Verus's repository is never transpiled; --verus-exec exempts it.
        let verus = canonical_git_repository_url(crate::verus_exec::VERUS_GIT_URL);
        let (verus_repositories, repositories): (Vec<_>, Vec<_>) = repositories
            .into_iter()
            .partition(|repository| canonical_git_repository_url(repository) == verus);
        let mut remedy = String::new();
        if !repositories.is_empty() {
            remedy.push_str("\nName each repository to transpile its packages as graph crates:");
            for repository in &repositories {
                remedy.push_str(&format!(" --crate-graph-git {repository}"));
            }
        }
        if !verus_repositories.is_empty() {
            remedy.push_str(
                "\nVerus's packages stay external under --verus-exec, which erases or lowers every use of vstd",
            );
        }
        return Err(format!(
            "graph crates depend on git packages whose repository no --crate-graph-git names; --crate-graph would leave them out and never emit them:{edges}{remedy}"
        ));
    }
    if !git_repositories.is_empty() {
        // Each graph crate is emitted into `<output-dir>/<package>/`, so two
        // must not share a package name. One repository depended on at two
        // references would give two; checked only when a repository is
        // named, which leaves the default graph exactly as it was.
        let mut members = visited
            .iter()
            .filter(|package_id| **package_id != selected.id)
            .filter_map(|package_id| packages_by_id.get(package_id.as_str()))
            .map(|package| (package.name.as_str(), package.id.as_str()))
            .collect::<Vec<_>>();
        members.sort_unstable();
        for pair in members.windows(2) {
            if pair[0].0 == pair[1].0 {
                return Err(format!(
                    "--crate-graph-git: graph crates '{}' and '{}' share the package name '{}' and so its output directory",
                    pair[0].1, pair[1].1, pair[0].0
                ));
            }
        }
    }
    Ok(EffectiveGraphWalk {
        direct_dependencies,
        extern_dependencies,
    })
}

/// Discover resolved dependency packages for the selected package.
///
/// Returns dependency packages in deterministic dependency order
/// (dependencies first), filtered to normal dependencies (optionally dev).
/// Target-qualified edges are accepted only when this query carries an exact
/// `--target`/`--filter-platform` context and Cargo has already pruned the
/// resolve graph; otherwise they remain excluded fail-closed.
///
/// When `include_registry_packages` is `false`, this preserves legacy behavior
/// and returns only local path dependencies (`source = null`).
pub fn discover_library_dependencies(
    manifest_path: &Path,
    package_filter: Option<&str>,
    include_registry_packages: bool,
    include_dev_dependencies: bool,
    cargo_flags: &[String],
) -> Result<Vec<LocalDependencyPackage>, String> {
    let project_dir = manifest_path.parent().unwrap_or(Path::new("."));
    let mut cmd = std::process::Command::new("cargo");
    cmd.arg("metadata")
        .arg("--format-version")
        .arg("1")
        .arg("--manifest-path")
        .arg(manifest_path);
    apply_resolution_flags_to_metadata(&mut cmd, cargo_flags)?;
    let output = cmd
        .current_dir(project_dir)
        .output()
        .map_err(|e| format!("Failed to run cargo metadata: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("cargo metadata failed:\n{}", stderr));
    }

    let metadata: CargoMetadataResolved = serde_json::from_slice(&output.stdout)
        .map_err(|e| format!("Failed to parse cargo metadata: {}", e))?;
    let selected = select_resolved_package(&metadata, manifest_path, package_filter)?;
    let root_id = selected.id.clone();

    let mut packages_by_id: HashMap<&str, &ResolvedPackage> = HashMap::new();
    for pkg in &metadata.packages {
        packages_by_id.insert(pkg.id.as_str(), pkg);
    }

    let mut edges: HashMap<&str, Vec<&str>> = HashMap::new();
    let mut resolved_features_by_id: HashMap<&str, Vec<String>> = HashMap::new();
    let mut dependency_roots_by_id: HashMap<&str, HashSet<String>> = HashMap::new();
    let platform_filtered = cargo_flags.iter().any(|flag| {
        flag == "--target"
            || flag == "--filter-platform"
            || flag.starts_with("--target=")
            || flag.starts_with("--filter-platform=")
    });
    if let Some(resolve) = &metadata.resolve {
        for node in &resolve.nodes {
            let mut deps = Vec::new();
            for dep in &node.deps {
                // Keep only unconditional normal dependencies; skip dev/build and
                // target-qualified edges we cannot soundly evaluate here.
                let include = dep.dep_kinds.is_empty()
                    || dep.dep_kinds.iter().any(|kind| {
                        // With `--filter-platform`, Cargo has already pruned
                        // non-matching target-qualified edges from the resolve
                        // graph.  Without it, retaining any such edge would
                        // guess a target context, so continue to fail closed.
                        if kind.target.is_some() && !platform_filtered {
                            return false;
                        }
                        kind.kind.is_none()
                            || (include_dev_dependencies && kind.kind.as_deref() == Some("dev"))
                    });
                if include {
                    deps.push(dep.pkg.as_str());
                    if !dep.name.trim().is_empty() {
                        dependency_roots_by_id
                            .entry(dep.pkg.as_str())
                            .or_default()
                            .insert(dep.name.replace('-', "_"));
                    }
                }
            }
            deps.sort_by(|a, b| {
                let a_name = packages_by_id.get(a).map(|p| p.name.as_str()).unwrap_or("");
                let b_name = packages_by_id.get(b).map(|p| p.name.as_str()).unwrap_or("");
                a_name.cmp(b_name).then_with(|| a.cmp(b))
            });
            deps.dedup();
            edges.insert(node.id.as_str(), deps);

            let mut features = node.features.clone();
            features.sort();
            features.dedup();
            resolved_features_by_id.insert(node.id.as_str(), features);
        }
    }

    fn visit<'a>(
        node_id: &'a str,
        root_id: &str,
        packages_by_id: &HashMap<&'a str, &'a ResolvedPackage>,
        edges: &HashMap<&'a str, Vec<&'a str>>,
        resolved_features_by_id: &HashMap<&'a str, Vec<String>>,
        dependency_roots_by_id: &HashMap<&'a str, HashSet<String>>,
        include_registry_packages: bool,
        visiting: &mut HashSet<String>,
        visited: &mut HashSet<String>,
        out: &mut Vec<LocalDependencyPackage>,
    ) {
        if visited.contains(node_id) || visiting.contains(node_id) {
            return;
        }
        visiting.insert(node_id.to_string());

        if let Some(deps) = edges.get(node_id) {
            for dep in deps {
                visit(
                    dep,
                    root_id,
                    packages_by_id,
                    edges,
                    resolved_features_by_id,
                    dependency_roots_by_id,
                    include_registry_packages,
                    visiting,
                    visited,
                    out,
                );
            }
        }

        visiting.remove(node_id);
        visited.insert(node_id.to_string());

        if node_id == root_id {
            return;
        }
        let Some(pkg) = packages_by_id.get(node_id) else {
            return;
        };
        if !include_registry_packages && pkg.source.is_some() {
            return;
        }
        out.push(LocalDependencyPackage {
            name: pkg.name.clone(),
            manifest_path: canonicalized_path(&pkg.manifest_path),
            resolved_features: resolved_features_by_id
                .get(node_id)
                .cloned()
                .unwrap_or_default(),
            extern_crate_roots: dependency_roots_by_id
                .get(node_id)
                .map(|roots| {
                    let mut out: Vec<String> = roots.iter().cloned().collect();
                    out.sort();
                    out.dedup();
                    out
                })
                .unwrap_or_default(),
        });
    }

    let mut visiting = HashSet::new();
    let mut visited = HashSet::new();
    let mut deps = Vec::new();
    visit(
        root_id.as_str(),
        root_id.as_str(),
        &packages_by_id,
        &edges,
        &resolved_features_by_id,
        &dependency_roots_by_id,
        include_registry_packages,
        &mut visiting,
        &mut visited,
        &mut deps,
    );

    // Keep deterministic uniqueness in case multiple IDs map to same manifest path.
    // Merge features and crate roots across IDs that resolve to the same manifest.
    let mut merged: Vec<LocalDependencyPackage> = Vec::new();
    let mut by_manifest: HashMap<PathBuf, usize> = HashMap::new();
    for mut dep in deps {
        dep.resolved_features.sort();
        dep.resolved_features.dedup();
        dep.extern_crate_roots.sort();
        dep.extern_crate_roots.dedup();
        if let Some(&idx) = by_manifest.get(&dep.manifest_path) {
            let existing = &mut merged[idx];
            existing.resolved_features.extend(dep.resolved_features);
            existing.resolved_features.sort();
            existing.resolved_features.dedup();
            existing.extern_crate_roots.extend(dep.extern_crate_roots);
            existing.extern_crate_roots.sort();
            existing.extern_crate_roots.dedup();
        } else {
            by_manifest.insert(dep.manifest_path.clone(), merged.len());
            merged.push(dep);
        }
    }
    Ok(merged)
}

/// Backward-compatible helper: local path dependencies only.
pub fn discover_local_path_dependencies(
    manifest_path: &Path,
    package_filter: Option<&str>,
) -> Result<Vec<LocalDependencyPackage>, String> {
    discover_library_dependencies(manifest_path, package_filter, false, false, &[])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_target_kind_from_cargo() {
        assert_eq!(
            TargetKind::from_cargo(&["lib".to_string()]),
            TargetKind::Lib
        );
        assert_eq!(
            TargetKind::from_cargo(&["bin".to_string()]),
            TargetKind::Bin
        );
        assert_eq!(
            TargetKind::from_cargo(&["test".to_string()]),
            TargetKind::Test
        );
        assert_eq!(
            TargetKind::from_cargo(&["example".to_string()]),
            TargetKind::Example
        );
        assert_eq!(
            TargetKind::from_cargo(&["proc-macro".to_string()]),
            TargetKind::Lib
        );
    }

    #[test]
    fn test_target_is_library_like_excludes_proc_macro() {
        let proc_macro_target = Target {
            name: "pollster_macro".to_string(),
            kind: vec!["proc-macro".to_string()],
            crate_types: vec!["proc-macro".to_string()],
            src_path: "src/lib.rs".to_string(),
        };
        assert!(!target_is_library_like(&proc_macro_target));

        let lib_target = Target {
            name: "pollster".to_string(),
            kind: vec!["lib".to_string()],
            crate_types: vec!["lib".to_string()],
            src_path: "src/lib.rs".to_string(),
        };
        assert!(target_is_library_like(&lib_target));
    }

    #[test]
    fn test_target_kind_test_capable() {
        assert!(TargetKind::Lib.is_test_capable());
        assert!(TargetKind::Bin.is_test_capable());
        assert!(TargetKind::Test.is_test_capable());
        assert!(!TargetKind::Example.is_test_capable());
        assert!(!TargetKind::Bench.is_test_capable());
    }

    #[test]
    fn test_cargo_expand_flag() {
        assert_eq!(TargetKind::Lib.cargo_expand_flag(), Some("--lib"));
        assert_eq!(TargetKind::Bin.cargo_expand_flag(), Some("--bin"));
        assert_eq!(TargetKind::Test.cargo_expand_flag(), Some("--test"));
    }

    #[test]
    fn test_module_name_from_target() {
        let target = CrateTarget {
            name: "my-crate".to_string(),
            kind: TargetKind::Lib,
            src_path: PathBuf::from("src/lib.rs"),
            module_name: "my_crate".to_string(),
        };
        assert_eq!(target.module_name, "my_crate");
    }

    #[test]
    fn test_normalize_module_base() {
        assert_eq!(normalize_module_base("cli-tool"), "cli_tool");
        assert_eq!(normalize_module_base("cfg.if"), "cfg_if");
        assert_eq!(normalize_module_base("123name"), "_123name");
    }

    #[test]
    fn test_assign_module_names_handles_normalized_collisions_deterministically() {
        let targets = assign_module_names(vec![
            RawTarget {
                name: "cli-tool".to_string(),
                kind: TargetKind::Bin,
                src_path: PathBuf::from("src/main.rs"),
            },
            RawTarget {
                name: "cli_tool".to_string(),
                kind: TargetKind::Test,
                src_path: PathBuf::from("tests/cli_tool.rs"),
            },
        ]);

        assert_eq!(targets.len(), 2);
        assert_eq!(targets[0].name, "cli-tool");
        assert_eq!(targets[0].module_name, "cli_tool");
        assert_eq!(targets[1].name, "cli_tool");
        assert_eq!(targets[1].module_name, "cli_tool_test");
    }

    #[test]
    fn test_assign_module_names_prefers_lib_base_name_when_colliding() {
        let targets = assign_module_names(vec![
            RawTarget {
                name: "demo-lib".to_string(),
                kind: TargetKind::Lib,
                src_path: PathBuf::from("src/lib.rs"),
            },
            RawTarget {
                name: "demo_lib".to_string(),
                kind: TargetKind::Test,
                src_path: PathBuf::from("tests/demo_lib.rs"),
            },
            RawTarget {
                name: "demo_lib".to_string(),
                kind: TargetKind::Bin,
                src_path: PathBuf::from("src/main.rs"),
            },
        ]);

        assert_eq!(targets.len(), 3);
        assert_eq!(targets[0].module_name, "demo_lib");
        assert_eq!(targets[1].module_name, "demo_lib_bin");
        assert_eq!(targets[2].module_name, "demo_lib_test");
    }

    #[test]
    fn test_select_target_package_prefers_manifest_owner_when_filter_missing() {
        let fixture = tempfile::tempdir().unwrap();
        let root_manifest = fixture.path().join("Cargo.toml");
        let xtask_manifest = fixture.path().join("xtask").join("Cargo.toml");
        std::fs::create_dir_all(xtask_manifest.parent().unwrap()).unwrap();
        std::fs::write(
            &root_manifest,
            "[package]\nname = \"root_pkg\"\nversion = \"0.1.0\"\n",
        )
        .unwrap();
        std::fs::write(
            &xtask_manifest,
            "[package]\nname = \"xtask\"\nversion = \"0.1.0\"\n",
        )
        .unwrap();

        let metadata = CargoMetadata {
            packages: vec![
                Package {
                    name: "xtask".to_string(),
                    version: "0.0.0".to_string(),
                    edition: "2015".to_string(),
                    rust_version: None,
                    targets: vec![],
                    manifest_path: xtask_manifest,
                    features: HashMap::new(),
                    dependencies: vec![],
                },
                Package {
                    name: "root_pkg".to_string(),
                    version: "0.1.0".to_string(),
                    edition: "2015".to_string(),
                    rust_version: None,
                    targets: vec![],
                    manifest_path: root_manifest.clone(),
                    features: HashMap::new(),
                    dependencies: vec![],
                },
            ],
            workspace_root: fixture.path().to_path_buf(),
        };

        let selected = select_target_package(&metadata, &root_manifest, None).unwrap();
        assert_eq!(selected.name, "root_pkg");
    }

    #[test]
    fn test_select_target_package_respects_explicit_filter() {
        let fixture = tempfile::tempdir().unwrap();
        let root_manifest = fixture.path().join("Cargo.toml");
        let member_manifest = fixture.path().join("xtask").join("Cargo.toml");
        std::fs::create_dir_all(member_manifest.parent().unwrap()).unwrap();
        std::fs::write(
            &root_manifest,
            "[package]\nname = \"root_pkg\"\nversion = \"0.1.0\"\n",
        )
        .unwrap();
        std::fs::write(
            &member_manifest,
            "[package]\nname = \"xtask\"\nversion = \"0.1.0\"\n",
        )
        .unwrap();

        let metadata = CargoMetadata {
            packages: vec![
                Package {
                    name: "root_pkg".to_string(),
                    version: "0.1.0".to_string(),
                    edition: "2015".to_string(),
                    rust_version: None,
                    targets: vec![],
                    manifest_path: root_manifest.clone(),
                    features: HashMap::new(),
                    dependencies: vec![],
                },
                Package {
                    name: "xtask".to_string(),
                    version: "0.0.0".to_string(),
                    edition: "2015".to_string(),
                    rust_version: None,
                    targets: vec![],
                    manifest_path: member_manifest,
                    features: HashMap::new(),
                    dependencies: vec![],
                },
            ],
            workspace_root: fixture.path().to_path_buf(),
        };

        let selected = select_target_package(&metadata, &root_manifest, Some("xtask")).unwrap();
        assert_eq!(selected.name, "xtask");
    }

    #[test]
    fn test_configured_cargo_build_target_uses_nearest_project_config() {
        let fixture = tempfile::tempdir().unwrap();
        let project = fixture.path().join("workspace/member");
        std::fs::create_dir_all(fixture.path().join(".cargo")).unwrap();
        std::fs::create_dir_all(project.join(".cargo")).unwrap();
        std::fs::write(
            fixture.path().join(".cargo/config.toml"),
            "[build]\ntarget='x86_64-pc-windows-msvc'\n",
        )
        .unwrap();
        std::fs::write(
            project.join(".cargo/config.toml"),
            "[build]\ntarget='x86_64-unknown-linux-gnu'\n",
        )
        .unwrap();

        assert_eq!(
            configured_cargo_build_target(&project).unwrap().as_deref(),
            Some("x86_64-unknown-linux-gnu")
        );
    }

    #[test]
    fn test_configured_cargo_build_target_uses_legacy_precedence_and_includes() {
        let fixture = tempfile::tempdir().unwrap();
        let cargo_dir = fixture.path().join(".cargo");
        std::fs::create_dir_all(&cargo_dir).unwrap();
        std::fs::write(
            cargo_dir.join("config.toml"),
            "[build]\ntarget='x86_64-unknown-linux-gnu'\n",
        )
        .unwrap();
        std::fs::write(
            cargo_dir.join("included.toml"),
            "[build]\ntarget='aarch64-unknown-linux-gnu'\n",
        )
        .unwrap();
        std::fs::write(cargo_dir.join("config"), "include=['included.toml']\n").unwrap();

        assert_eq!(
            configured_cargo_build_target(fixture.path())
                .unwrap()
                .as_deref(),
            Some("aarch64-unknown-linux-gnu")
        );
    }

    #[test]
    fn test_effective_graph_probe_preserves_workspace_resolution_context() {
        let fixture = tempfile::tempdir().unwrap();
        let workspace = fixture.path().join("workspace");
        let member = workspace.join("member");
        std::fs::create_dir_all(&member).unwrap();
        std::fs::write(
            workspace.join("Cargo.toml"),
            "[workspace]\nresolver='2'\n[patch.crates-io]\npatched={path='vendor/patched'}\n",
        )
        .unwrap();
        std::fs::write(workspace.join("Cargo.lock"), "probe-lock\n").unwrap();
        std::fs::write(member.join("Cargo.toml"), "[package]\nname='member'\n").unwrap();

        let identity = ManifestIdentity {
            package_name: "member".to_string(),
            edition: "2021".to_string(),
            rust_version: Some("1.85".to_string()),
            workspace_root: workspace.clone(),
            feature_names: Vec::new(),
            dependencies: Vec::new(),
            targets: Vec::new(),
        };
        let context = effective_graph_cargo_context(&[], &identity).unwrap();
        let probe =
            write_effective_graph_probe_manifest(&member.join("Cargo.toml"), &identity, &context)
                .expect("write effective-graph probe");
        let source = std::fs::read_to_string(probe.path.join("Cargo.toml")).unwrap();
        let manifest = toml::from_str::<toml::Value>(&source).unwrap();
        assert_eq!(manifest["package"]["edition"].as_str(), Some("2021"));
        assert_eq!(manifest["package"]["rust-version"].as_str(), Some("1.85"));
        assert_eq!(manifest["workspace"]["resolver"].as_str(), Some("2"));
        assert_eq!(
            manifest["patch"]["crates-io"]["patched"]["path"].as_str(),
            Some(workspace.join("vendor/patched").to_string_lossy().as_ref())
        );
        assert_eq!(
            std::fs::read_to_string(probe.path.join("Cargo.lock")).unwrap(),
            "probe-lock\n"
        );

        let path = probe.path.clone();
        drop(probe);
        assert!(!path.exists(), "effective-graph probe was not removed");
    }

    #[test]
    fn test_effective_graph_context_projects_only_selected_root_features() {
        let identity = ManifestIdentity {
            package_name: "root_pkg".to_string(),
            edition: "2024".to_string(),
            rust_version: None,
            workspace_root: PathBuf::from("/workspace"),
            feature_names: vec![
                "alpha".to_string(),
                "beta".to_string(),
                "default".to_string(),
            ],
            dependencies: Vec::new(),
            targets: Vec::new(),
        };

        let context = effective_graph_cargo_context(
            &[
                "--features".to_string(),
                "root_pkg/alpha,beta".to_string(),
                "--no-default-features".to_string(),
                "--target".to_string(),
                "aarch64-unknown-linux-gnu".to_string(),
                "--config=net.offline=true".to_string(),
            ],
            &identity,
        )
        .unwrap();
        assert_eq!(context.dependency_features, vec!["alpha", "beta"]);
        assert!(!context.default_features);
        assert_eq!(
            context.explicit_target.as_deref(),
            Some("aarch64-unknown-linux-gnu")
        );
        assert_eq!(
            context.non_feature_flags,
            vec![
                "--target",
                "aarch64-unknown-linux-gnu",
                "--config=net.offline=true",
            ]
        );

        let all_features =
            effective_graph_cargo_context(&["--all-features".to_string()], &identity).unwrap();
        assert_eq!(
            all_features.dependency_features,
            vec!["alpha", "beta", "default"]
        );
        assert!(all_features.default_features);
        assert!(all_features.explicit_target.is_none());
        assert!(all_features.non_feature_flags.is_empty());

        assert_eq!(
            effective_graph_probe_non_feature_flags(&[
                "--target".to_string(),
                "aarch64-unknown-linux-gnu".to_string(),
                "--locked".to_string(),
                "--frozen".to_string(),
                "--offline".to_string(),
                "--config=net.retry=0".to_string(),
            ]),
            vec![
                "--target",
                "aarch64-unknown-linux-gnu",
                "--offline",
                "--config=net.retry=0",
            ]
        );

        for selector in ["dependency/activate", "dependency?/activate"] {
            let error = effective_graph_cargo_context(
                &["--features".to_string(), selector.to_string()],
                &identity,
            )
            .expect_err("dependency-qualified features must fail closed");
            assert!(
                error.contains("unsupported Cargo dependency feature selector")
                    && error.contains(selector)
                    && error.contains("cannot exactly project"),
                "unexpected diagnostic for {selector}: {error}"
            );
        }
    }

    #[test]
    fn test_target_normal_package_witness_rejects_ambiguous_dependency_aliases() {
        let packages = vec![
            ResolvedPackage {
                id: "path+file:///fixture/parent#0.1.0".to_string(),
                name: "parent".to_string(),
                version: "0.1.0".to_string(),
                manifest_path: PathBuf::from("/fixture/parent/Cargo.toml"),
                source: None,
                targets: Vec::new(),
            },
            ResolvedPackage {
                id: "path+file:///fixture/child#0.1.0".to_string(),
                name: "child".to_string(),
                version: "0.1.0".to_string(),
                manifest_path: PathBuf::from("/fixture/child/Cargo.toml"),
                source: None,
                targets: Vec::new(),
            },
        ];
        let packages_by_id = packages
            .iter()
            .map(|package| (package.id.as_str(), package))
            .collect::<HashMap<_, _>>();
        let child_id = packages[1].id.clone();
        let mut node = ResolveNode {
            id: packages[0].id.clone(),
            deps: vec![
                ResolveDep {
                    name: "z_alias".to_string(),
                    pkg: child_id.clone(),
                    dep_kinds: vec![ResolveDepKind {
                        kind: None,
                        target: Some("cfg(unix)".to_string()),
                    }],
                },
                ResolveDep {
                    name: "a_alias".to_string(),
                    pkg: child_id.clone(),
                    dep_kinds: vec![ResolveDepKind {
                        kind: None,
                        target: Some("cfg(windows)".to_string()),
                    }],
                },
            ],
            features: Vec::new(),
        };
        let selected_pairs = HashSet::from([(packages[0].id.clone(), child_id)]);

        let error = reject_ambiguous_selected_local_aliases(
            &packages[0],
            &node,
            &packages_by_id,
            &selected_pairs,
        )
        .unwrap_err();
        assert!(error.contains("cannot distinguish dependency aliases a_alias, z_alias"));
        assert!(error.contains("from 'parent' to package 'child'"));
        assert!(error.contains("refusing to guess"));

        node.deps.truncate(1);
        reject_ambiguous_selected_local_aliases(
            &packages[0],
            &node,
            &packages_by_id,
            &selected_pairs,
        )
        .expect("one metadata alias is unambiguous");
    }

    #[test]
    fn test_discover_targets_prefers_manifest_owner_package_when_workspace_member_precedes_it() {
        let fixture = tempfile::tempdir().unwrap();
        let root_manifest = fixture.path().join("Cargo.toml");
        let root_src = fixture.path().join("src");
        let xtask_src = fixture.path().join("xtask").join("src");
        std::fs::create_dir_all(&root_src).unwrap();
        std::fs::create_dir_all(&xtask_src).unwrap();

        std::fs::write(
            &root_manifest,
            "[package]\nname = \"manifest_owned_fixture\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[workspace]\nmembers = [\"xtask\"]\nresolver = \"2\"\n",
        )
        .unwrap();
        std::fs::write(root_src.join("lib.rs"), "pub fn value() -> i32 { 7 }\n").unwrap();
        std::fs::write(
            fixture.path().join("xtask").join("Cargo.toml"),
            "[package]\nname = \"xtask\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
        )
        .unwrap();
        std::fs::write(xtask_src.join("main.rs"), "fn main() {}\n").unwrap();

        let (pkg_name, targets) = discover_targets(&root_manifest, None).unwrap();
        assert_eq!(pkg_name, "manifest_owned_fixture");
        assert_eq!(targets.len(), 1);
        assert_eq!(targets[0].name, "manifest_owned_fixture");
        assert_eq!(targets[0].kind, TargetKind::Lib);
    }

    #[test]
    fn test_resolved_extern_dependency_uses_cargo_crate_name_not_package_key() {
        let fixture = tempfile::tempdir().unwrap();
        let provider = fixture.path().join("provider");
        let consumer = fixture.path().join("consumer");
        std::fs::create_dir_all(provider.join("src")).unwrap();
        std::fs::create_dir_all(consumer.join("src")).unwrap();
        std::fs::write(
            provider.join("Cargo.toml"),
            "[package]\nname='innocent_package'\nversion='0.0.0'\nedition='2024'\n[lib]\nname='std'\n[workspace]\n",
        )
        .unwrap();
        std::fs::write(
            provider.join("src/lib.rs"),
            "#![no_std]\npub mod default { pub trait Default {} }\n",
        )
        .unwrap();
        std::fs::write(
            consumer.join("Cargo.toml"),
            "[package]\nname='consumer'\nversion='0.0.0'\nedition='2024'\n[dependencies]\ninnocent_package={path='../provider'}\n[workspace]\n",
        )
        .unwrap();
        std::fs::write(
            consumer.join("src/lib.rs"),
            "#![no_std]\npub fn value<T: std::default::Default>() {}\n",
        )
        .unwrap();
        let check = std::process::Command::new("cargo")
            .arg("check")
            .arg("--manifest-path")
            .arg(consumer.join("Cargo.toml"))
            .env("CARGO_TARGET_DIR", fixture.path().join("target"))
            .output()
            .unwrap();
        assert!(
            check.status.success(),
            "resolved extern-name fixture must be Cargo-valid:\n{}",
            String::from_utf8_lossy(&check.stderr)
        );
        let dependencies =
            inspect_resolved_extern_dependencies(&consumer.join("Cargo.toml")).unwrap();
        assert_eq!(
            dependencies,
            vec![ResolvedExternDependency {
                extern_crate_root: "std".to_string(),
                package_name: "innocent_package".to_string(),
            }]
        );
    }

    #[test]
    fn test_resolved_extern_dependency_uses_exact_feature_context() {
        let fixture = tempfile::tempdir().unwrap();
        let provider = fixture.path().join("provider");
        let consumer = fixture.path().join("consumer");
        std::fs::create_dir_all(provider.join("src")).unwrap();
        std::fs::create_dir_all(consumer.join("src")).unwrap();
        std::fs::write(
            provider.join("Cargo.toml"),
            "[package]\nname='innocent_package'\nversion='0.0.0'\nedition='2024'\n[lib]\nname='std'\n[workspace]\n",
        )
        .unwrap();
        std::fs::write(
            provider.join("src/lib.rs"),
            "#![no_std]\npub mod default { pub trait Default {} }\n",
        )
        .unwrap();
        std::fs::write(
            consumer.join("Cargo.toml"),
            "[package]\nname='feature_consumer'\nversion='0.0.0'\nedition='2024'\n[features]\ndefault=[]\nfake-std=['dep:innocent_package']\n[dependencies]\ninnocent_package={path='../provider',optional=true}\n[workspace]\n",
        )
        .unwrap();
        std::fs::write(
            consumer.join("src/lib.rs"),
            "#![no_std]\nextern crate std;\npub fn value<T: std::default::Default>() {}\n",
        )
        .unwrap();
        let manifest = consumer.join("Cargo.toml");

        let inactive = inspect_resolved_extern_dependencies_with_context(
            &manifest,
            Some("feature_consumer"),
            &["--no-default-features".to_string()],
        )
        .unwrap();
        assert!(inactive.is_empty(), "inactive optional leaked: {inactive:?}");

        for flags in [
            vec!["--features".to_string(), "fake-std".to_string()],
            vec!["--all-features".to_string()],
        ] {
            let active = inspect_resolved_extern_dependencies_with_context(
                &manifest,
                Some("feature_consumer"),
                &flags,
            )
            .unwrap();
            assert_eq!(
                active,
                vec![ResolvedExternDependency {
                    extern_crate_root: "std".to_string(),
                    package_name: "innocent_package".to_string(),
                }],
                "wrong graph for flags {flags:?}"
            );
        }
    }

    #[test]
    fn test_resolved_extern_dependency_honors_package_rename_and_target_filter() {
        let fixture = tempfile::tempdir().unwrap();
        let provider = fixture.path().join("provider");
        let consumer = fixture.path().join("consumer");
        std::fs::create_dir_all(provider.join("src")).unwrap();
        std::fs::create_dir_all(consumer.join("src")).unwrap();
        std::fs::write(
            provider.join("Cargo.toml"),
            "[package]\nname='renamed_provider'\nversion='0.0.0'\nedition='2024'\n[workspace]\n",
        )
        .unwrap();
        std::fs::write(provider.join("src/lib.rs"), "#![no_std]\n").unwrap();
        std::fs::write(
            consumer.join("Cargo.toml"),
            "[package]\nname='target_consumer'\nversion='0.0.0'\nedition='2024'\n[target.'cfg(target_pointer_width = \"64\")'.dependencies]\nstd={package='renamed_provider',path='../provider'}\n[workspace]\n",
        )
        .unwrap();
        std::fs::write(consumer.join("src/lib.rs"), "#![no_std]\n").unwrap();
        let manifest = consumer.join("Cargo.toml");

        let host = inspect_resolved_extern_dependencies_with_context(
            &manifest,
            Some("target_consumer"),
            &[
                "--target".to_string(),
                "x86_64-unknown-linux-gnu".to_string(),
            ],
        )
        .unwrap();
        assert_eq!(
            host,
            vec![ResolvedExternDependency {
                extern_crate_root: "std".to_string(),
                package_name: "renamed_provider".to_string(),
            }]
        );

        let other = inspect_resolved_extern_dependencies_with_context(
            &manifest,
            Some("target_consumer"),
            &[
                "--target".to_string(),
                "wasm32-unknown-unknown".to_string(),
            ],
        )
        .unwrap();
        assert!(other.is_empty(), "non-matching target edge leaked: {other:?}");
    }

    #[test]
    fn compilation_context_separates_normal_dev_build_features_and_unknown_sources() {
        let fixture = tempfile::tempdir().unwrap();
        for (directory, package, lib_name) in [
            ("normal", "normal_package", "normal_root"),
            ("dev", "dev_package", "std"),
            ("build", "build_package", "core"),
            ("optional", "optional_package", "alloc"),
            ("rusty", "innocent_rusty_package", "rusty"),
        ] {
            let provider = fixture.path().join(directory);
            std::fs::create_dir_all(provider.join("src")).unwrap();
            std::fs::write(
                provider.join("Cargo.toml"),
                format!(
                    "[package]\nname='{package}'\nversion='0.0.0'\nedition='2024'\n[lib]\nname='{lib_name}'\n[workspace]\n"
                ),
            )
            .unwrap();
            std::fs::write(provider.join("src/lib.rs"), "#![no_std]\n").unwrap();
        }

        let consumer = fixture.path().join("consumer");
        std::fs::create_dir_all(consumer.join("src/nested")).unwrap();
        std::fs::create_dir_all(consumer.join("tests")).unwrap();
        std::fs::write(
            consumer.join("Cargo.toml"),
            "[package]\nname='context_consumer'\nversion='0.0.0'\nedition='2024'\n\
             [features]\ndefault=[]\nfake-alloc=['dep:optional_package']\n\
             [dependencies]\nnormal_package={path='../normal'}\noptional_package={path='../optional',optional=true}\n\
             [dev-dependencies]\ndev_package={path='../dev'}\ninnocent_rusty_package={path='../rusty'}\n\
             [build-dependencies]\nbuild_package={path='../build'}\n\
             [[test]]\nname='integration'\npath='tests/integration.rs'\n\
             [workspace]\n",
        )
        .unwrap();
        std::fs::write(
            consumer.join("src/lib.rs"),
            "pub mod nested; pub fn value() {}\n",
        )
        .unwrap();
        std::fs::write(consumer.join("src/nested/mod.rs"), "pub fn nested() {}\n").unwrap();
        std::fs::write(consumer.join("tests/integration.rs"), "#[test] fn works() {}\n")
            .unwrap();
        std::fs::write(consumer.join("build.rs"), "fn main() {}\n").unwrap();
        let manifest = consumer.join("Cargo.toml");
        let (_, targets) = discover_targets_with_context(&manifest, None, &[]).unwrap();
        let lib = targets
            .iter()
            .find(|target| target.kind == TargetKind::Lib)
            .unwrap();
        let test = targets
            .iter()
            .find(|target| target.kind == TargetKind::Test)
            .unwrap();

        let dependency_roots = |context: CargoCompilationContext, flags: &[String]| {
            inspect_resolved_extern_dependencies_for_compilation(
                &manifest,
                None,
                flags,
                &context,
            )
            .unwrap()
            .into_iter()
            .map(|dependency| dependency.extern_crate_root)
            .collect::<HashSet<_>>()
        };
        assert_eq!(
            dependency_roots(CargoCompilationContext::exact(lib, false), &[]),
            HashSet::from(["normal_root".to_string()])
        );
        assert_eq!(
            dependency_roots(CargoCompilationContext::exact(test, false), &[]),
            HashSet::from([
                "normal_root".to_string(),
                "rusty".to_string(),
                "std".to_string(),
            ])
        );
        let build = CargoCompilationContext {
            target_name: Some("build-script-build".to_string()),
            target_kind: Some(TargetKind::Other("custom-build".to_string())),
            target_src_path: None,
            dependency_context: CargoDependencyContext::Build,
        };
        assert_eq!(
            dependency_roots(build, &[]),
            HashSet::from(["core".to_string()])
        );
        assert_eq!(
            dependency_roots(CargoCompilationContext::conservative(), &[]),
            HashSet::from([
                "core".to_string(),
                "normal_root".to_string(),
                "rusty".to_string(),
                "std".to_string(),
            ])
        );
        assert_eq!(
            dependency_roots(
                CargoCompilationContext::conservative(),
                &["--features".to_string(), "fake-alloc".to_string()],
            ),
            HashSet::from([
                "alloc".to_string(),
                "core".to_string(),
                "normal_root".to_string(),
                "rusty".to_string(),
                "std".to_string(),
            ])
        );

        assert_eq!(
            compilation_context_for_source(
                &manifest,
                None,
                &[],
                &consumer.join("src/lib.rs"),
                false,
            )
            .unwrap()
            .dependency_context(),
            CargoDependencyContext::Normal
        );
        assert_eq!(
            compilation_context_for_source(
                &manifest,
                None,
                &[],
                &consumer.join("tests/integration.rs"),
                false,
            )
            .unwrap()
            .dependency_context(),
            CargoDependencyContext::Development
        );
        let build_context = compilation_context_for_source(
            &manifest,
            None,
            &[],
            &consumer.join("build.rs"),
            false,
        )
        .unwrap();
        assert_eq!(
            build_context.dependency_context(),
            CargoDependencyContext::Build
        );
        assert!(
            build_context.cargo_target_args().is_err(),
            "build-script expansion silently selected another Cargo target"
        );
        assert_eq!(
            compilation_context_for_source(
                &manifest,
                None,
                &[],
                &consumer.join("src/nested/mod.rs"),
                false,
            )
            .unwrap()
            .dependency_context(),
            CargoDependencyContext::Unknown
        );
        assert!(
            compilation_context_for_source(
                &manifest,
                None,
                &[],
                &consumer.join("src/nested/mod.rs"),
                false,
            )
            .unwrap()
            .cargo_target_args()
            .is_err(),
            "nested module expansion silently selected Cargo's default target"
        );
    }

    #[test]
    fn effective_normal_graph_keeps_target_features_separate_from_build_union() {
        let fixture = tempfile::tempdir().unwrap();
        let root = fixture.path().join("root");
        let chooser = fixture.path().join("chooser");
        let poison = fixture.path().join("poison");
        for package in [&root, &chooser, &poison] {
            std::fs::create_dir_all(package.join("src")).unwrap();
        }
        std::fs::write(
            root.join("Cargo.toml"),
            "[package]\nname='context_root'\nversion='0.0.0'\nedition='2024'\nbuild='build.rs'\n\
             [dependencies]\ncontext_chooser={path='../chooser',default-features=false}\n\
             [build-dependencies]\ncontext_chooser={path='../chooser',default-features=false,features=['activate']}\n\
             [workspace]\nresolver='2'\n",
        )
        .unwrap();
        std::fs::write(root.join("src/lib.rs"), "pub fn value() {}\n").unwrap();
        std::fs::write(root.join("build.rs"), "fn main() {}\n").unwrap();
        std::fs::write(
            chooser.join("Cargo.toml"),
            "[package]\nname='context_chooser'\nversion='0.0.0'\nedition='2024'\n\
             [features]\ndefault=[]\nactivate=['dep:context_poison']\n\
             [dependencies]\ncontext_poison={path='../poison',optional=true}\n",
        )
        .unwrap();
        std::fs::write(chooser.join("src/lib.rs"), "pub fn value() {}\n").unwrap();
        std::fs::write(
            poison.join("Cargo.toml"),
            "[package]\nname='context_poison'\nversion='0.0.0'\nedition='2024'\n",
        )
        .unwrap();
        std::fs::write(poison.join("src/lib.rs"), "pub fn poison() {}\n").unwrap();

        let graph = resolve_effective_local_normal_dependency_graph_with_context(
            &root.join("Cargo.toml"),
            None,
            &[],
            &CrateGraphGitRepositories::default(),
        )
        .unwrap();
        let root_dependencies = graph
            .direct_dependencies(&root.join("Cargo.toml"))
            .unwrap();
        assert_eq!(root_dependencies.len(), 1);
        assert_eq!(root_dependencies[0].dependency_key, "context_chooser");
        assert!(
            root_dependencies[0].resolved_features.is_empty(),
            "build-only metadata feature union leaked into target context: {:?}",
            root_dependencies[0].resolved_features
        );
        assert!(
            graph
                .direct_dependencies(&chooser.join("Cargo.toml"))
                .unwrap()
                .is_empty(),
            "build-only optional dependency leaked into target-normal graph"
        );

        let lock = std::process::Command::new("cargo")
            .arg("generate-lockfile")
            .arg("--manifest-path")
            .arg(root.join("Cargo.toml"))
            .arg("--offline")
            .output()
            .unwrap();
        assert!(
            lock.status.success(),
            "could not establish locked fixture:\n{}",
            String::from_utf8_lossy(&lock.stderr)
        );
        for (label, flags) in [
            (
                "locked offline",
                vec!["--locked".to_string(), "--offline".to_string()],
            ),
            ("frozen", vec!["--frozen".to_string()]),
        ] {
            let locked_graph = resolve_effective_local_normal_dependency_graph_with_context(
                &root.join("Cargo.toml"),
                None,
                &flags,
                &CrateGraphGitRepositories::default(),
            )
            .unwrap_or_else(|error| panic!("{label} effective graph failed: {error}"));
            assert_eq!(
                locked_graph
                    .direct_dependencies(&root.join("Cargo.toml"))
                    .unwrap()
                    .iter()
                    .map(|dependency| dependency.dependency_key.as_str())
                    .collect::<Vec<_>>(),
                vec!["context_chooser"],
                "{label} changed the selected normal graph"
            );
        }
    }

    #[test]
    fn crate_graph_git_urls_compare_like_cargo_canonical_urls() {
        let canonical = |url: &str| canonical_git_repository_url(url);
        let lion = canonical("https://github.com/stonysystems/lion");
        assert_eq!(lion, "https://github.com/stonysystems/lion");
        for spelling in [
            "https://github.com/stonysystems/lion/",
            "https://github.com/stonysystems/lion.git",
            "https://github.com/stonysystems/lion.git/",
            "https://github.com/StonySystems/Lion",
            "HTTPS://GitHub.COM/stonysystems/lion",
            "http://github.com/stonysystems/lion",
        ] {
            assert_eq!(canonical(spelling), lion, "{spelling}");
        }
        assert_eq!(canonical("ssh://git@github.com/A/B.git"), "https://git@github.com/a/b");
        // Only github.com paths ignore case; other hosts keep scheme and path.
        assert_eq!(canonical("https://gitlab.com/A/B.git/"), "https://gitlab.com/A/B");
        assert_eq!(canonical("http://Example.com/A"), "http://example.com/A");
        assert_eq!(canonical("FILE:///tmp/Repo/"), "file:///tmp/Repo");
        assert_ne!(canonical("https://github.com/stonysystems/lion2"), lion);
    }

    #[test]
    fn crate_graph_git_names_repositories_only() {
        for rejected in [
            "",
            "git+https://github.com/stonysystems/lion",
            "https://github.com/stonysystems/lion?rev=d697b77",
            "https://github.com/stonysystems/lion#d697b77",
            "git@github.com:stonysystems/lion.git",
        ] {
            assert!(
                CrateGraphGitRepositories::from_urls(&[rejected.to_string()]).is_err(),
                "accepted {rejected:?}"
            );
        }
        let repositories = CrateGraphGitRepositories::from_urls(&[
            "https://github.com/stonysystems/lion".to_string(),
            "https://github.com/StonySystems/lion.git".to_string(),
        ])
        .unwrap();
        assert_eq!(repositories.named.len(), 1);
        assert!(CrateGraphGitRepositories::from_urls(&[]).unwrap().is_empty());

        let lion = "git+https://github.com/stonysystems/lion?rev=d697b779bb05314e7425b1746b5f89441e8e6712#d697b779bb05314e7425b1746b5f89441e8e6712";
        let vstd = "git+https://github.com/verus-lang/verus?rev=db81a74#db81a749e8d50000000000000000000000000000";
        assert_eq!(git_source_repository(lion), Some("https://github.com/stonysystems/lion"));
        assert_eq!(
            git_source_repository("registry+https://github.com/rust-lang/crates.io-index"),
            None
        );
        assert_eq!(git_source_repository("sparse+https://index.crates.io/"), None);
        assert!(repositories.admits(None));
        assert!(repositories.admits(Some(lion)));
        assert!(!repositories.admits(Some(vstd)));
        assert!(!repositories.admits(Some("registry+https://github.com/rust-lang/crates.io-index")));
        let default = CrateGraphGitRepositories::default();
        assert!(default.admits(None));
        assert!(!default.admits(Some(lion)));
    }

    #[test]
    fn crate_graph_git_sources_render_as_cargo_tree_prints_them() {
        // Each pair is `cargo metadata`'s source and `cargo tree`'s `{p}`
        // spelling of it, as Cargo 1.91-1.99 printed them.
        for (source, rendered) in [
            (
                "git+https://github.com/stonysystems/lion?rev=d697b779bb05314e7425b1746b5f89441e8e6712#d697b779bb05314e7425b1746b5f89441e8e6712",
                "https://github.com/stonysystems/lion?rev=d697b779bb05314e7425b1746b5f89441e8e6712#d697b779",
            ),
            (
                "git+https://github.com/verus-lang/verus?rev=db81a74#db81a749e8d50000000000000000000000000000",
                "https://github.com/verus-lang/verus?rev=db81a74#db81a749",
            ),
            (
                "git+file:///r/?branch=feat/x+y#419d705b73fb6b469b6a20159c83991dd093e5b2",
                "file:///r/?branch=feat%2Fx%2By#419d705b",
            ),
            (
                "git+file:///r?tag=v1.0#419d705b73fb6b469b6a20159c83991dd093e5b2",
                "file:///r?tag=v1.0#419d705b",
            ),
            (
                "git+file:///r#419d705b73fb6b469b6a20159c83991dd093e5b2",
                "file:///r#419d705b",
            ),
            (
                "git+file:///r.git?rev=419d705#419d705b73fb6b469b6a20159c83991dd093e5b2",
                "file:///r.git?rev=419d705#419d705b",
            ),
        ] {
            assert_eq!(cargo_tree_git_source(source).as_deref(), Some(rendered), "{source}");
        }
        assert_eq!(cargo_tree_git_source("git+file:///r?depth=1#419d705b"), None);
        assert_eq!(cargo_tree_git_source("registry+https://example.com/index"), None);
    }

    /// A probe metadata and tree for: root -> gcore (git, family repository)
    /// -> gbase (same repository) -> vstd (git, verus repository), and
    /// root -> lbase (path). The family repository's URL carries `.git`
    /// and the tree prints its branch form-urlencoded.
    const FAMILY: &str =
        "git+https://github.com/Example/Family.git?branch=feat/x+y#0123456789abcdef0123456789abcdef01234567";
    const VERUS: &str =
        "git+https://github.com/verus-lang/verus?rev=db81a74#db81a749e8d50000000000000000000000000000";

    fn synthetic_probe(extra_tree: &str) -> (CargoMetadataResolved, String) {
        let package = |id: &str, name: &str, version: &str, manifest: &str, source: Option<&str>| {
            serde_json::json!({
                "id": id, "name": name, "version": version, "manifest_path": manifest,
                "source": source, "targets": [],
            })
        };
        let node = |id: &str, deps: &[(&str, &str)]| {
            serde_json::json!({
                "id": id,
                "deps": deps.iter().map(|(name, pkg)| serde_json::json!({
                    "name": name, "pkg": pkg,
                    "dep_kinds": [{"kind": null, "target": null}],
                })).collect::<Vec<_>>(),
            })
        };
        let metadata = serde_json::json!({
            "packages": [
                package("probe", "rusty-cpp-effective-graph-probe", "0.0.0", "/probe/Cargo.toml", None),
                package("root", "graph-root", "0.1.0", "/w/root/Cargo.toml", None),
                package("lbase", "lbase", "0.1.0", "/w/lbase/Cargo.toml", None),
                package("gcore", "gcore", "0.2.0", "/checkouts/family/0123456/gcore/Cargo.toml", Some(FAMILY)),
                package("gbase", "gbase", "0.1.0", "/checkouts/family/0123456/gbase/Cargo.toml", Some(FAMILY)),
                package("vstd", "vstd", "0.0.0", "/checkouts/verus/db81a74/source/vstd/Cargo.toml", Some(VERUS)),
            ],
            "resolve": {"nodes": [
                node("probe", &[("__rusty_cpp_requested_root", "root")]),
                node("root", &[("gcore", "gcore"), ("lbase", "lbase")]),
                node("lbase", &[]),
                node("gcore", &[("gbase", "gbase")]),
                node("gbase", &[("vstd", "vstd")]),
                node("vstd", &[]),
            ]},
        });
        let family = "https://github.com/Example/Family.git?branch=feat%2Fx%2By#01234567";
        let tree = format!(
            "0rusty-cpp-effective-graph-probe v0.0.0 (/probe)|\n\
             1graph-root v0.1.0 (/w/root)|\n\
             2gcore v0.2.0 ({family})|default,fast\n\
             3gbase v0.1.0 ({family})|\n\
             4vstd v0.0.0 (https://github.com/verus-lang/verus?rev=db81a74#db81a749)|\n\
             2lbase v0.1.0 (/w/lbase)|\n{extra_tree}"
        );
        (serde_json::from_value(metadata).unwrap(), tree)
    }

    fn walk_synthetic(
        metadata: &CargoMetadataResolved,
        tree: &str,
        urls: &[&str],
    ) -> Result<EffectiveGraphWalk, String> {
        let repositories = CrateGraphGitRepositories::from_urls(
            &urls.iter().map(ToString::to_string).collect::<Vec<_>>(),
        )?;
        let selection = select_normal_target_packages(tree, metadata, "probe", &repositories)?;
        let root = metadata.packages.iter().find(|package| package.id == "root").unwrap();
        walk_effective_local_graph(
            &metadata.packages,
            metadata.resolve.as_ref().unwrap(),
            &selection,
            root,
            &repositories,
        )
    }

    fn graph_edges(walk: &EffectiveGraphWalk, manifest: &str) -> Vec<(String, Vec<String>)> {
        walk.direct_dependencies
            .get(Path::new(manifest))
            .unwrap_or_else(|| panic!("{manifest} is not a graph crate"))
            .iter()
            .map(|edge| (edge.package_name.clone(), edge.resolved_features.clone()))
            .collect()
    }

    #[test]
    fn crate_graph_git_repository_joins_the_effective_graph_only_when_named() {
        let (metadata, tree) = synthetic_probe("");
        let root = "/w/root/Cargo.toml";
        let gcore = "/checkouts/family/0123456/gcore/Cargo.toml";
        let gbase = "/checkouts/family/0123456/gbase/Cargo.toml";

        // Default: the git packages are opaque external nodes, exactly as
        // before (the tree witnesses no edge to an opaque node).
        let default = walk_synthetic(&metadata, &tree, &[]).unwrap();
        assert_eq!(graph_edges(&default, root), [("lbase".to_string(), vec![])]);
        assert_eq!(default.direct_dependencies.len(), 2, "only root and lbase");

        // Naming the repository (another spelling of it) brings in both of
        // its packages, with the tree's features; vstd stays external.
        let named = walk_synthetic(&metadata, &tree, &["https://github.com/example/family/"]).unwrap();
        assert_eq!(
            graph_edges(&named, root),
            [
                ("gcore".to_string(), vec!["default".to_string(), "fast".to_string()]),
                ("lbase".to_string(), vec![]),
            ]
        );
        assert_eq!(graph_edges(&named, gcore), [("gbase".to_string(), vec![])]);
        assert_eq!(graph_edges(&named, gbase), []);
        assert_eq!(named.direct_dependencies.len(), 4, "root, lbase, gcore, gbase");

        // vstd joins only when its own repository is named.
        let both = walk_synthetic(
            &metadata,
            &tree,
            &["https://github.com/example/family", "https://github.com/verus-lang/verus"],
        )
        .unwrap();
        assert_eq!(graph_edges(&both, gbase), [("vstd".to_string(), vec![])]);
    }

    #[test]
    fn crate_graph_git_rejects_a_repository_the_graph_does_not_reach() {
        let (metadata, tree) = synthetic_probe("");
        let error = match walk_synthetic(
            &metadata,
            &tree,
            &["https://github.com/example/family", "https://github.com/example/famliy"],
        ) {
            Ok(_) => panic!("an unreached repository was accepted"),
            Err(error) => error,
        };
        assert!(error.contains("--crate-graph-git https://github.com/example/famliy:"), "{error}");
        assert!(
            error.contains(
                "the git repositories its graph crates depend on: https://github.com/Example/Family.git, https://github.com/verus-lang/verus"
            ),
            "{error}"
        );
        // Unreached even though Cargo resolved it: vstd sits behind gbase,
        // which is not a graph crate unless the family repository is named.
        let error = walk_synthetic(&metadata, &tree, &["https://github.com/verus-lang/verus"])
            .err()
            .expect("verus is not reached without the family repository");
        assert!(error.ends_with("depend on: https://github.com/Example/Family.git"), "{error}");
    }

    #[test]
    fn crate_graph_git_fails_closed_on_an_unrecognized_tree_line() {
        // A line that names a graph crate but not in Cargo's spelling (a
        // 10-digit commit) must not silently become an opaque node.
        let (metadata, tree) = synthetic_probe("");
        let tree = tree.replace("#01234567)|default", "#0123456789)|default");
        let error = walk_synthetic(&metadata, &tree, &["https://github.com/example/family"])
            .err()
            .expect("an inexact git rendering was accepted");
        assert!(error.contains("inexact local package identity"), "{error}");
        // Without the opt-in the same line is an opaque external node.
        assert!(walk_synthetic(&metadata, &tree, &[]).is_ok());
    }

    #[test]
    fn crate_graph_git_rejects_two_graph_crates_of_one_name() {
        // The repository at a second reference brings a second `gbase`.
        let (mut metadata, _) = synthetic_probe("");
        let other = FAMILY.replace("?branch=feat/x+y#0123456789", "?rev=fedcba9#fedcba9876");
        metadata.packages.push(serde_json::from_value(serde_json::json!({
            "id": "gbase-2", "name": "gbase", "version": "0.1.0",
            "manifest_path": "/checkouts/family/fedcba9/gbase/Cargo.toml",
            "source": other, "targets": [],
        })).unwrap());
        let resolve = metadata.resolve.as_mut().unwrap();
        resolve.nodes.push(serde_json::from_value(serde_json::json!({"id": "gbase-2"})).unwrap());
        let root = resolve.nodes.iter_mut().find(|node| node.id == "root").unwrap();
        root.deps.push(serde_json::from_value(serde_json::json!({
            "name": "gbase2", "pkg": "gbase-2", "dep_kinds": [{"kind": null, "target": null}],
        })).unwrap());
        let (_, tree) = synthetic_probe(
            "2gbase v0.1.0 (https://github.com/Example/Family.git?rev=fedcba9#fedcba98)|\n",
        );
        let error = walk_synthetic(&metadata, &tree, &["https://github.com/example/family"])
            .err()
            .expect("two graph crates named gbase were accepted");
        assert!(error.contains("share the package name 'gbase'"), "{error}");
    }

    /// [`walk_synthetic`] under `--crate-graph`'s policy.
    fn walk_synthetic_refusing(
        metadata: &CargoMetadataResolved,
        tree: &str,
        urls: &[&str],
        verus_exec: bool,
    ) -> Result<EffectiveGraphWalk, String> {
        let repositories = CrateGraphGitRepositories::from_urls(
            &urls.iter().map(ToString::to_string).collect::<Vec<_>>(),
        )?
        .refusing_unnamed(verus_exec);
        let selection = select_normal_target_packages(tree, metadata, "probe", &repositories)?;
        let root = metadata.packages.iter().find(|package| package.id == "root").unwrap();
        walk_effective_local_graph(
            &metadata.packages,
            metadata.resolve.as_ref().unwrap(),
            &selection,
            root,
            &repositories,
        )
    }

    #[test]
    fn crate_graph_refuses_only_git_packages_of_unnamed_repositories() {
        let lion = "git+https://github.com/stonysystems/lion?rev=d697b77#d697b779bb05314e7425b1746b5f89441e8e6712";
        let registry = "registry+https://github.com/rust-lang/crates.io-index";
        let lenient = CrateGraphGitRepositories::default();
        assert!(!lenient.refuses(Some(lion)) && !lenient.refuses(Some(VERUS)));
        let strict = CrateGraphGitRepositories::default().refusing_unnamed(false);
        assert!(strict.refuses(Some(lion)) && strict.refuses(Some(VERUS)));
        assert!(!strict.refuses(None) && !strict.refuses(Some(registry)));
        let named = CrateGraphGitRepositories::from_urls(&["https://github.com/StonySystems/lion.git".to_string()])
            .unwrap()
            .refusing_unnamed(false);
        assert!(!named.refuses(Some(lion)));
        // --verus-exec exempts Verus's repository, in any spelling Cargo
        // takes for the same repository, and nothing else.
        let verus_exec = CrateGraphGitRepositories::default().refusing_unnamed(true);
        assert!(!verus_exec.refuses(Some(VERUS)));
        assert!(!verus_exec.refuses(Some("git+https://github.com/Verus-Lang/verus.git?branch=main#db81a749e8d5")));
        assert!(verus_exec.refuses(Some(lion)));
        assert!(verus_exec.refuses(Some("git+https://github.com/verus-lang/verus-fork?rev=db81a74#db81a749e8d5")));
        // The lenient form of each is what every resolution outside
        // --crate-graph uses; refusing changes nothing about admission.
        assert!(named.admits(Some(lion)));
        assert_eq!(strict.admits(Some(lion)), lenient.admits(Some(lion)));
    }

    #[test]
    fn crate_graph_refuses_a_graph_crate_depending_on_an_unnamed_git_repository() {
        let (metadata, tree) = synthetic_probe("");
        // Nothing named: the root's edge to gcore is refused. gbase -> vstd is
        // not, because gbase is not a graph crate.
        let error = walk_synthetic_refusing(&metadata, &tree, &[], true)
            .err()
            .expect("an unnamed git dependency was accepted");
        assert!(
            error.starts_with("graph crates depend on git packages whose repository no --crate-graph-git names;"),
            "{error}"
        );
        assert!(
            error.contains("\n  graph-root -> gcore v0.2.0 (https://github.com/Example/Family.git)\n"),
            "{error}"
        );
        assert!(error.ends_with("graph crates: --crate-graph-git https://github.com/Example/Family.git"), "{error}");
        assert!(!error.contains("vstd") && !error.contains("Verus"), "{error}");

        // The family named, without --verus-exec: gbase -> vstd is refused, and
        // the remedy is --verus-exec, not naming Verus's repository.
        let error = walk_synthetic_refusing(&metadata, &tree, &["https://github.com/example/family"], false)
            .err()
            .expect("vstd was accepted without --verus-exec");
        assert!(error.contains("\n  gbase -> vstd v0.0.0 (https://github.com/verus-lang/verus)\n"), "{error}");
        assert!(!error.contains("gcore") && !error.contains("--crate-graph-git https"), "{error}");
        assert!(error.ends_with("external under --verus-exec, which erases or lowers every use of vstd"), "{error}");

        // With --verus-exec the same graph is accepted, and it is exactly the
        // lenient walk's: vstd stays external.
        let strict = walk_synthetic_refusing(&metadata, &tree, &["https://github.com/example/family"], true).unwrap();
        let lenient = walk_synthetic(&metadata, &tree, &["https://github.com/example/family"]).unwrap();
        assert_eq!(strict.direct_dependencies, lenient.direct_dependencies);
        assert_eq!(strict.extern_dependencies, lenient.extern_dependencies);
        // Naming Verus's repository too still works: vstd becomes a graph crate.
        let both = walk_synthetic_refusing(
            &metadata,
            &tree,
            &["https://github.com/example/family", "https://github.com/verus-lang/verus"],
            false,
        )
        .unwrap();
        assert_eq!(graph_edges(&both, "/checkouts/family/0123456/gbase/Cargo.toml"), [("vstd".to_string(), vec![])]);

        // The unreached-repository error comes first, as it did before.
        let error = walk_synthetic_refusing(&metadata, &tree, &["https://github.com/example/famliy"], true)
            .err()
            .unwrap();
        assert!(error.starts_with("--crate-graph-git https://github.com/example/famliy: no package"), "{error}");
    }

    #[test]
    fn crate_graph_refuses_only_the_edges_cargo_selects() {
        // `gwin` comes from an unnamed repository and is in the resolve graph
        // (a dependency for another target, say), but the target's tree does
        // not select it: it is not refused.
        let (mut metadata, tree) = synthetic_probe("");
        let other = "git+https://github.com/example/other?rev=89abcde#89abcdef0123456789abcdef0123456789abcdef";
        metadata.packages.push(serde_json::from_value(serde_json::json!({
            "id": "gwin", "name": "gwin", "version": "0.1.0",
            "manifest_path": "/checkouts/other/89abcde/Cargo.toml", "source": other, "targets": [],
        })).unwrap());
        let resolve = metadata.resolve.as_mut().unwrap();
        resolve.nodes.push(serde_json::from_value(serde_json::json!({"id": "gwin"})).unwrap());
        let root = resolve.nodes.iter_mut().find(|node| node.id == "root").unwrap();
        root.deps.push(serde_json::from_value(serde_json::json!({
            "name": "gwin", "pkg": "gwin", "dep_kinds": [{"kind": null, "target": "cfg(windows)"}],
        })).unwrap());
        assert!(walk_synthetic_refusing(&metadata, &tree, &["https://github.com/example/family"], true).is_ok());
        // Selected, it is refused.
        let selected = tree.replace(
            "2lbase v0.1.0 (/w/lbase)|\n",
            "2lbase v0.1.0 (/w/lbase)|\n2gwin v0.1.0 (https://github.com/example/other?rev=89abcde#89abcdef)|\n",
        );
        let error = walk_synthetic_refusing(&metadata, &selected, &["https://github.com/example/family"], true)
            .err()
            .expect("a selected unnamed git dependency was accepted");
        assert!(error.contains("graph-root -> gwin v0.1.0 (https://github.com/example/other)"), "{error}");
        // A refused package that a non-graph crate depends on is not read.
        let below_vstd = tree.replace(
            "4vstd v0.0.0 (https://github.com/verus-lang/verus?rev=db81a74#db81a749)|\n",
            "4vstd v0.0.0 (https://github.com/verus-lang/verus?rev=db81a74#db81a749)|\n5gwin v0.1.0 (https://github.com/example/other?rev=89abcde#89abcdef)|\n",
        );
        assert!(walk_synthetic_refusing(&metadata, &below_vstd, &["https://github.com/example/family"], true).is_ok());
    }

    #[test]
    fn crate_graph_refusal_fails_closed_on_an_unrecognized_tree_line() {
        // gcore's line in a spelling Cargo does not print (a 10-digit commit)
        // must not pass as an opaque node when gcore would be refused.
        let (metadata, tree) = synthetic_probe("");
        let tree = tree.replace("#01234567)|default", "#0123456789)|default");
        let error = walk_synthetic_refusing(&metadata, &tree, &[], true)
            .err()
            .expect("an inexact refused rendering was accepted");
        assert!(error.contains("inexact git package identity on line 3"), "{error}");
        // Outside --crate-graph the line stays opaque, as before.
        assert!(walk_synthetic(&metadata, &tree, &[]).is_ok());
    }
}
