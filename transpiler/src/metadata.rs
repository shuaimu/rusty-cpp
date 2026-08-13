use serde::Deserialize;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

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
}

#[derive(Debug, Clone, PartialEq)]
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
pub fn discover_targets(
    manifest_path: &Path,
    package_filter: Option<&str>,
) -> Result<(String, Vec<CrateTarget>), String> {
    let project_dir = manifest_path.parent().unwrap_or(Path::new("."));

    let output = std::process::Command::new("cargo")
        .arg("metadata")
        .arg("--format-version")
        .arg("1")
        .arg("--no-deps")
        .arg("--manifest-path")
        .arg(manifest_path)
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
pub fn inspect_manifest_identity(manifest_path: &Path) -> Result<ManifestIdentity, String> {
    let project_dir = manifest_path.parent().unwrap_or(Path::new("."));
    let output = std::process::Command::new("cargo")
        .arg("metadata")
        .arg("--format-version")
        .arg("1")
        .arg("--no-deps")
        .arg("--manifest-path")
        .arg(manifest_path)
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
    let requested_manifest = canonicalized_path(manifest_path);
    let package = metadata
        .packages
        .iter()
        .find(|package| canonicalized_path(&package.manifest_path) == requested_manifest)
        .ok_or_else(|| {
            format!(
                "cargo metadata did not report the requested package manifest {}",
                manifest_path.display()
            )
        })?;
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
        dependencies,
        targets,
    })
}

fn configured_cargo_build_target(project_dir: &Path) -> Result<Option<String>, String> {
    let mut cargo_directories = Vec::new();
    if let Some(cargo_home) = std::env::var_os("CARGO_HOME") {
        cargo_directories.push(PathBuf::from(cargo_home));
    } else if let Some(user_home) = std::env::var_os("HOME") {
        cargo_directories.push(PathBuf::from(user_home).join(".cargo"));
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
        let modern_exists = modern.is_file();
        let legacy_exists = legacy.is_file();
        if modern_exists && legacy_exists {
            return Err(format!(
                "both {} and {} exist; refusing to guess Cargo's build.target precedence",
                modern.display(),
                legacy.display()
            ));
        }
        let config_path = if modern_exists {
            modern
        } else if legacy_exists {
            legacy
        } else {
            continue;
        };
        let source = std::fs::read_to_string(&config_path).map_err(|error| {
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
        let Some(target) = config
            .get("build")
            .and_then(toml::Value::as_table)
            .and_then(|build| build.get("target"))
        else {
            continue;
        };
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

fn write_effective_graph_probe_manifest(
    requested_manifest: &Path,
    identity: &ManifestIdentity,
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
fn resolve_normal_target_package_edges(
    probe_manifest: &Path,
    project_dir: &Path,
    target_triple: &str,
    metadata: &CargoMetadataResolved,
    expected_root_package_id: &str,
) -> Result<HashSet<(String, String)>, String> {
    let output = std::process::Command::new("cargo")
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
        .arg("{p}")
        .arg("--color")
        .arg("never")
        .arg("--quiet")
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

    // Cargo's documented `{p}` rendering for local path packages includes
    // the exact package name/version and package directory. Construct every
    // accepted rendering from the same metadata invocation, require it to be
    // unique, and fail closed if a local-looking line differs. Registry/git
    // nodes may remain opaque because local traversal stops at them.
    let mut local_by_render = HashMap::<String, String>::new();
    let mut local_prefixes = Vec::<String>::new();
    for package in metadata
        .packages
        .iter()
        .filter(|package| package.source.is_none())
    {
        let package_dir = package.manifest_path.parent().ok_or_else(|| {
            format!(
                "Cargo metadata reported a local manifest without a parent: {}",
                package.manifest_path.display()
            )
        })?;
        let prefix = format!("{} v{} ", package.name, package.version);
        let proc_macro_annotation = if package.targets.iter().any(target_is_proc_macro) {
            "(proc-macro) "
        } else {
            ""
        };
        let rendered = format!("{prefix}{proc_macro_annotation}({})", package_dir.display());
        if let Some(previous) = local_by_render.insert(rendered.clone(), package.id.clone()) {
            return Err(format!(
                "Cargo normal-tree rendering is ambiguous for package ids '{previous}' and '{}': {rendered}",
                package.id
            ));
        }
        local_prefixes.push(prefix);
    }
    local_prefixes.sort();
    local_prefixes.dedup();

    let mut stack = Vec::<Option<String>>::new();
    let mut edges = HashSet::new();
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
        let rendered = &line[depth_end..];
        let package_id = local_by_render.get(rendered).cloned();
        if package_id.is_none()
            && local_prefixes
                .iter()
                .any(|prefix| rendered.starts_with(prefix))
        {
            return Err(format!(
                "Cargo normal tree reported an inexact local package identity on line {}: {rendered}",
                line_index + 1
            ));
        }

        stack.truncate(depth);
        if depth == 0 {
            roots.push(package_id.clone());
        } else if let (Some(Some(parent)), Some(child)) = (stack.last(), package_id.as_ref()) {
            edges.insert((parent.clone(), child.clone()));
        }
        stack.push(package_id);
    }
    if roots.len() != 1 || roots[0].as_deref() != Some(expected_root_package_id) {
        return Err(format!(
            "cargo tree normal-graph root did not match probe package '{expected_root_package_id}': {roots:?}"
        ));
    }
    Ok(edges)
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
pub fn resolve_effective_local_normal_dependency_graph(
    manifest_path: &Path,
) -> Result<EffectiveLocalNormalDependencyGraph, String> {
    let project_dir = manifest_path.parent().unwrap_or(Path::new("."));
    let target_triple = effective_target_triple(project_dir)?;
    let requested_manifest = canonicalized_path(manifest_path);
    let requested_identity = inspect_manifest_identity(manifest_path)?;
    let probe = write_effective_graph_probe_manifest(&requested_manifest, &requested_identity)?;
    let output = std::process::Command::new("cargo")
        .arg("metadata")
        .arg("--format-version")
        .arg("1")
        .arg("--manifest-path")
        .arg(probe.path.join("Cargo.toml"))
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
    let normal_package_edges = resolve_normal_target_package_edges(
        &probe.path.join("Cargo.toml"),
        project_dir,
        &target_triple,
        &metadata,
        &probe_package.id,
    )?;
    let selected = select_resolved_package(&metadata, &requested_manifest, None)?;
    let root_manifest = canonicalized_path(&selected.manifest_path);
    let resolve = metadata.resolve.as_ref().ok_or_else(|| {
        "effective-graph cargo metadata did not report a resolved dependency graph".to_string()
    })?;

    let packages_by_id = metadata
        .packages
        .iter()
        .map(|package| (package.id.as_str(), package))
        .collect::<HashMap<_, _>>();
    let nodes_by_id = resolve
        .nodes
        .iter()
        .map(|node| (node.id.as_str(), node))
        .collect::<HashMap<_, _>>();
    if !nodes_by_id.contains_key(selected.id.as_str()) {
        return Err(format!(
            "effective-graph cargo metadata omitted the root resolve node for {}",
            manifest_path.display()
        ));
    }

    let mut direct_dependencies = HashMap::new();
    let mut pending = vec![selected.id.as_str()];
    let mut visited = HashSet::new();
    while let Some(package_id) = pending.pop() {
        if !visited.insert(package_id.to_string()) {
            continue;
        }
        let package = packages_by_id.get(package_id).ok_or_else(|| {
            format!("effective Cargo resolve graph references unknown package id '{package_id}'")
        })?;
        if package.source.is_some() && package_id != selected.id {
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
            &normal_package_edges,
        )?;

        let mut edges = Vec::new();
        for dependency in &node.deps {
            let has_normal_kind = dependency.dep_kinds.is_empty()
                || dependency.dep_kinds.iter().any(|kind| kind.kind.is_none());
            if !has_normal_kind {
                continue;
            }
            if !normal_package_edges.contains(&(package.id.clone(), dependency.pkg.clone())) {
                continue;
            }
            let dependency_package =
                packages_by_id.get(dependency.pkg.as_str()).ok_or_else(|| {
                    format!(
                        "effective Cargo resolve graph references unknown dependency id '{}'",
                        dependency.pkg
                    )
                })?;
            if dependency_package.source.is_some() {
                continue;
            }
            if dependency.name.trim().is_empty() {
                return Err(format!(
                    "effective Cargo metadata reported an empty dependency key from '{}' to '{}'",
                    package.name, dependency_package.name
                ));
            }
            let dependency_manifest = canonicalized_path(&dependency_package.manifest_path);
            edges.push(EffectiveLocalNormalDependency {
                dependency_key: dependency.name.clone(),
                package_name: dependency_package.name.clone(),
                manifest_path: dependency_manifest,
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

    Ok(EffectiveLocalNormalDependencyGraph {
        target_triple,
        root_manifest,
        direct_dependencies,
    })
}

/// Discover resolved dependency packages for the selected package.
///
/// Returns dependency packages in deterministic dependency order
/// (dependencies first), filtered to unconditional normal dependencies
/// (`kind = null`, `target = null`; optionally `kind = dev`) from the
/// resolved graph.
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
    for flag in cargo_flags {
        cmd.arg(flag);
    }
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
    if let Some(resolve) = &metadata.resolve {
        for node in &resolve.nodes {
            let mut deps = Vec::new();
            for dep in &node.deps {
                // Keep only unconditional normal dependencies; skip dev/build and
                // target-qualified edges we cannot soundly evaluate here.
                let include = dep.dep_kinds.is_empty()
                    || dep.dep_kinds.iter().any(|kind| {
                        if kind.target.is_some() {
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
                    dependencies: vec![],
                },
                Package {
                    name: "root_pkg".to_string(),
                    version: "0.1.0".to_string(),
                    edition: "2015".to_string(),
                    rust_version: None,
                    targets: vec![],
                    manifest_path: root_manifest.clone(),
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
                    dependencies: vec![],
                },
                Package {
                    name: "xtask".to_string(),
                    version: "0.0.0".to_string(),
                    edition: "2015".to_string(),
                    rust_version: None,
                    targets: vec![],
                    manifest_path: member_manifest,
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
    fn test_configured_cargo_build_target_rejects_ambiguous_config_files() {
        let fixture = tempfile::tempdir().unwrap();
        let cargo_dir = fixture.path().join(".cargo");
        std::fs::create_dir_all(&cargo_dir).unwrap();
        std::fs::write(
            cargo_dir.join("config.toml"),
            "[build]\ntarget='x86_64-unknown-linux-gnu'\n",
        )
        .unwrap();
        std::fs::write(
            cargo_dir.join("config"),
            "[build]\ntarget='x86_64-pc-windows-msvc'\n",
        )
        .unwrap();

        let error = configured_cargo_build_target(fixture.path()).unwrap_err();
        assert!(error.contains("refusing to guess Cargo's build.target precedence"));
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
            dependencies: Vec::new(),
            targets: Vec::new(),
        };
        let probe = write_effective_graph_probe_manifest(&member.join("Cargo.toml"), &identity)
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
}
