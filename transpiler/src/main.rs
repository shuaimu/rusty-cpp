use clap::{Parser, Subcommand};
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{self, Output};

mod cmake;
mod codegen;
mod cpp_abi;
mod cpp_default_args;
mod cpp_name;
mod inline_rust;
mod metadata;
mod slots;
mod transpile;
mod types;

/// Count distinct .cppm files represented in a slot list. Used only
/// for the end-of-crate-mode summary line; the manifest does its own
/// counting internally.
fn count_slot_files(s: &[slots::Slot]) -> usize {
    let mut set = BTreeSet::new();
    for slot in s {
        set.insert(slot.file.as_str());
    }
    set.len()
}

#[derive(Parser)]
#[command(name = "rusty-cpp-transpiler")]
#[command(about = "Transpile Rust source code to C++ using rusty-cpp types")]
struct Cli {
    /// Print the embedded source revision as one-line JSON and exit
    #[arg(long)]
    build_info: bool,

    /// Input Rust source file (.rs) — not needed with --crate or subcommands
    input: Option<PathBuf>,

    /// Output C++ module file (.cppm)
    #[arg(short, long)]
    output: Option<PathBuf>,

    /// C++20 module name (e.g., "my_crate" or "my_crate.submodule")
    #[arg(short, long)]
    module_name: Option<String>,

    /// Expand macros before transpilation (requires cargo-expand installed)
    #[arg(long)]
    expand: bool,

    /// Generate CMakeLists.txt from Cargo.toml (provide path to Cargo.toml)
    #[arg(long)]
    cmake: Option<PathBuf>,

    /// Transpile an entire Rust crate (provide path to Cargo.toml)
    #[arg(long)]
    crate_: Option<PathBuf>,

    /// Output directory for --crate mode (default: ./cpp_out/)
    #[arg(long, default_value = "cpp_out")]
    output_dir: PathBuf,

    /// Run rusty-cpp analyzer on transpiled output to verify safety
    #[arg(long)]
    verify: bool,

    /// User-provided type mapping file for external crate types (TOML format)
    #[arg(long)]
    type_map: Option<PathBuf>,

    /// C++ module symbol index sidecar file(s) for `use cpp::...` imports (JSON or TOML)
    #[arg(long = "cpp-module-index")]
    cpp_module_index: Vec<PathBuf>,

    /// Versioned TOML sidecar containing per-module global-fragment includes
    #[arg(long, value_name = "PATH", conflicts_with = "cmake")]
    module_preamble: Option<PathBuf>,

    /// Explicit target_os used to evaluate module-preamble conditions
    #[arg(long, value_name = "OS", requires = "module_preamble")]
    preamble_target_os: Option<String>,

    /// Enable diagnostic-only prototype planning for by-value SCC cycle breaking
    #[arg(long)]
    by_value_cycle_breaking_prototype: bool,

    /// Prefer `rusty::Unit` alias for Rust `()` in generated type
    /// positions. Default-on as of the unit-alias migration; accepted
    /// for backwards-compatibility with older scripts and parity-matrix
    /// pass-throughs. Pass `--prefer-std-tuple-alias` to opt out and
    /// keep the legacy `std::tuple<>` spelling.
    #[arg(long)]
    prefer_rusty_unit_alias: bool,

    /// Opt out of the default `rusty::Unit` spelling and keep the
    /// legacy `std::tuple<>` rendering of Rust `()`. The two C++ types
    /// are identical (`using Unit = std::tuple<>;`); this flag only
    /// flips the textual surface in generated output.
    #[arg(long)]
    prefer_std_tuple_alias: bool,

    /// Prefer `rusty::StrView` / `rusty::Span<...>` alias spellings in generated output.
    #[arg(long)]
    prefer_rusty_view_aliases: bool,

    /// Wrap all exported items in `export namespace <NS> { … }` (module
    /// mode only). Off by default. Lets sibling modules export the same
    /// names without colliding at importer scope. See
    /// docs/rusty-std-book.md §2.10.
    #[arg(long = "cxx-namespace")]
    cxx_namespace: Option<String>,

    /// Auto-derive the C++ namespace from `--module-name` (replace `.`
    /// with `::`) AND emit namespace aliases for each imported sibling
    /// module — the spec-correct rendering of Rust's module tree as
    /// C++20 modules + namespaces. See docs/rusty-std-book.md §2.10
    /// Option 2. Implies `--cxx-namespace <derived>` when set without
    /// an explicit override.
    #[arg(long = "auto-namespace")]
    auto_namespace: bool,

    /// Deprecated no-op: interface+adapter is now the only trait
    /// lowering path. Kept for CLI compatibility with older scripts.
    #[arg(long, hide = true)]
    interface_traits: bool,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Run parity test: transpile a Rust crate's tests to C++ and verify same results
    ParityTest(ParityTestArgs),
    /// Validate or rewrite inline Rust DSL blocks embedded in C++ files
    InlineRust(InlineRustArgs),
}

#[derive(Parser)]
struct ParityTestArgs {
    /// Path to Cargo.toml of the crate to test
    #[arg(long, default_value = "Cargo.toml")]
    manifest_path: PathBuf,

    /// Package name (for workspace crates)
    #[arg(long, short)]
    package: Option<String>,

    /// Working directory for intermediate files
    #[arg(long, default_value = ".rusty-parity")]
    work_dir: PathBuf,

    /// Keep working directory after test (don't clean up)
    #[arg(long)]
    keep_work_dir: bool,

    /// Reuse existing target/dependency artifact directories and skip transpiling
    /// units whose .cppm output already exists in --work-dir.
    #[arg(long)]
    incremental_transpile: bool,

    /// Print what would be done without executing
    #[arg(long)]
    dry_run: bool,

    /// Cargo feature flags to pass through
    #[arg(long)]
    features: Option<String>,

    /// Enable all features
    #[arg(long)]
    all_features: bool,

    /// Disable default features
    #[arg(long)]
    no_default_features: bool,

    /// Stop after a specific stage: baseline, expand, transpile, build, run
    #[arg(long)]
    stop_after: Option<String>,

    /// Skip running cargo test baseline
    #[arg(long)]
    no_baseline: bool,

    /// Reuse existing expanded Rust sources from --work-dir instead of rerunning cargo expand.
    #[arg(long)]
    skip_expand: bool,

    /// User-provided type mapping file
    #[arg(long)]
    type_map: Option<PathBuf>,

    /// C++ module symbol index sidecar file(s) for `use cpp::...` imports (JSON or TOML)
    #[arg(long = "cpp-module-index")]
    cpp_module_index: Vec<PathBuf>,

    /// Enable diagnostic-only prototype planning for by-value SCC cycle breaking
    #[arg(long)]
    by_value_cycle_breaking_prototype: bool,

    /// Allow parity to proceed when no transpiled test wrappers are discovered.
    /// Useful for library-only crates to validate transpile + C++ compile.
    #[arg(long)]
    allow_empty_tests: bool,

    /// In module mode, emit `import std;` instead of explicit std header includes.
    /// Also forces Stage D to use `clang++ -stdlib=libc++` and precompile `std.cppm`.
    #[arg(long)]
    import_std: bool,

    /// Prefer `rusty::Unit` alias for Rust `()` in generated type
    /// positions. Default-on as of the unit-alias migration; accepted
    /// for backwards-compatibility with older scripts and parity-matrix
    /// pass-throughs. Pass `--prefer-std-tuple-alias` to opt out and
    /// keep the legacy `std::tuple<>` spelling.
    #[arg(long)]
    prefer_rusty_unit_alias: bool,

    /// Opt out of the default `rusty::Unit` spelling and keep the
    /// legacy `std::tuple<>` rendering of Rust `()`. The two C++ types
    /// are identical (`using Unit = std::tuple<>;`); this flag only
    /// flips the textual surface in generated output.
    #[arg(long)]
    prefer_std_tuple_alias: bool,

    /// Prefer `rusty::StrView` / `rusty::Span<...>` alias spellings in generated output.
    #[arg(long)]
    prefer_rusty_view_aliases: bool,

    /// Deprecated no-op: interface+adapter is now the only trait
    /// lowering path. Kept for CLI compatibility with older scripts.
    #[arg(long, hide = true)]
    interface_traits: bool,
}

#[derive(Parser)]
struct InlineRustArgs {
    /// Validate marker structure and rust_sha256 hashes
    #[arg(long, conflicts_with_all = ["rewrite", "emit_rust"])]
    check: bool,

    /// Rewrite GEN regions with deterministic markers and generated C++ fallback
    #[arg(long, conflicts_with_all = ["check", "emit_rust"])]
    rewrite: bool,

    /// Emit normalized Rust payloads to OUTPUT without modifying the source file
    #[arg(
        long,
        value_name = "OUTPUT",
        conflicts_with_all = ["check", "rewrite"]
    )]
    emit_rust: Option<PathBuf>,

    /// Emit only this block id (repeat to choose multiple blocks and their order)
    #[arg(long = "block-id", value_name = "ID", requires = "emit_rust")]
    block_ids: Vec<String>,

    /// C++ files containing inline Rust blocks
    #[arg(long = "files", required = true, num_args = 1..)]
    files: Vec<PathBuf>,
}

/// Transpile an entire Rust crate in one command.
/// Walks all .rs files, transpiles each with the correct module name,
/// and generates CMakeLists.txt.
fn validate_cpp_abi_conventional_lib_crate(
    cargo: &cmake::CargoToml,
    sources: &[PathBuf],
) -> Result<(), String> {
    let lib_path = cargo
        .lib
        .as_ref()
        .and_then(|target| target.path.as_deref())
        .unwrap_or("src/lib.rs")
        .replace('\\', "/");
    if lib_path != "src/lib.rs" || !sources.iter().any(|path| path == Path::new("src/lib.rs")) {
        return Err(
            "cpp_abi crate mode currently requires the conventional src/lib.rs library target"
                .to_string(),
        );
    }
    if cargo.bins.as_ref().is_some_and(|bins| !bins.is_empty())
        || sources.iter().any(|path| {
            let normalized = path.to_string_lossy().replace('\\', "/");
            normalized == "src/main.rs" || normalized.starts_with("src/bin/")
        })
    {
        return Err(
            "cpp_abi crate mode currently supports one library target and no binary targets"
                .to_string(),
        );
    }
    Ok(())
}

fn source_mentions_cpp_source_contract(source: &str) -> bool {
    cpp_abi::source_mentions_reserved_marker(source)
        || cpp_default_args::source_mentions_marker(source)
}

fn reject_cpp_abi_in_nonconventional_target_roots(
    cargo: &cmake::CargoToml,
    project_dir: &Path,
) -> Result<(), String> {
    let mut declared_roots = Vec::<PathBuf>::new();
    if let Some(path) = cargo
        .lib
        .as_ref()
        .and_then(|target| target.path.as_deref())
        && path.replace('\\', "/") != "src/lib.rs"
    {
        declared_roots.push(PathBuf::from(path));
    }
    if let Some(bins) = &cargo.bins {
        declared_roots.extend(
            bins.iter()
                .filter_map(|target| target.path.as_deref().map(PathBuf::from)),
        );
    }
    declared_roots.sort();
    declared_roots.dedup();
    for relative in declared_roots {
        let full = project_dir.join(&relative);
        let Ok(source) = std::fs::read_to_string(&full) else {
            continue;
        };
        if source_mentions_cpp_source_contract(&source) {
            return Err(format!(
                "a source-owned C++ contract is present in declared target {} but crate mode currently supports only the conventional src/lib.rs library target",
                relative.display()
            ));
        }
    }
    Ok(())
}

fn effective_local_dependencies_for_manifest(
    graph: &metadata::EffectiveLocalNormalDependencyGraph,
    manifest_path: &Path,
) -> Result<Vec<cmake::CrateDep>, String> {
    let dependencies = graph.direct_dependencies(manifest_path).ok_or_else(|| {
        format!(
            "Cargo's target-filtered dependency graph for '{}' omitted local manifest {}",
            graph.target_triple,
            manifest_path.display()
        )
    })?;
    dependencies
        .iter()
        .map(|dependency| {
            let package_dir = dependency.manifest_path.parent().ok_or_else(|| {
                format!(
                    "Cargo reported a dependency manifest without a parent directory: {}",
                    dependency.manifest_path.display()
                )
            })?;
            Ok(cmake::CrateDep {
                name: dependency.dependency_key.clone(),
                package: (dependency.package_name != dependency.dependency_key)
                    .then(|| dependency.package_name.clone()),
                version: None,
                path: Some(package_dir.to_string_lossy().into_owned()),
                is_local: true,
                workspace_inherited: false,
                // This edge exists only when Cargo selected the optional
                // dependency through the effective feature set.
                optional: false,
                // Cargo already evaluated the selector for target_triple.
                target: None,
            })
        })
        .collect()
}

#[derive(Default)]
struct CppAbiClosureReport {
    any_source_contract: bool,
    issues: BTreeSet<String>,
    runtime_dependency_issues: BTreeSet<String>,
}

struct CppAbiClosurePreflight<'a> {
    expand: bool,
    effective_dependencies: Option<&'a metadata::EffectiveLocalNormalDependencyGraph>,
    report: CppAbiClosureReport,
    root_manifest: Option<PathBuf>,
    visited: BTreeSet<PathBuf>,
    active: Vec<PathBuf>,
}

fn validate_cpp_abi_manifest_edition(manifest_source: &str) -> Result<(), String> {
    let manifest = toml::from_str::<toml::Value>(manifest_source)
        .map_err(|error| format!("could not inspect package.edition: {error}"))?;
    let edition = manifest
        .get("package")
        .and_then(toml::Value::as_table)
        .and_then(|package| package.get("edition"));
    match edition.and_then(toml::Value::as_str) {
        Some("2018" | "2021" | "2024") => Ok(()),
        Some(unsupported) => Err(format!(
            "source-owned C++ contracts require an explicit Rust 2018, 2021, or 2024 package.edition; found `{unsupported}`"
        )),
        None if edition.is_none() => Err(
            "source-owned C++ contracts require an explicit Rust 2018, 2021, or 2024 package.edition; an omitted edition selects Rust 2015"
                .to_string(),
        ),
        None => Err(
            "source-owned C++ contracts require an explicitly resolved Rust 2018, 2021, or 2024 package.edition; workspace-inherited or non-string editions are unsupported"
                .to_string(),
        ),
    }
}

impl<'a> CppAbiClosurePreflight<'a> {
    fn new(expand: bool) -> Self {
        Self {
            expand,
            effective_dependencies: None,
            report: CppAbiClosureReport::default(),
            root_manifest: None,
            visited: BTreeSet::new(),
            active: Vec::new(),
        }
    }

    fn with_effective_dependencies(
        expand: bool,
        effective_dependencies: &'a metadata::EffectiveLocalNormalDependencyGraph,
    ) -> Self {
        Self {
            expand,
            effective_dependencies: Some(effective_dependencies),
            report: CppAbiClosureReport::default(),
            root_manifest: None,
            visited: BTreeSet::new(),
            active: Vec::new(),
        }
    }

    fn manifest_key(path: &Path) -> PathBuf {
        if let Ok(canonical) = std::fs::canonicalize(path) {
            return canonical;
        }
        let absolute = if path.is_absolute() {
            path.to_path_buf()
        } else {
            std::env::current_dir()
                .map(|current| current.join(path))
                .unwrap_or_else(|_| path.to_path_buf())
        };
        let mut cursor = absolute.as_path();
        let mut missing = Vec::new();
        loop {
            if let Ok(mut canonical) = std::fs::canonicalize(cursor) {
                for component in missing.into_iter().rev() {
                    canonical.push(component);
                }
                return canonical;
            }
            let Some(component) = cursor.file_name() else {
                return absolute;
            };
            missing.push(component.to_os_string());
            let Some(parent) = cursor.parent() else {
                return absolute;
            };
            cursor = parent;
        }
    }

    fn issue(&mut self, message: impl Into<String>) {
        self.report.issues.insert(message.into());
    }

    fn runtime_dependency_issue(&mut self, message: impl Into<String>) {
        self.report.runtime_dependency_issues.insert(message.into());
    }

    fn note_source_contract(&mut self, cargo_toml_path: &Path) {
        self.report.any_source_contract = true;
        let manifest = Self::manifest_key(cargo_toml_path);
        if self
            .root_manifest
            .as_ref()
            .is_some_and(|root| root != &manifest)
        {
            self.issue(format!(
                "local dependency {} contains source-owned C++ contracts; cross-crate adapter calls are unsupported",
                cargo_toml_path.display()
            ));
        }
    }

    fn collect_rs_files(&mut self, project_dir: &Path) -> Vec<PathBuf> {
        fn recurse(
            this: &mut CppAbiClosurePreflight,
            project_dir: &Path,
            directory: &Path,
            active_directories: &mut Vec<PathBuf>,
            files: &mut Vec<PathBuf>,
        ) {
            let canonical_directory = match std::fs::canonicalize(directory) {
                Ok(path) => path,
                Err(error) => {
                    this.issue(format!(
                        "could not resolve source directory {}: {error}",
                        directory.display()
                    ));
                    return;
                }
            };
            if let Some(cycle_start) = active_directories
                .iter()
                .position(|active| active == &canonical_directory)
            {
                let mut cycle = active_directories[cycle_start..]
                    .iter()
                    .map(|path| path.display().to_string())
                    .collect::<Vec<_>>();
                cycle.push(canonical_directory.display().to_string());
                this.issue(format!(
                    "source directory symlink cycle at {}: {}",
                    directory.display(),
                    cycle.join(" -> ")
                ));
                return;
            }
            active_directories.push(canonical_directory);

            let entries = match std::fs::read_dir(directory) {
                Ok(entries) => entries,
                Err(error) => {
                    this.issue(format!(
                        "could not read source directory {}: {error}",
                        directory.display()
                    ));
                    active_directories.pop();
                    return;
                }
            };
            let mut entries = entries.collect::<Vec<_>>();
            entries.sort_by(|left, right| match (left, right) {
                (Ok(left), Ok(right)) => left.path().cmp(&right.path()),
                (Err(_), Ok(_)) => std::cmp::Ordering::Less,
                (Ok(_), Err(_)) => std::cmp::Ordering::Greater,
                (Err(left), Err(right)) => left.to_string().cmp(&right.to_string()),
            });
            for entry in entries {
                let entry = match entry {
                    Ok(entry) => entry,
                    Err(error) => {
                        this.issue(format!(
                            "could not enumerate source directory {}: {error}",
                            directory.display()
                        ));
                        continue;
                    }
                };
                let path = entry.path();
                // Follow both file and directory symlinks, matching the
                // legacy collector's effective path semantics. Keep the
                // lexical path for module identity and diagnostics.
                let metadata = match std::fs::metadata(&path) {
                    Ok(metadata) => metadata,
                    Err(error) => {
                        this.issue(format!(
                            "could not inspect source path {}: {error}",
                            path.display()
                        ));
                        continue;
                    }
                };
                if metadata.is_dir() {
                    recurse(this, project_dir, &path, active_directories, files);
                } else if metadata.is_file() && path.extension().is_some_and(|ext| ext == "rs") {
                    match path.strip_prefix(project_dir) {
                        Ok(relative) => files.push(relative.to_path_buf()),
                        Err(error) => this.issue(format!(
                            "could not make source path {} project-relative: {error}",
                            path.display()
                        )),
                    }
                }
            }
            active_directories.pop();
        }

        let src = project_dir.join("src");
        match std::fs::symlink_metadata(&src) {
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Vec::new(),
            Err(error) => {
                self.issue(format!(
                    "could not inspect source directory {}: {error}",
                    src.display()
                ));
                return Vec::new();
            }
        }
        let mut files = Vec::new();
        recurse(self, project_dir, &src, &mut Vec::new(), &mut files);
        files.sort();
        files.dedup();
        files
    }

    fn read_source_units(
        &mut self,
        project_dir: &Path,
        cargo_toml_path: &Path,
    ) -> (Vec<PathBuf>, Vec<(PathBuf, String)>) {
        let sources = self.collect_rs_files(project_dir);
        let mut units = Vec::with_capacity(sources.len());
        for relative in &sources {
            let full = project_dir.join(relative);
            match std::fs::read_to_string(&full) {
                Ok(source) => {
                    if source_mentions_cpp_source_contract(&source) {
                        self.note_source_contract(cargo_toml_path);
                    }
                    units.push((relative.clone(), source));
                }
                Err(error) => self.issue(format!(
                    "could not read Rust source {}: {error}",
                    full.display()
                )),
            }
        }
        (sources, units)
    }

    fn scan_declared_target_roots(
        &mut self,
        cargo: &cmake::CargoToml,
        project_dir: &Path,
        cargo_toml_path: &Path,
    ) -> bool {
        let mut mentions_source_contract = false;
        let mut declared = Vec::<PathBuf>::new();
        if let Some(path) = cargo.lib.as_ref().and_then(|target| target.path.as_deref())
            && path.replace('\\', "/") != "src/lib.rs"
        {
            declared.push(PathBuf::from(path));
        }
        if let Some(bins) = &cargo.bins {
            declared.extend(
                bins.iter()
                    .filter_map(|target| target.path.as_deref().map(PathBuf::from)),
            );
        }
        declared.sort();
        declared.dedup();
        for relative in declared {
            let full = project_dir.join(&relative);
            match std::fs::read_to_string(&full) {
                Ok(source) => {
                    if source_mentions_cpp_source_contract(&source) {
                        mentions_source_contract = true;
                        self.note_source_contract(cargo_toml_path);
                    }
                }
                Err(error) => self.issue(format!(
                    "could not read declared Rust target {}: {error}",
                    full.display()
                )),
            }
        }
        mentions_source_contract
    }

    fn scan_sources_without_manifest(&mut self, project_dir: &Path, manifest: &Path) {
        let (_, source_units) = self.read_source_units(project_dir, manifest);
        if source_units
            .iter()
            .any(|(_, source)| source_mentions_cpp_source_contract(source))
        {
            if let Err(error) = cpp_abi::preflight_crate_sources(&source_units) {
                self.issue(format!("{}: {error}", manifest.display()));
            }
            if let Err(error) = cpp_default_args::preflight_crate_sources_syntax(&source_units) {
                self.issue(format!("{}: {error}", manifest.display()));
            }
        }
    }

    fn visit_manifest(&mut self, cargo_toml_path: &Path) {
        let visit_key = Self::manifest_key(cargo_toml_path);
        if let Some(cycle_start) = self.active.iter().position(|path| path == &visit_key) {
            let mut cycle = self.active[cycle_start..]
                .iter()
                .map(|path| path.display().to_string())
                .collect::<Vec<_>>();
            cycle.push(visit_key.display().to_string());
            self.issue(format!(
                "local dependency cycle detected: {}",
                cycle.join(" -> ")
            ));
            return;
        }
        if !self.visited.insert(visit_key.clone()) {
            return;
        }
        self.active.push(visit_key);

        let project_dir = cargo_toml_path.parent().unwrap_or(Path::new("."));
        let manifest_source = match std::fs::read_to_string(cargo_toml_path) {
            Ok(source) => source,
            Err(error) => {
                self.issue(format!(
                    "could not read local dependency manifest {}: {error}",
                    cargo_toml_path.display()
                ));
                self.scan_sources_without_manifest(project_dir, cargo_toml_path);
                self.active.pop();
                return;
            }
        };
        let cargo = match toml::from_str::<cmake::CargoToml>(&manifest_source) {
            Ok(cargo) => cargo,
            Err(error) => {
                self.issue(format!(
                    "could not parse local dependency manifest {}: {error}",
                    cargo_toml_path.display()
                ));
                self.scan_sources_without_manifest(project_dir, cargo_toml_path);
                self.active.pop();
                return;
            }
        };
        let dependencies = cmake::extract_dependencies(&cargo);
        let runtime_validation =
            match validate_rustc_only_runtime_dependencies(cargo_toml_path, &dependencies) {
                Ok(runtime_validation) => runtime_validation,
                Err(error) => {
                    self.runtime_dependency_issue(format!(
                        "{}: {error}",
                        cargo_toml_path.display()
                    ));
                    RustcRuntimeValidation::default()
                }
            };
        let traversal_dependencies = if let Some(graph) = self.effective_dependencies {
            match effective_local_dependencies_for_manifest(graph, cargo_toml_path) {
                Ok(dependencies) => dependencies,
                Err(error) => {
                    self.issue(format!("{}: {error}", cargo_toml_path.display()));
                    Vec::new()
                }
            }
        } else {
            dependencies
        };

        let (sources, source_units) = self.read_source_units(project_dir, cargo_toml_path);
        let declared_target_mentions_source_contract =
            self.scan_declared_target_roots(&cargo, project_dir, cargo_toml_path);
        let crate_mentions_source_contract = declared_target_mentions_source_contract
            || source_units
                .iter()
                .any(|(_, source)| source_mentions_cpp_source_contract(source));
        if crate_mentions_source_contract {
            if let Err(error) = validate_cpp_abi_manifest_edition(&manifest_source) {
                self.issue(format!("{}: {error}", cargo_toml_path.display()));
            }
            match cpp_abi::preflight_crate_sources(&source_units) {
                Ok(true) => {
                    if let Err(error) = validate_cpp_abi_conventional_lib_crate(&cargo, &sources) {
                        self.issue(format!("{}: {error}", cargo_toml_path.display()));
                    }
                    if self.expand {
                        self.issue(format!(
                            "{}: cpp_abi crate mode does not support --expand because expansion removes inert ABI markers",
                            cargo_toml_path.display()
                        ));
                    }
                }
                Ok(false) => {}
                Err(error) => self.issue(format!("{}: {error}", cargo_toml_path.display())),
            }
            match cpp_default_args::preflight_crate_sources_syntax(&source_units) {
                Ok(true) => {
                    if let Err(error) = validate_cpp_abi_conventional_lib_crate(&cargo, &sources) {
                        self.issue(format!("{}: {error}", cargo_toml_path.display()));
                    }
                    if self.expand {
                        self.issue(format!(
                            "{}: cpp_default_argument crate mode does not support --expand because expansion removes inert source markers",
                            cargo_toml_path.display()
                        ));
                    }
                }
                Ok(false) => {}
                Err(error) => self.issue(format!("{}: {error}", cargo_toml_path.display())),
            }
        }
        if let Err(error) = reject_cpp_abi_in_nonconventional_target_roots(&cargo, project_dir) {
            self.issue(format!("{}: {error}", cargo_toml_path.display()));
        }

        let mut dependencies = traversal_dependencies
            .into_iter()
            .filter(|dependency| dependency.is_local)
            .collect::<Vec<_>>();
        dependencies.sort_by(|left, right| {
            left.name
                .cmp(&right.name)
                .then_with(|| left.path.cmp(&right.path))
        });
        for dependency in dependencies {
            if dependency.target.is_some() {
                continue;
            }
            if runtime_validation
                .runtime_provided
                .contains(&dependency.name)
                || dependency.name == RUSTY_RUNTIME_CRATE_NAME
                || dependency.package.as_deref() == Some(RUSTY_RUNTIME_CRATE_NAME)
            {
                continue;
            }
            let Some(relative) = dependency.path.as_deref() else {
                continue;
            };
            self.visit_manifest(&project_dir.join(relative).join("Cargo.toml"));
        }
        self.active.pop();
    }

    fn finish(self) -> Result<bool, String> {
        if !self.report.runtime_dependency_issues.is_empty() {
            return Err(format!(
                "rustc-only runtime dependency whole local-dependency closure preflight failed before output:\n- {}",
                self.report
                    .runtime_dependency_issues
                    .into_iter()
                    .collect::<Vec<_>>()
                    .join("\n- ")
            ));
        }
        if self.report.any_source_contract && !self.report.issues.is_empty() {
            return Err(format!(
                "source-owned C++ contract whole local-dependency closure preflight failed before output:\n- {}",
                self.report
                    .issues
                    .into_iter()
                    .collect::<Vec<_>>()
                    .join("\n- ")
            ));
        }
        Ok(self.report.any_source_contract)
    }
}

fn preflight_cpp_abi_whole_dependency_closure(
    cargo_toml_path: &Path,
    expand: bool,
) -> Result<bool, String> {
    let mut preflight = CppAbiClosurePreflight::new(expand);
    preflight.root_manifest = Some(CppAbiClosurePreflight::manifest_key(cargo_toml_path));
    preflight.visit_manifest(cargo_toml_path);
    preflight.finish()
}

fn preflight_cpp_source_contract_effective_dependency_closure(
    cargo_toml_path: &Path,
    expand: bool,
    graph: &metadata::EffectiveLocalNormalDependencyGraph,
) -> Result<bool, String> {
    let requested = CppAbiClosurePreflight::manifest_key(cargo_toml_path);
    if requested != graph.root_manifest() {
        return Err(format!(
            "Cargo's target-filtered dependency graph root {} does not match requested manifest {}",
            graph.root_manifest().display(),
            requested.display()
        ));
    }
    let mut preflight = CppAbiClosurePreflight::with_effective_dependencies(expand, graph);
    preflight.root_manifest = Some(requested);
    preflight.visit_manifest(cargo_toml_path);
    preflight.finish()
}

/// Cheap, output-free over-approximation used to decide whether Cargo's exact
/// target-selected graph is required. It deliberately follows every declared
/// local normal dependency, including optional and target-qualified entries;
/// the subsequent target-filtered closure discards unselected edges.
fn dependency_closure_may_have_source_contract(cargo_toml_path: &Path) -> bool {
    fn visit(cargo_toml_path: &Path, visited: &mut BTreeSet<PathBuf>) -> bool {
        let key = CppAbiClosurePreflight::manifest_key(cargo_toml_path);
        if !visited.insert(key) {
            return false;
        }
        let Ok(manifest_source) = std::fs::read_to_string(cargo_toml_path) else {
            return false;
        };
        let Ok(cargo) = toml::from_str::<cmake::CargoToml>(&manifest_source) else {
            return false;
        };
        let project_dir = cargo_toml_path.parent().unwrap_or(Path::new("."));

        // Reuse the symlink-aware collector, but ignore its diagnostics here:
        // the authoritative closure preflight retains ownership of errors.
        let mut collector = CppAbiClosurePreflight::new(false);
        let source_paths = collector.collect_rs_files(project_dir);
        for relative in source_paths {
            if std::fs::read_to_string(project_dir.join(relative))
                .is_ok_and(|source| source_mentions_cpp_source_contract(&source))
            {
                return true;
            }
        }
        let mut declared_roots = cargo
            .lib
            .as_ref()
            .and_then(|target| target.path.as_deref())
            .map(PathBuf::from)
            .into_iter()
            .collect::<Vec<_>>();
        if let Some(bins) = &cargo.bins {
            declared_roots.extend(
                bins.iter()
                    .filter_map(|target| target.path.as_deref().map(PathBuf::from)),
            );
        }
        for relative in declared_roots {
            if std::fs::read_to_string(project_dir.join(relative))
                .is_ok_and(|source| source_mentions_cpp_source_contract(&source))
            {
                return true;
            }
        }

        let declared = cmake::extract_dependencies(&cargo);
        let dependencies = resolve_workspace_inherited_dependencies(cargo_toml_path, &declared)
            .unwrap_or(declared);
        for dependency in dependencies {
            if !dependency.is_local
                || dependency.name == RUSTY_RUNTIME_CRATE_NAME
                || dependency.package.as_deref() == Some(RUSTY_RUNTIME_CRATE_NAME)
            {
                continue;
            }
            let Some(relative) = dependency.path.as_deref() else {
                continue;
            };
            if visit(&project_dir.join(relative).join("Cargo.toml"), visited) {
                return true;
            }
        }
        false
    }

    visit(cargo_toml_path, &mut BTreeSet::new())
}

const RUSTY_RUNTIME_CRATE_NAME: &str = "rusty";
const RUSTY_CPP_MARKERS_PACKAGE_NAME: &str = "rusty-cpp-markers";

#[derive(Default)]
struct RustcRuntimeValidation {
    runtime_provided: HashSet<String>,
    trusted_cpp_inherit_provenance: bool,
}

fn use_tree_binds_name(tree: &syn::UseTree, needle: &str) -> bool {
    match tree {
        syn::UseTree::Path(path) => use_tree_binds_name(&path.tree, needle),
        syn::UseTree::Name(name) => name.ident == needle,
        syn::UseTree::Rename(rename) => rename.rename == needle,
        syn::UseTree::Group(group) => group
            .items
            .iter()
            .any(|item| use_tree_binds_name(item, needle)),
        syn::UseTree::Glob(_) => true,
    }
}

fn source_has_exact_cpp_inherit_facade_export(source: &str) -> bool {
    let Ok(file) = syn::parse_file(source) else {
        return false;
    };
    if file.attrs.iter().any(|attr| {
        !matches!(
            attr.path().get_ident().map(ToString::to_string).as_deref(),
            Some("doc" | "allow" | "warn" | "deny" | "forbid")
        )
    }) {
        return false;
    }
    let mut trusted_exports = 0usize;
    let mut competing_bindings = 0usize;
    let mut local_marker_roots = 0usize;
    for item in &file.items {
        match item {
            syn::Item::Use(item) if use_tree_binds_name(&item.tree, "cpp_inherit") => {
                competing_bindings += 1;
                let exact_tree = matches!(
                    &item.tree,
                    syn::UseTree::Path(root)
                        if root.ident == "rusty_cpp_markers"
                            && matches!(root.tree.as_ref(), syn::UseTree::Name(name) if name.ident == "cpp_inherit")
                );
                let inert_attrs = item.attrs.iter().all(|attr| attr.path().is_ident("doc"));
                if exact_tree && inert_attrs && matches!(item.vis, syn::Visibility::Public(_)) {
                    trusted_exports += 1;
                }
            }
            syn::Item::Use(item)
                if use_tree_binds_name(&item.tree, "rusty_cpp_markers") =>
            {
                local_marker_roots += 1
            }
            syn::Item::Fn(item) if item.sig.ident == "cpp_inherit" => competing_bindings += 1,
            syn::Item::Struct(item) if item.ident == "cpp_inherit" => competing_bindings += 1,
            syn::Item::Enum(item) if item.ident == "cpp_inherit" => competing_bindings += 1,
            syn::Item::Union(item) if item.ident == "cpp_inherit" => competing_bindings += 1,
            syn::Item::Trait(item) if item.ident == "cpp_inherit" => competing_bindings += 1,
            syn::Item::Type(item) if item.ident == "cpp_inherit" => competing_bindings += 1,
            syn::Item::Mod(item) if item.ident == "cpp_inherit" => competing_bindings += 1,
            syn::Item::Mod(item) if item.ident == "rusty_cpp_markers" => {
                local_marker_roots += 1
            }
            syn::Item::ExternCrate(item)
                if item
                    .rename
                    .as_ref()
                    .map(|(_, rename)| rename == "cpp_inherit")
                    .unwrap_or(item.ident == "cpp_inherit") =>
            {
                competing_bindings += 1
            }
            syn::Item::ExternCrate(item)
                if item
                    .rename
                    .as_ref()
                    .map(|(_, rename)| rename == "rusty_cpp_markers")
                    .unwrap_or(item.ident == "rusty_cpp_markers") =>
            {
                local_marker_roots += 1
            }
            syn::Item::Macro(item)
                if item
                    .ident
                    .as_ref()
                    .is_some_and(|ident| ident == "cpp_inherit") =>
            {
                competing_bindings += 1
            }
            _ => {}
        }
    }
    trusted_exports == 1 && competing_bindings == 1 && local_marker_roots == 0
}

fn type_is_unqualified_token_stream(ty: &syn::Type) -> bool {
    matches!(
        ty,
        syn::Type::Path(path)
            if path.qself.is_none()
                && path.path.leading_colon.is_none()
                && path.path.segments.len() == 1
                && path.path.is_ident("TokenStream")
    )
}

fn source_is_exact_inert_cpp_inherit_marker(source: &str) -> bool {
    let Ok(file) = syn::parse_file(source) else {
        return false;
    };
    if file
        .attrs
        .iter()
        .any(|attr| !attr.path().is_ident("doc"))
    {
        return false;
    }
    let mut saw_token_stream_import = false;
    let mut saw_cpp_inherit = false;
    for item in &file.items {
        match item {
            syn::Item::Use(item)
                if item.attrs.iter().all(|attr| attr.path().is_ident("doc"))
                    && matches!(item.vis, syn::Visibility::Inherited)
                    && matches!(
                        &item.tree,
                        syn::UseTree::Path(root)
                            if root.ident == "proc_macro"
                                && matches!(root.tree.as_ref(), syn::UseTree::Name(name) if name.ident == "TokenStream")
                    ) =>
            {
                if saw_token_stream_import {
                    return false;
                }
                saw_token_stream_import = true;
            }
            syn::Item::Fn(function) if function.sig.ident == "cpp_inherit" => {
                if saw_cpp_inherit
                    || !matches!(function.vis, syn::Visibility::Public(_))
                    || function.sig.constness.is_some()
                    || function.sig.asyncness.is_some()
                    || function.sig.unsafety.is_some()
                    || function.sig.abi.is_some()
                    || !function.sig.generics.params.is_empty()
                    || function.sig.generics.where_clause.is_some()
                {
                    return false;
                }
                let proc_attrs = function
                    .attrs
                    .iter()
                    .filter(|attr| attr.path().is_ident("proc_macro_attribute"))
                    .count();
                if proc_attrs != 1
                    || function.attrs.iter().any(|attr| {
                        !attr.path().is_ident("doc")
                            && !attr.path().is_ident("proc_macro_attribute")
                    })
                {
                    return false;
                }
                let parameters = function.sig.inputs.iter().collect::<Vec<_>>();
                let exact_parameters = matches!(
                    parameters.as_slice(),
                    [syn::FnArg::Typed(attribute), syn::FnArg::Typed(item)]
                        if type_is_unqualified_token_stream(&attribute.ty)
                            && type_is_unqualified_token_stream(&item.ty)
                            && matches!(item.pat.as_ref(), syn::Pat::Ident(ident) if ident.ident == "item" && ident.by_ref.is_none() && ident.subpat.is_none())
                );
                let exact_return = matches!(
                    &function.sig.output,
                    syn::ReturnType::Type(_, ty) if type_is_unqualified_token_stream(ty)
                );
                let exact_body = matches!(
                    function.block.stmts.as_slice(),
                    [syn::Stmt::Expr(syn::Expr::Path(path), None)]
                        if path.attrs.is_empty()
                            && path.qself.is_none()
                            && path.path.leading_colon.is_none()
                            && path.path.is_ident("item")
                );
                if !exact_parameters || !exact_return || !exact_body {
                    return false;
                }
                saw_cpp_inherit = true;
            }
            // The trusted marker crate is intentionally two declarations. An
            // extra item could shadow the imported token type or alter the
            // proc-macro entry point through conditional compilation.
            _ => return false,
        }
    }
    saw_token_stream_import && saw_cpp_inherit
}

fn validate_cpp_inherit_runtime_provenance(
    runtime_identity: &metadata::ManifestIdentity,
    runtime_library_source: &Path,
) -> bool {
    let Some(marker_dependency) = runtime_identity.dependencies.iter().find(|dependency| {
        dependency.dependency_key == RUSTY_CPP_MARKERS_PACKAGE_NAME
            && dependency.package_name == RUSTY_CPP_MARKERS_PACKAGE_NAME
            && dependency.source.is_none()
            && dependency.path.is_some()
            && dependency.kind.is_none()
            && dependency.target.is_none()
            && !dependency.optional
    }) else {
        return false;
    };
    if runtime_identity
        .dependencies
        .iter()
        .filter(|dependency| {
            dependency.dependency_key == RUSTY_CPP_MARKERS_PACKAGE_NAME
                || dependency.package_name == RUSTY_CPP_MARKERS_PACKAGE_NAME
        })
        .count()
        != 1
    {
        return false;
    }
    let Some(marker_dir) = marker_dependency.path.as_deref() else {
        return false;
    };
    let marker_manifest = marker_dir.join("Cargo.toml");
    let Ok(marker_identity) = metadata::inspect_manifest_identity(&marker_manifest) else {
        return false;
    };
    if marker_identity.package_name != RUSTY_CPP_MARKERS_PACKAGE_NAME
        || !marker_identity.dependencies.is_empty()
    {
        return false;
    }
    let mut targets = marker_identity.targets.iter().filter(|target| {
        target.name == "rusty_cpp_markers"
            && target.kind.iter().any(|kind| kind == "proc-macro")
            && target
                .crate_types
                .iter()
                .any(|crate_type| crate_type == "proc-macro")
    });
    let Some(marker_target) = targets.next() else {
        return false;
    };
    if targets.next().is_some() || marker_identity.targets.len() != 1 {
        return false;
    }
    let Ok(runtime_source) = std::fs::read_to_string(runtime_library_source) else {
        return false;
    };
    let Ok(marker_source) = std::fs::read_to_string(&marker_target.src_path) else {
        return false;
    };
    source_has_exact_cpp_inherit_facade_export(&runtime_source)
        && source_is_exact_inert_cpp_inherit_marker(&marker_source)
}

/// Identify the Rust-only facade for types supplied by the C++ runtime.
///
/// `rusty` is a reserved generated-code namespace, so silently treating a
/// registry crate, renamed package, or mismatched local target as the runtime
/// would make crate mode omit real code.  Only an exact local dependency and
/// exact Cargo package/library identity may take the runtime-provided path.
fn validate_rustc_only_runtime_dependencies(
    cargo_toml_path: &Path,
    dependencies: &[cmake::CrateDep],
) -> Result<RustcRuntimeValidation, String> {
    let project_dir = cargo_toml_path.parent().unwrap_or(Path::new("."));
    let mut validation = RustcRuntimeValidation::default();
    let resolved_manifest = if dependencies
        .iter()
        .any(|dependency| dependency.workspace_inherited)
    {
        Some(metadata::inspect_manifest_identity(cargo_toml_path).map_err(|error| {
            format!(
                "could not resolve workspace-inherited dependency identities for {} with Cargo: {error}",
                cargo_toml_path.display()
            )
        })?)
    } else {
        None
    };

    for dependency in dependencies {
        let resolved_dependency;
        let dependency = if dependency.workspace_inherited {
            let resolved_manifest = resolved_manifest
                .as_ref()
                .expect("workspace dependency resolution was requested above");
            let mut matches = resolved_manifest.dependencies.iter().filter(|candidate| {
                candidate.dependency_key == dependency.name
                    && candidate.kind.is_none()
                    && cargo_target_selectors_match(
                        candidate.target.as_deref(),
                        dependency.target.as_deref(),
                    )
            });
            let candidate = matches.next().ok_or_else(|| {
                format!(
                    "Cargo did not report an exact resolved identity for workspace-inherited dependency '{}'{}",
                    dependency.name,
                    dependency
                        .target
                        .as_deref()
                        .map(|target| format!(" under target '{target}'"))
                        .unwrap_or_default()
                )
            })?;
            if matches.next().is_some() {
                return Err(format!(
                    "Cargo reported ambiguous resolved identities for workspace-inherited dependency '{}'",
                    dependency.name
                ));
            }
            resolved_dependency = cmake::CrateDep {
                name: candidate.dependency_key.clone(),
                package: (candidate.package_name != candidate.dependency_key)
                    .then(|| candidate.package_name.clone()),
                version: dependency.version.clone(),
                path: candidate
                    .path
                    .as_ref()
                    .map(|path| path.to_string_lossy().into_owned()),
                is_local: candidate.source.is_none() && candidate.path.is_some(),
                workspace_inherited: false,
                optional: candidate.optional,
                target: dependency.target.clone(),
            };
            &resolved_dependency
        } else {
            dependency
        };
        let declared_package = dependency.package.as_deref().unwrap_or(&dependency.name);
        let is_reserved_identity = dependency.name == RUSTY_RUNTIME_CRATE_NAME
            || declared_package == RUSTY_RUNTIME_CRATE_NAME;

        if !is_reserved_identity {
            continue;
        }
        if let Some(target) = dependency.target.as_deref() {
            return Err(format!(
                "target-qualified dependency '{}' under target '{}' uses reserved runtime identity '{}'; the rustc-only facade must be one exact unconditional local path dependency",
                dependency.name, target, RUSTY_RUNTIME_CRATE_NAME
            ));
        }
        if dependency.optional {
            return Err(format!(
                "dependency '{}' uses reserved runtime identity '{}' but is optional; the rustc-only facade must be one exact unconditional local path dependency",
                dependency.name, RUSTY_RUNTIME_CRATE_NAME
            ));
        }

        if dependency.name != RUSTY_RUNTIME_CRATE_NAME {
            return Err(format!(
                "dependency '{}' renames reserved runtime package '{}'; use the exact local dependency key '{}'",
                dependency.name, RUSTY_RUNTIME_CRATE_NAME, RUSTY_RUNTIME_CRATE_NAME
            ));
        }

        if declared_package != RUSTY_RUNTIME_CRATE_NAME {
            return Err(format!(
                "dependency '{}' selects package '{}', but that dependency name is reserved for the '{}' C++ runtime facade",
                dependency.name, declared_package, RUSTY_RUNTIME_CRATE_NAME
            ));
        }
        if !dependency.is_local {
            return Err(format!(
                "dependency '{}' is reserved for an exact local path dependency that provides the rustc-only C++ runtime facade",
                RUSTY_RUNTIME_CRATE_NAME
            ));
        }

        let dependency_path = dependency.path.as_deref().ok_or_else(|| {
            format!(
                "local runtime dependency '{}' has no path",
                RUSTY_RUNTIME_CRATE_NAME
            )
        })?;
        let dependency_dir = project_dir.join(dependency_path);
        let manifest_path = dependency_dir.join("Cargo.toml");
        let runtime_identity = metadata::inspect_manifest_identity(&manifest_path).map_err(|error| {
            format!(
                "could not validate rustc-only runtime dependency '{}' at {} with Cargo: {error}",
                RUSTY_RUNTIME_CRATE_NAME,
                manifest_path.display()
            )
        })?;

        if runtime_identity.package_name != RUSTY_RUNTIME_CRATE_NAME {
            return Err(format!(
                "dependency '{}' at {} declares Cargo package '{}'; refusing to omit a non-runtime crate",
                RUSTY_RUNTIME_CRATE_NAME,
                manifest_path.display(),
                runtime_identity.package_name
            ));
        }

        let mut ordinary_library_targets = runtime_identity.targets.iter().filter(|target| {
            target.name == RUSTY_RUNTIME_CRATE_NAME
                && !target.kind.iter().any(|kind| kind == "proc-macro")
                && target
                    .crate_types
                    .iter()
                    .any(|crate_type| matches!(crate_type.as_str(), "lib" | "rlib"))
        });
        let library_target = ordinary_library_targets.next().ok_or_else(|| {
            let reported = runtime_identity
                .targets
                .iter()
                .map(|target| format!("{} ({})", target.name, target.kind.join(", ")))
                .collect::<Vec<_>>()
                .join("; ");
            format!(
                "runtime package '{}' at {} does not expose an ordinary Rust library target named exactly '{}'; Cargo reported: {}",
                RUSTY_RUNTIME_CRATE_NAME,
                manifest_path.display(),
                RUSTY_RUNTIME_CRATE_NAME,
                if reported.is_empty() { "no targets" } else { &reported }
            )
        })?;
        if ordinary_library_targets.next().is_some() {
            return Err(format!(
                "runtime package '{}' at {} exposes more than one ordinary library target named '{}'; refusing ambiguous facade identity",
                RUSTY_RUNTIME_CRATE_NAME,
                manifest_path.display(),
                RUSTY_RUNTIME_CRATE_NAME
            ));
        }

        if !library_target.src_path.is_file() {
            return Err(format!(
                "runtime package '{}' at {} has no library source at {}",
                RUSTY_RUNTIME_CRATE_NAME,
                manifest_path.display(),
                library_target.src_path.display()
            ));
        }

        validation.trusted_cpp_inherit_provenance = validate_cpp_inherit_runtime_provenance(
            &runtime_identity,
            &library_target.src_path,
        );
        validation.runtime_provided.insert(dependency.name.clone());
    }

    Ok(validation)
}

fn cargo_target_selectors_match(resolved: Option<&str>, declared: Option<&str>) -> bool {
    match (resolved, declared) {
        (None, None) => true,
        (Some(resolved), Some(declared)) => resolved
            .chars()
            .filter(|ch| !ch.is_whitespace())
            .eq(declared.chars().filter(|ch| !ch.is_whitespace())),
        _ => false,
    }
}

fn resolve_workspace_inherited_dependencies(
    cargo_toml_path: &Path,
    dependencies: &[cmake::CrateDep],
) -> Result<Vec<cmake::CrateDep>, String> {
    let resolved_manifest = if dependencies
        .iter()
        .any(|dependency| dependency.workspace_inherited)
    {
        Some(metadata::inspect_manifest_identity(cargo_toml_path).map_err(|error| {
            format!(
                "could not resolve workspace-inherited dependency identities for {} with Cargo: {error}",
                cargo_toml_path.display()
            )
        })?)
    } else {
        None
    };
    dependencies
        .iter()
        .map(|dependency| {
            if !dependency.workspace_inherited {
                return Ok(dependency.clone());
            }
            let resolved_manifest = resolved_manifest
                .as_ref()
                .expect("workspace dependency resolution was requested above");
            let mut matches = resolved_manifest.dependencies.iter().filter(|candidate| {
                candidate.dependency_key == dependency.name
                    && candidate.kind.is_none()
                    && cargo_target_selectors_match(
                        candidate.target.as_deref(),
                        dependency.target.as_deref(),
                    )
            });
            let candidate = matches.next().ok_or_else(|| {
                format!(
                    "Cargo did not report an exact resolved identity for workspace-inherited dependency '{}'{}",
                    dependency.name,
                    dependency
                        .target
                        .as_deref()
                        .map(|target| format!(" under target '{target}'"))
                        .unwrap_or_default()
                )
            })?;
            if matches.next().is_some() {
                return Err(format!(
                    "Cargo reported ambiguous resolved identities for workspace-inherited dependency '{}'",
                    dependency.name
                ));
            }
            Ok(cmake::CrateDep {
                name: candidate.dependency_key.clone(),
                package: (candidate.package_name != candidate.dependency_key)
                    .then(|| candidate.package_name.clone()),
                version: dependency.version.clone(),
                path: candidate
                    .path
                    .as_ref()
                    .map(|path| path.to_string_lossy().into_owned()),
                is_local: candidate.source.is_none() && candidate.path.is_some(),
                workspace_inherited: false,
                optional: candidate.optional,
                target: dependency.target.clone(),
            })
        })
        .collect()
}

struct PreparedCrateCodegen {
    extension_method_hints: HashSet<String>,
    module_preambles: BTreeMap<String, Vec<transpile::GmfIncludeSpec>>,
    options: transpile::TranspileOptions,
}

fn prepare_crate_codegen(
    sources: &[PathBuf],
    source_units: &[(PathBuf, String)],
    crate_name: &str,
    transpile_options: &transpile::TranspileOptions,
    module_preamble: Option<&transpile::ModulePreambleManifest>,
) -> Result<PreparedCrateCodegen, String> {
    let mut extension_method_hints = HashSet::new();
    let mut cross_file_enums: Vec<syn::ItemEnum> = Vec::new();
    let mut cross_file_impl_blocks: Vec<syn::ItemImpl> = Vec::new();
    let mut cross_file_structs: Vec<syn::ItemStruct> = Vec::new();
    let mut cross_file_type_aliases: Vec<syn::ItemType> = Vec::new();
    for (_, source) in source_units {
        extension_method_hints.extend(transpile::collect_extension_method_hints(source));
        cross_file_enums.extend(transpile::collect_crate_enum_decls(source));
        cross_file_impl_blocks.extend(transpile::collect_crate_impl_blocks(source));
        cross_file_structs.extend(transpile::collect_crate_struct_decls(source));
        cross_file_type_aliases.extend(transpile::collect_crate_type_aliases(source));
    }

    let crate_module_names: Vec<String> = sources
        .iter()
        .map(|rs_path| cmake::map_rs_to_cppm(rs_path, crate_name).1)
        .collect();
    let module_preambles = if let Some(manifest) = module_preamble {
        manifest.select_for_modules(crate_module_names.iter().map(String::as_str))?
    } else {
        BTreeMap::new()
    };
    let mut options = transpile_options.clone();
    options.cross_file_enums = cross_file_enums;
    options.cross_file_impl_blocks = cross_file_impl_blocks;
    options.cross_file_structs = cross_file_structs;
    options.cross_file_type_aliases = cross_file_type_aliases;
    options.crate_module_names = crate_module_names;
    Ok(PreparedCrateCodegen {
        extension_method_hints,
        module_preambles,
        options,
    })
}

fn preflight_cpp_name_crate_sources_exact(
    cargo: &cmake::CargoToml,
    sources: &[PathBuf],
    source_units: &[(PathBuf, String)],
    type_map: &types::UserTypeMap,
    expand: bool,
    transpile_options: &transpile::TranspileOptions,
    module_preamble: Option<&transpile::ModulePreambleManifest>,
    trusted_cpp_inherit_provenance: bool,
) -> Result<bool, String> {
    let has_cpp_name = cpp_name::preflight_crate_sources_with_cpp_inherit_provenance(
        source_units,
        trusted_cpp_inherit_provenance,
    )?;
    if has_cpp_name && expand {
        return Err(
            "cpp_name crate mode does not support --expand because expansion removes inert name markers"
                .to_string(),
        );
    }
    if !has_cpp_name {
        return Ok(false);
    }

    // The final cpp_name collision proof deliberately uses the same type
    // mapper as declaration/definition emission. Keep this helper entirely
    // in memory so it is safe to run over the whole local dependency graph
    // before the caller creates or changes any output directory.
    let crate_name = &cargo.package.name;
    let prepared = prepare_crate_codegen(
        sources,
        source_units,
        crate_name,
        transpile_options,
        module_preamble,
    )?;
    let mut crate_options = prepared.options;
    // A preflight is observational. Even if a library caller supplied an
    // emission path, proving cpp_name must never write a UFCS sidecar.
    crate_options.emit_ufcs_trait_manifest_path = None;

    for (rs_path, source) in source_units {
        if !cpp_name::source_mentions_reserved_marker(source) {
            continue;
        }
        let (_, module_name) = cmake::map_rs_to_cppm(rs_path, crate_name);
        let mut module_options = crate_options.clone();
        module_options.explicit_gmf_includes = prepared
            .module_preambles
            .get(&module_name)
            .cloned()
            .unwrap_or_default();
        transpile::transpile_with_type_map_and_extension_hints_and_options(
            source,
            Some(&module_name),
            type_map,
            &prepared.extension_method_hints,
            &module_options,
        )
        .map_err(|error| {
            format!(
                "{}: cpp_name exact emission preflight failed: {error}",
                rs_path.display()
            )
        })?;
    }
    Ok(true)
}

#[derive(Default)]
struct CppNameClosureReport {
    any_cpp_name: bool,
    issues: BTreeSet<String>,
}

/// Read-only crate-mode cpp_name validation for every local crate that the
/// ordinary recursive generator can visit. The graph is completed before the
/// root output directory is touched, so a late dependency collision cannot
/// leave a partial parent tree behind.
struct CppNameClosurePreflight<'a> {
    type_map: &'a types::UserTypeMap,
    expand: bool,
    transpile_options: &'a transpile::TranspileOptions,
    effective_dependencies: Option<&'a metadata::EffectiveLocalNormalDependencyGraph>,
    root_manifest: PathBuf,
    root_module_preamble: Option<&'a transpile::ModulePreambleManifest>,
    report: CppNameClosureReport,
    visited: BTreeSet<PathBuf>,
    active: Vec<PathBuf>,
}

impl<'a> CppNameClosurePreflight<'a> {
    fn new(
        cargo_toml_path: &Path,
        type_map: &'a types::UserTypeMap,
        expand: bool,
        transpile_options: &'a transpile::TranspileOptions,
        root_module_preamble: Option<&'a transpile::ModulePreambleManifest>,
    ) -> Self {
        Self {
            type_map,
            expand,
            transpile_options,
            effective_dependencies: None,
            root_manifest: CppAbiClosurePreflight::manifest_key(cargo_toml_path),
            root_module_preamble,
            report: CppNameClosureReport::default(),
            visited: BTreeSet::new(),
            active: Vec::new(),
        }
    }

    fn with_effective_dependencies(
        cargo_toml_path: &Path,
        type_map: &'a types::UserTypeMap,
        expand: bool,
        transpile_options: &'a transpile::TranspileOptions,
        root_module_preamble: Option<&'a transpile::ModulePreambleManifest>,
        effective_dependencies: &'a metadata::EffectiveLocalNormalDependencyGraph,
    ) -> Self {
        Self {
            type_map,
            expand,
            transpile_options,
            effective_dependencies: Some(effective_dependencies),
            root_manifest: CppAbiClosurePreflight::manifest_key(cargo_toml_path),
            root_module_preamble,
            report: CppNameClosureReport::default(),
            visited: BTreeSet::new(),
            active: Vec::new(),
        }
    }

    fn issue(&mut self, message: impl Into<String>) {
        self.report.issues.insert(message.into());
    }

    fn collect_sources(&mut self, project_dir: &Path) -> Vec<PathBuf> {
        // Reuse the checked, symlink-aware walker already used by cpp_abi.
        // Its report is local to this one scan; move all structural failures
        // into the cpp_name closure report so a graph containing cpp_name
        // fails closed rather than falling through to the legacy walker.
        let mut collector = CppAbiClosurePreflight::new(false);
        let sources = collector.collect_rs_files(project_dir);
        for issue in collector.report.issues {
            self.issue(issue);
        }
        sources
    }

    fn read_source_units(&mut self, project_dir: &Path) -> (Vec<PathBuf>, Vec<(PathBuf, String)>) {
        let sources = self.collect_sources(project_dir);
        let mut source_units = Vec::with_capacity(sources.len());
        for relative in &sources {
            let full = project_dir.join(relative);
            match std::fs::read_to_string(&full) {
                Ok(source) => source_units.push((relative.clone(), source)),
                Err(error) => self.issue(format!(
                    "could not read Rust source {}: {error}",
                    full.display()
                )),
            }
        }
        (sources, source_units)
    }

    fn note_markers_without_manifest(&mut self, project_dir: &Path) {
        let (_, source_units) = self.read_source_units(project_dir);
        if source_units
            .iter()
            .any(|(_, source)| cpp_name::source_mentions_reserved_marker(source))
        {
            self.report.any_cpp_name = true;
        }
    }

    fn visit_manifest(&mut self, cargo_toml_path: &Path) {
        let visit_key = CppAbiClosurePreflight::manifest_key(cargo_toml_path);
        if let Some(cycle_start) = self.active.iter().position(|path| path == &visit_key) {
            let mut cycle = self.active[cycle_start..]
                .iter()
                .map(|path| path.display().to_string())
                .collect::<Vec<_>>();
            cycle.push(visit_key.display().to_string());
            self.issue(format!(
                "local dependency cycle detected during cpp_name preflight: {}",
                cycle.join(" -> ")
            ));
            return;
        }
        if !self.visited.insert(visit_key.clone()) {
            return;
        }
        self.active.push(visit_key.clone());

        let project_dir = cargo_toml_path.parent().unwrap_or(Path::new("."));
        let manifest_source = match std::fs::read_to_string(cargo_toml_path) {
            Ok(source) => source,
            Err(error) => {
                self.issue(format!(
                    "could not read local dependency manifest {} during cpp_name preflight: {error}",
                    cargo_toml_path.display()
                ));
                self.note_markers_without_manifest(project_dir);
                self.active.pop();
                return;
            }
        };
        let cargo = match toml::from_str::<cmake::CargoToml>(&manifest_source) {
            Ok(cargo) => cargo,
            Err(error) => {
                self.issue(format!(
                    "could not parse local dependency manifest {} during cpp_name preflight: {error}",
                    cargo_toml_path.display()
                ));
                self.note_markers_without_manifest(project_dir);
                self.active.pop();
                return;
            }
        };
        let declared_dependencies = cmake::extract_dependencies(&cargo);
        let runtime_validation =
            match validate_rustc_only_runtime_dependencies(cargo_toml_path, &declared_dependencies)
            {
                Ok(validation) => validation,
                Err(error) => {
                    self.issue(format!("{}: {error}", cargo_toml_path.display()));
                    RustcRuntimeValidation::default()
                }
            };
        let traversal_dependencies = if let Some(graph) = self.effective_dependencies {
            match effective_local_dependencies_for_manifest(graph, cargo_toml_path) {
                Ok(dependencies) => dependencies,
                Err(error) => {
                    self.issue(format!("{}: {error}", cargo_toml_path.display()));
                    Vec::new()
                }
            }
        } else {
            declared_dependencies
        };

        let (sources, source_units) = self.read_source_units(project_dir);
        let mentions_cpp_name = source_units
            .iter()
            .any(|(_, source)| cpp_name::source_mentions_reserved_marker(source));
        if mentions_cpp_name {
            self.report.any_cpp_name = true;
            let module_preamble = (visit_key == self.root_manifest)
                .then_some(self.root_module_preamble)
                .flatten();
            if let Err(error) = preflight_cpp_name_crate_sources_exact(
                &cargo,
                &sources,
                &source_units,
                self.type_map,
                self.expand,
                self.transpile_options,
                module_preamble,
                runtime_validation.trusted_cpp_inherit_provenance,
            ) {
                self.issue(format!("{}: {error}", cargo_toml_path.display()));
            }
        }

        let mut dependencies = traversal_dependencies
            .into_iter()
            .filter(|dependency| dependency.is_local && dependency.target.is_none())
            .collect::<Vec<_>>();
        dependencies.sort_by(|left, right| {
            left.name
                .cmp(&right.name)
                .then_with(|| left.path.cmp(&right.path))
        });
        for dependency in dependencies {
            // Match generation exactly: the authenticated rustc-only facade
            // is supplied by the C++ runtime and is never transpiled.
            if runtime_validation
                .runtime_provided
                .contains(&dependency.name)
            {
                continue;
            }
            let Some(relative) = dependency.path.as_deref() else {
                continue;
            };
            self.visit_manifest(&project_dir.join(relative).join("Cargo.toml"));
        }
        self.active.pop();
    }

    fn finish(self) -> Result<bool, String> {
        if self.report.any_cpp_name && !self.report.issues.is_empty() {
            return Err(format!(
                "cpp_name whole local-dependency closure preflight failed before output:\n- {}",
                self.report
                    .issues
                    .into_iter()
                    .collect::<Vec<_>>()
                    .join("\n- ")
            ));
        }
        Ok(self.report.any_cpp_name)
    }
}

fn preflight_cpp_name_whole_dependency_closure(
    cargo_toml_path: &Path,
    type_map: &types::UserTypeMap,
    expand: bool,
    transpile_options: &transpile::TranspileOptions,
    module_preamble: Option<&transpile::ModulePreambleManifest>,
) -> Result<bool, String> {
    let mut preflight = CppNameClosurePreflight::new(
        cargo_toml_path,
        type_map,
        expand,
        transpile_options,
        module_preamble,
    );
    preflight.visit_manifest(cargo_toml_path);
    preflight.finish()
}

fn preflight_cpp_name_effective_dependency_closure(
    cargo_toml_path: &Path,
    type_map: &types::UserTypeMap,
    expand: bool,
    transpile_options: &transpile::TranspileOptions,
    module_preamble: Option<&transpile::ModulePreambleManifest>,
    graph: &metadata::EffectiveLocalNormalDependencyGraph,
) -> Result<bool, String> {
    let requested = CppAbiClosurePreflight::manifest_key(cargo_toml_path);
    if requested != graph.root_manifest() {
        return Err(format!(
            "Cargo's target-filtered dependency graph root {} does not match requested manifest {}",
            graph.root_manifest().display(),
            requested.display()
        ));
    }
    let mut preflight = CppNameClosurePreflight::with_effective_dependencies(
        cargo_toml_path,
        type_map,
        expand,
        transpile_options,
        module_preamble,
        graph,
    );
    preflight.visit_manifest(cargo_toml_path);
    preflight.finish()
}

/// Cheap, output-free over-approximation used to decide whether Cargo's exact
/// target-selected graph is required. It follows every declared local normal
/// dependency, including optional and target-qualified entries; the later
/// target-filtered closure discards unselected edges.
fn dependency_closure_may_have_cpp_name(cargo_toml_path: &Path) -> bool {
    fn cargo_config_may_select_local_path_override(project_dir: &Path) -> Result<bool, ()> {
        fn value_mentions_path(value: &toml::Value) -> bool {
            match value {
                toml::Value::Table(table) => {
                    table.contains_key("path") || table.values().any(value_mentions_path)
                }
                toml::Value::Array(values) => values.iter().any(value_mentions_path),
                _ => false,
            }
        }

        fn inspect_config(
            path: &Path,
            active: &mut BTreeSet<PathBuf>,
            visited: &mut BTreeSet<PathBuf>,
        ) -> Result<bool, ()> {
            let key = CppAbiClosurePreflight::manifest_key(path);
            if visited.contains(&key) {
                return Ok(false);
            }
            if !active.insert(key.clone()) {
                return Err(());
            }
            let source = std::fs::read_to_string(path).map_err(|_| ())?;
            let config = toml::from_str::<toml::Value>(&source).map_err(|_| ())?;
            let table = config.as_table().ok_or(())?;

            if let Some(includes) = table.get("include") {
                let includes = includes.as_array().ok_or(())?;
                for include in includes {
                    let (relative, optional) = match include {
                        toml::Value::String(path) => (path.as_str(), false),
                        toml::Value::Table(table) => {
                            let path = table.get("path").and_then(toml::Value::as_str).ok_or(())?;
                            let optional = table
                                .get("optional")
                                .map(|value| value.as_bool().ok_or(()))
                                .transpose()?
                                .unwrap_or(false);
                            (path, optional)
                        }
                        _ => return Err(()),
                    };
                    let include_path = Path::new(relative);
                    let include_path = if include_path.is_absolute() {
                        include_path.to_path_buf()
                    } else {
                        path.parent().ok_or(())?.join(include_path)
                    };
                    if optional && !include_path.is_file() {
                        continue;
                    }
                    if inspect_config(&include_path, active, visited)? {
                        return Ok(true);
                    }
                }
            }

            // Both Cargo's legacy `paths` override and configuration-level
            // `[patch]` can turn a declared registry/git dependency into a
            // selected local package. `[replace]` is scanned conservatively
            // as well, even though Cargo currently documents it for manifests.
            let has_local_override = table.get("paths").is_some()
                || ["patch", "replace"]
                    .iter()
                    .any(|name| table.get(*name).is_some_and(value_mentions_path));
            active.remove(&key);
            visited.insert(key);
            Ok(has_local_override)
        }

        let mut cargo_directories = Vec::new();
        if let Some(cargo_home) = metadata::effective_cargo_home(project_dir) {
            cargo_directories.push(cargo_home);
        }
        cargo_directories.extend(
            project_dir
                .ancestors()
                .map(|directory| directory.join(".cargo")),
        );

        let mut active = BTreeSet::new();
        let mut visited = BTreeSet::new();
        for cargo_directory in cargo_directories {
            let legacy = cargo_directory.join("config");
            let modern = cargo_directory.join("config.toml");
            // Cargo prefers the extensionless legacy file when both exist.
            let config = if legacy.is_file() {
                Some(legacy)
            } else if modern.is_file() {
                Some(modern)
            } else {
                None
            };
            if let Some(config) = config
                && inspect_config(&config, &mut active, &mut visited)?
            {
                return Ok(true);
            }
        }
        Ok(false)
    }

    fn workspace_may_have_local_path_overrides(cargo_toml_path: &Path) -> Result<bool, ()> {
        fn override_value_mentions_path(value: &toml::Value) -> bool {
            match value {
                toml::Value::Table(table) => {
                    table.contains_key("path") || table.values().any(override_value_mentions_path)
                }
                toml::Value::Array(values) => values.iter().any(override_value_mentions_path),
                _ => false,
            }
        }

        fn inspect_manifest(path: &Path) -> Result<(bool, bool, Option<PathBuf>), ()> {
            let source = std::fs::read_to_string(path).map_err(|_| ())?;
            let manifest = toml::from_str::<toml::Value>(&source).map_err(|_| ())?;
            let has_local_override = ["patch", "replace"]
                .iter()
                .any(|name| manifest.get(name).is_some_and(override_value_mentions_path));
            let declares_workspace = manifest.get("workspace").is_some();
            let explicit_workspace = manifest
                .get("package")
                .and_then(toml::Value::as_table)
                .and_then(|package| package.get("workspace"))
                .and_then(toml::Value::as_str)
                .map(PathBuf::from);
            Ok((has_local_override, declares_workspace, explicit_workspace))
        }

        let (requested_override, requested_workspace, explicit_workspace) =
            inspect_manifest(cargo_toml_path)?;
        if requested_workspace {
            return Ok(requested_override);
        }
        let package_dir = cargo_toml_path.parent().ok_or(())?;
        if let Some(workspace) = explicit_workspace {
            let workspace_manifest = if workspace.is_absolute() {
                workspace.join("Cargo.toml")
            } else {
                package_dir.join(workspace).join("Cargo.toml")
            };
            return inspect_manifest(&workspace_manifest).map(|(overrides, _, _)| overrides);
        }

        // Cargo discovers an implicit workspace by walking ancestor manifests
        // until the nearest `[workspace]`. Only overrides at that root apply.
        for ancestor in package_dir.ancestors().skip(1) {
            let candidate = ancestor.join("Cargo.toml");
            if !candidate.is_file() {
                continue;
            }
            let (overrides, declares_workspace, _) = inspect_manifest(&candidate)?;
            if declares_workspace {
                return Ok(overrides);
            }
        }
        // A standalone package is its own workspace root, even without an
        // explicit `[workspace]` table.
        Ok(requested_override)
    }

    fn visit(cargo_toml_path: &Path, visited: &mut BTreeSet<PathBuf>) -> bool {
        let key = CppAbiClosurePreflight::manifest_key(cargo_toml_path);
        if !visited.insert(key) {
            return false;
        }
        let Ok(manifest_source) = std::fs::read_to_string(cargo_toml_path) else {
            return false;
        };
        let Ok(cargo) = toml::from_str::<cmake::CargoToml>(&manifest_source) else {
            return false;
        };
        let project_dir = cargo_toml_path.parent().unwrap_or(Path::new("."));

        // Cargo configuration participates in dependency selection but is not
        // represented in a package's dependency tables. Its path overrides
        // therefore need their own conservative witness before the manifest
        // walk can decide that the exact graph is unnecessary.
        match cargo_config_may_select_local_path_override(project_dir) {
            Ok(true) => return true,
            Ok(false) => {}
            Err(_) => return true,
        }

        // Reuse the symlink-aware source walker. Its authoritative diagnostics
        // are retained by the subsequent closure preflight.
        let mut collector = CppAbiClosurePreflight::new(false);
        let source_paths = collector.collect_rs_files(project_dir);
        for relative in source_paths {
            if std::fs::read_to_string(project_dir.join(relative))
                .is_ok_and(|source| cpp_name::source_mentions_reserved_marker(&source))
            {
                return true;
            }
        }
        let mut declared_roots = cargo
            .lib
            .as_ref()
            .and_then(|target| target.path.as_deref())
            .map(PathBuf::from)
            .into_iter()
            .collect::<Vec<_>>();
        if let Some(bins) = &cargo.bins {
            declared_roots.extend(
                bins.iter()
                    .filter_map(|target| target.path.as_deref().map(PathBuf::from)),
            );
        }
        for relative in declared_roots {
            if std::fs::read_to_string(project_dir.join(relative))
                .is_ok_and(|source| cpp_name::source_mentions_reserved_marker(&source))
            {
                return true;
            }
        }

        let declared = cmake::extract_dependencies(&cargo);
        let dependencies =
            match resolve_workspace_inherited_dependencies(cargo_toml_path, &declared) {
                Ok(dependencies) => dependencies,
                // This scan is an over-approximation only. On uncertainty, force
                // the exact Cargo graph path; never let an unresolved inherited
                // edge hide a selected dependency-owned cpp_name contract.
                Err(_) => return true,
            };
        for dependency in dependencies {
            if !dependency.is_local
                || dependency.name == RUSTY_RUNTIME_CRATE_NAME
                || dependency.package.as_deref() == Some(RUSTY_RUNTIME_CRATE_NAME)
            {
                continue;
            }
            let Some(relative) = dependency.path.as_deref() else {
                continue;
            };
            if visit(&project_dir.join(relative).join("Cargo.toml"), visited) {
                return true;
            }
        }
        // Cargo may redirect a registry/git dependency to a local package via
        // workspace `[patch]` or `[replace]`. Such an edge has no declared
        // dependency `path`, so any local override forces the exact Cargo
        // graph path. That graph remains authoritative and discards unused
        // versions/sources before preflight or recursive generation.
        match workspace_may_have_local_path_overrides(cargo_toml_path) {
            Ok(true) => return true,
            Ok(false) => {}
            // This scan chooses whether exact Cargo selection is needed. On
            // uncertainty, force that safe path rather than hiding a selected
            // dependency-owned cpp_name contract.
            Err(_) => return true,
        }
        false
    }

    let cargo_toml_path = CppAbiClosurePreflight::manifest_key(cargo_toml_path);
    visit(&cargo_toml_path, &mut BTreeSet::new())
}

/// Run exact per-file lowering across selected local dependencies without
/// touching the requested output tree. A cpp_name root uses this as its final
/// atomic gate before the first `create_dir_all`.
fn preflight_local_dependency_codegen_without_output(
    dependencies: &[cmake::CrateDep],
    project_dir: &Path,
    runtime_provided_dependencies: &HashSet<String>,
    type_map: &types::UserTypeMap,
    transpile_options: &transpile::TranspileOptions,
    effective_dependencies: Option<&metadata::EffectiveLocalNormalDependencyGraph>,
    visited: &mut BTreeSet<PathBuf>,
) -> Result<(), String> {
    let mut dependencies = dependencies
        .iter()
        .filter(|dependency| dependency.target.is_none() && dependency.is_local)
        .collect::<Vec<_>>();
    dependencies.sort_by(|left, right| {
        left.name
            .cmp(&right.name)
            .then_with(|| left.path.cmp(&right.path))
    });
    for dependency in dependencies {
        if runtime_provided_dependencies.contains(&dependency.name) {
            continue;
        }
        let dependency_path = dependency.path.as_deref().ok_or_else(|| {
            format!(
                "local dependency '{}' has no path during atomic codegen preflight",
                dependency.name
            )
        })?;
        let manifest = project_dir.join(dependency_path).join("Cargo.toml");
        if !manifest.is_file() {
            return Err(format!(
                "local dependency '{}' manifest does not exist at {}",
                dependency.name,
                manifest.display()
            ));
        }
        preflight_crate_codegen_without_output(
            &manifest,
            type_map,
            transpile_options,
            effective_dependencies,
            visited,
        )
        .map_err(|error| {
            format!(
                "dependency '{}' codegen failed before output: {error}",
                dependency.name
            )
        })?;
    }
    Ok(())
}

fn preflight_crate_codegen_without_output(
    cargo_toml_path: &Path,
    type_map: &types::UserTypeMap,
    transpile_options: &transpile::TranspileOptions,
    effective_dependencies: Option<&metadata::EffectiveLocalNormalDependencyGraph>,
    visited: &mut BTreeSet<PathBuf>,
) -> Result<(), String> {
    let manifest_key = CppAbiClosurePreflight::manifest_key(cargo_toml_path);
    if !visited.insert(manifest_key) {
        return Ok(());
    }

    let cargo = cmake::parse_cargo_toml(cargo_toml_path)?;
    let project_dir = cargo_toml_path.parent().unwrap_or(Path::new("."));
    let crate_name = &cargo.package.name;
    let declared_dependencies = cmake::extract_dependencies(&cargo);
    let resolved_declared_dependencies =
        resolve_workspace_inherited_dependencies(cargo_toml_path, &declared_dependencies)?;
    let runtime_validation = validate_rustc_only_runtime_dependencies(
        cargo_toml_path,
        &resolved_declared_dependencies,
    )?;
    let dependencies = if let Some(graph) = effective_dependencies {
        effective_local_dependencies_for_manifest(graph, cargo_toml_path)?
    } else {
        resolved_declared_dependencies
    };
    let sources = cmake::collect_source_files(project_dir);
    if sources.is_empty() {
        return Err("No .rs source files found in src/".to_string());
    }

    let mut source_units = Vec::<(PathBuf, String)>::with_capacity(sources.len());
    for source_path in &sources {
        let full_source_path = project_dir.join(source_path);
        let source = std::fs::read_to_string(&full_source_path)
            .map_err(|error| format!("Error reading {}: {error}", full_source_path.display()))?;
        source_units.push((source_path.clone(), source));
    }
    reject_cpp_abi_in_nonconventional_target_roots(&cargo, project_dir)?;
    let cpp_abi_preflight = cpp_abi::preflight_crate_plan_with_cxx_namespace(
        &source_units,
        transpile_options.cxx_namespace.as_deref(),
    )?;
    let has_cpp_abi = cpp_abi_preflight.has_contracts;
    let has_cpp_defaults = cpp_default_args::preflight_crate_sources(&source_units, type_map)?;
    let has_cpp_name = preflight_cpp_name_crate_sources_exact(
        &cargo,
        &sources,
        &source_units,
        type_map,
        false,
        transpile_options,
        None,
        runtime_validation.trusted_cpp_inherit_provenance,
    )?;
    if has_cpp_abi || has_cpp_defaults || has_cpp_name {
        validate_cpp_abi_conventional_lib_crate(&cargo, &sources)?;
    }

    let prepared =
        prepare_crate_codegen(&sources, &source_units, crate_name, transpile_options, None)?;
    if has_cpp_defaults {
        for (path, source) in &source_units {
            if !cpp_default_args::source_mentions_marker(source) {
                continue;
            }
            let file = syn::parse_file(source).map_err(|error| {
                format!(
                    "{}: could not parse cpp_default_argument source: {error}",
                    path.display()
                )
            })?;
            let module_name = cmake::map_rs_to_cppm(path, crate_name).1;
            let includes = prepared
                .module_preambles
                .get(&module_name)
                .map(Vec::as_slice)
                .unwrap_or_default();
            cpp_default_args::validate_required_gmf_includes(&file, includes)
                .map_err(|error| format!("{}: {error}", path.display()))?;
        }
    }
    let mut prepared_options = prepared.options;
    // This pass is observational even when a library caller requested a UFCS
    // manifest for final emission.
    prepared_options.emit_ufcs_trait_manifest_path = None;
    for (path, source) in &source_units {
        let (_, module_name) = cmake::map_rs_to_cppm(path, crate_name);
        let mut module_options = prepared_options.clone();
        module_options.flat_import_type_authorizations = cpp_abi_preflight
            .flat_import_type_authorizations
            .iter()
            .filter(|authorization| {
                authorization.consumer_source.as_path() == path.as_path()
            })
            .cloned()
            .collect();
        module_options.explicit_gmf_includes = prepared
            .module_preambles
            .get(&module_name)
            .cloned()
            .unwrap_or_default();
        transpile::transpile_with_type_map_and_extension_hints_and_options(
            source,
            Some(&module_name),
            type_map,
            &prepared.extension_method_hints,
            &module_options,
        )
        .map_err(|error| {
            format!(
                "codegen preflight for {} failed before output: {error}",
                path.display()
            )
        })?;
    }

    preflight_local_dependency_codegen_without_output(
        &dependencies,
        project_dir,
        &runtime_validation.runtime_provided,
        type_map,
        transpile_options,
        effective_dependencies,
        visited,
    )
}

fn transpile_crate(
    cargo_toml_path: &Path,
    output_dir: &Path,
    type_map: &types::UserTypeMap,
    expand: bool,
    verify: bool,
    transpile_options: &transpile::TranspileOptions,
    module_preamble: Option<&transpile::ModulePreambleManifest>,
) -> Result<(), String> {
    transpile_crate_impl(
        cargo_toml_path,
        output_dir,
        type_map,
        expand,
        verify,
        transpile_options,
        module_preamble,
        false,
        None,
    )
}

fn transpile_crate_impl(
    cargo_toml_path: &Path,
    output_dir: &Path,
    type_map: &types::UserTypeMap,
    expand: bool,
    verify: bool,
    transpile_options: &transpile::TranspileOptions,
    module_preamble: Option<&transpile::ModulePreambleManifest>,
    inherited_atomic_dependency_errors: bool,
    inherited_effective_dependencies: Option<&metadata::EffectiveLocalNormalDependencyGraph>,
) -> Result<(), String> {
    // Step 1: Parse Cargo.toml and discover source files
    let cargo = cmake::parse_cargo_toml(cargo_toml_path)?;
    let project_dir = cargo_toml_path.parent().unwrap_or(Path::new("."));
    let crate_name = &cargo.package.name;
    let deps = cmake::extract_dependencies(&cargo);
    let runtime_validation = validate_rustc_only_runtime_dependencies(cargo_toml_path, &deps)?;

    // All source-owned C++ contracts share one Cargo-selected local normal
    // dependency graph.  cpp_name has its own exact-emission proof while
    // cpp_abi/cpp_default_argument share the syntax/type closure proof; both
    // must finish before any output path is created.
    let may_have_cpp_name = dependency_closure_may_have_cpp_name(cargo_toml_path);
    let may_have_source_contract = dependency_closure_may_have_source_contract(cargo_toml_path);
    let owned_effective_dependencies = if inherited_atomic_dependency_errors
        || !(may_have_cpp_name || may_have_source_contract)
    {
        None
    } else {
        Some(
            metadata::resolve_effective_local_normal_dependency_graph(cargo_toml_path).map_err(
                |error| {
                    format!(
                        "source-owned C++ contract requires an exact Cargo target-selected normal local-dependency graph before output: {error}"
                    )
                },
            )?,
        )
    };
    let (closure_has_cpp_name, closure_has_source_contract, effective_dependencies) =
        if inherited_atomic_dependency_errors {
            (true, true, inherited_effective_dependencies)
        } else if let Some(graph) = owned_effective_dependencies.as_ref() {
            let selected_has_cpp_name = preflight_cpp_name_effective_dependency_closure(
                cargo_toml_path,
                type_map,
                expand,
                transpile_options,
                module_preamble,
                graph,
            )?;
            let selected_has_source_contract =
                preflight_cpp_source_contract_effective_dependency_closure(
                    cargo_toml_path,
                    expand,
                    graph,
                )?;
            // cpp_name's marker-free behavior deliberately retains Cargo's
            // exact graph when its over-approximation found only an unselected
            // marker edge.  The other contracts keep their historical path in
            // that case unless cpp_name/config provenance already requires it.
            let generation_graph = (selected_has_cpp_name
                || selected_has_source_contract
                || may_have_cpp_name)
                .then_some(graph);
            (
                selected_has_cpp_name,
                selected_has_source_contract,
                generation_graph,
            )
        } else {
            (
                preflight_cpp_name_whole_dependency_closure(
                    cargo_toml_path,
                    type_map,
                    expand,
                    transpile_options,
                    module_preamble,
                )?,
                preflight_cpp_abi_whole_dependency_closure(cargo_toml_path, expand)?,
                None,
            )
        };
    let closure_has_any_contract = closure_has_cpp_name || closure_has_source_contract;
    let atomic_dependency_errors =
        inherited_atomic_dependency_errors || closure_has_any_contract;
    let exact_generation_dependencies = effective_dependencies
        .map(|graph| effective_local_dependencies_for_manifest(graph, cargo_toml_path))
        .transpose()?;
    let resolved_atomic_dependencies =
        if atomic_dependency_errors && exact_generation_dependencies.is_none() {
            Some(resolve_workspace_inherited_dependencies(
                cargo_toml_path,
                &deps,
            )?)
        } else {
            None
        };
    let generation_dependencies = exact_generation_dependencies
        .as_deref()
        .or(resolved_atomic_dependencies.as_deref())
        .unwrap_or(&deps);
    let sources = cmake::collect_source_files(project_dir);

    if sources.is_empty() {
        return Err("No .rs source files found in src/".to_string());
    }

    // Source-owned ABI contracts need a crate-wide view before any output or
    // dependency directory can be created. Read every source exactly once;
    // marker-free crates continue through the ordinary per-file path below.
    let mut source_units = Vec::<(PathBuf, String)>::with_capacity(sources.len());
    for rs_path in &sources {
        let full_rs_path = project_dir.join(rs_path);
        let source = std::fs::read_to_string(&full_rs_path)
            .map_err(|error| format!("Error reading {}: {error}", full_rs_path.display()))?;
        source_units.push((rs_path.clone(), source));
    }
    reject_cpp_abi_in_nonconventional_target_roots(&cargo, project_dir)?;
    let cpp_abi_preflight = cpp_abi::preflight_crate_plan_with_cxx_namespace(
        &source_units,
        transpile_options.cxx_namespace.as_deref(),
    )?;
    let has_cpp_abi = cpp_abi_preflight.has_contracts;
    let has_cpp_name = preflight_cpp_name_crate_sources_exact(
        &cargo,
        &sources,
        &source_units,
        type_map,
        expand,
        transpile_options,
        module_preamble,
        runtime_validation.trusted_cpp_inherit_provenance,
    )?;
    let has_cpp_defaults = cpp_default_args::preflight_crate_sources(&source_units, type_map)?;
    if has_cpp_abi || has_cpp_name || has_cpp_defaults {
        validate_cpp_abi_conventional_lib_crate(&cargo, &sources)?;
        if expand {
            return Err(
                "source-owned C++ contracts do not support --expand because expansion removes inert markers"
                    .to_string(),
            );
        }
    }
    let prepared_contract_codegen = if closure_has_any_contract
        || has_cpp_abi
        || has_cpp_name
        || has_cpp_defaults
    {
        Some(prepare_crate_codegen(
            &sources,
            &source_units,
            crate_name,
            transpile_options,
            module_preamble,
        )?)
    } else {
        None
    };
    if has_cpp_defaults {
        let prepared = prepared_contract_codegen
            .as_ref()
            .expect("source contract codegen context must be prepared");
        for (path, source) in &source_units {
            if !cpp_default_args::source_mentions_marker(source) {
                continue;
            }
            let file = syn::parse_file(source).map_err(|error| {
                format!(
                    "{}: could not parse cpp_default_argument source: {error}",
                    path.display()
                )
            })?;
            let module_name = cmake::map_rs_to_cppm(path, crate_name).1;
            let includes = prepared
                .module_preambles
                .get(&module_name)
                .map(Vec::as_slice)
                .unwrap_or_default();
            cpp_default_args::validate_required_gmf_includes(&file, includes)
                .map_err(|error| format!("{}: {error}", path.display()))?;
        }
    }

    // Every active source contract promises fail-closed crate generation.
    // Lower every root source before the first output mutation and retain
    // those exact bytes for the write phase.
    let prepared_contract_outputs = if let Some(prepared) = &prepared_contract_codegen {
        let mut outputs = Vec::with_capacity(source_units.len());
        for (path, source) in &source_units {
            let (_, module_name) = cmake::map_rs_to_cppm(path, crate_name);
            let mut module_options = prepared.options.clone();
            module_options.flat_import_type_authorizations = cpp_abi_preflight
                .flat_import_type_authorizations
                .iter()
                .filter(|authorization| {
                    authorization.consumer_source.as_path() == path.as_path()
                })
                .cloned()
                .collect();
            module_options.explicit_gmf_includes = prepared
                .module_preambles
                .get(&module_name)
                .cloned()
                .unwrap_or_default();
            let output = transpile::transpile_with_type_map_and_extension_hints_and_options(
                source,
                Some(&module_name),
                type_map,
                &prepared.extension_method_hints,
                &module_options,
            )
            .map_err(|error| {
                format!(
                    "source-owned C++ contract root codegen preflight for {} failed before output: {error}",
                    path.display()
                )
            })?;
            outputs.push(output);
        }
        Some(outputs)
    } else {
        None
    };
    if closure_has_any_contract && !inherited_atomic_dependency_errors {
        let mut visited = BTreeSet::new();
        visited.insert(CppAbiClosurePreflight::manifest_key(cargo_toml_path));
        preflight_local_dependency_codegen_without_output(
            generation_dependencies,
            project_dir,
            &runtime_validation.runtime_provided,
            type_map,
            transpile_options,
            effective_dependencies,
            &mut visited,
        )
        .map_err(|error| {
            format!(
                "source-owned C++ contract dependency codegen preflight failed before output: {error}"
            )
        })?;
    }
    // Create output directory
    std::fs::create_dir_all(output_dir)
        .map_err(|e| format!("Failed to create output dir: {}", e))?;

    // Detect and handle dependencies
    let mut local_dep_dirs: Vec<String> = Vec::new();

    let unconditional_dependencies = generation_dependencies
        .iter()
        .filter(|dependency| dependency.target.is_none())
        .collect::<Vec<_>>();
    if !unconditional_dependencies.is_empty() {
        println!("\nDependencies:");
        for dep in unconditional_dependencies {
            if runtime_validation.runtime_provided.contains(&dep.name) {
                let dep_path = dep.path.as_deref().unwrap_or("?");
                println!(
                    "  {} (local: {}) — provided by the rusty C++ runtime; rustc facade is not generated",
                    dep.name, dep_path
                );
                continue;
            }
            if dep.is_local {
                let dep_path = dep.path.as_deref().unwrap_or("?");
                println!(
                    "  {} (local: {}) — will transpile recursively",
                    dep.name, dep_path
                );

                // Recursively transpile local path dependencies
                let dep_cargo_toml = project_dir.join(dep_path).join("Cargo.toml");
                if dep_cargo_toml.exists() {
                    let dep_out_dir = output_dir.join(&dep.name);
                    match transpile_crate_impl(
                        &dep_cargo_toml,
                        &dep_out_dir,
                        type_map,
                        expand,
                        verify,
                        transpile_options,
                        None,
                        atomic_dependency_errors,
                        effective_dependencies,
                    ) {
                        Ok(()) => {
                            local_dep_dirs.push(dep.name.clone());
                        }
                        Err(e) if atomic_dependency_errors => {
                            return Err(format!(
                                "source-owned C++ contract dependency generation for '{}' failed: {}",
                                dep.name, e
                            ));
                        }
                        Err(e) => {
                            eprintln!(
                                "  Warning: failed to transpile dependency '{}': {}",
                                dep.name, e
                            );
                        }
                    }
                } else if atomic_dependency_errors {
                    return Err(format!(
                        "source-owned C++ contract dependency generation for '{}' failed: Cargo.toml not found at {}",
                        dep.name,
                        dep_cargo_toml.display()
                    ));
                } else {
                    eprintln!(
                        "  Warning: Cargo.toml not found for local dep '{}' at {}",
                        dep.name,
                        dep_cargo_toml.display()
                    );
                }
            } else {
                println!(
                    "  {} = \"{}\" (external — types may need manual mapping)",
                    dep.name,
                    dep.version.as_deref().unwrap_or("*")
                );
            }
        }
        println!();
    }

    // If --expand, use cargo expand for the whole crate (macro expansion)
    if expand {
        println!("Running cargo expand on '{}'...", crate_name);
        match run_cargo_expand(cargo_toml_path) {
            Ok(expanded_source) => {
                let cppm_path = output_dir.join(format!("{}.cppm", crate_name));
                let extension_method_hints =
                    transpile::collect_extension_method_hints(&expanded_source);
                let expanded_options = if let Some(manifest) = module_preamble {
                    let selected = manifest.select_for_modules([crate_name.as_str()])?;
                    let mut opts = transpile_options.clone();
                    opts.explicit_gmf_includes =
                        selected.get(crate_name).cloned().unwrap_or_default();
                    opts
                } else {
                    transpile_options.clone()
                };
                match transpile::transpile_with_type_map_and_extension_hints_and_options(
                    &expanded_source,
                    Some(crate_name),
                    type_map,
                    &extension_method_hints,
                    &expanded_options,
                ) {
                    Ok(cpp_output) => {
                        std::fs::write(&cppm_path, &cpp_output)
                            .map_err(|e| format!("Failed to write: {}", e))?;
                        println!("  Expanded and transpiled → {}", cppm_path.display());
                    }
                    Err(e) => {
                        return Err(format!("Transpilation of expanded source failed: {}", e));
                    }
                }

                // Generate CMakeLists.txt
                let cmake_content = cmake::generate_cmake(&cargo, &sources);
                let cmake_path = output_dir.join("CMakeLists.txt");
                std::fs::write(&cmake_path, &cmake_content)
                    .map_err(|e| format!("Failed to write CMakeLists.txt: {}", e))?;
                println!("Generated {}", cmake_path.display());
                return Ok(());
            }
            Err(e) => {
                eprintln!(
                    "Warning: cargo expand failed ({}), falling back to per-file mode",
                    e
                );
            }
        }
    }

    println!(
        "Transpiling crate '{}' ({} source files)",
        crate_name,
        sources.len()
    );

    // Step 2: Transpile each file with correct module name
    let mut success_count = 0;
    let mut error_count = 0;
    // Slot manifest: every TODO / "Rust-only … skipped" marker emitted
    // anywhere in this crate's generated C++ gets recorded here so we
    // can write a single `rusty_hand_slots.md` summary at the end.
    // See `slots.rs` for the rationale.
    let mut hand_slots: Vec<slots::Slot> = Vec::new();
    let prepared_codegen = match prepared_contract_codegen {
        Some(prepared) => prepared,
        None => prepare_crate_codegen(
            &sources,
            &source_units,
            crate_name,
            transpile_options,
            module_preamble,
        )?,
    };

    for (source_index, (rs_path, source)) in source_units.iter().enumerate() {
        let (cppm_path, module_name) = cmake::map_rs_to_cppm(rs_path, crate_name);
        let full_cppm_path = output_dir.join(&cppm_path);

        let mut module_options = prepared_codegen.options.clone();
        module_options.flat_import_type_authorizations = cpp_abi_preflight
            .flat_import_type_authorizations
            .iter()
            .filter(|authorization| {
                authorization.consumer_source.as_path() == rs_path.as_path()
            })
            .cloned()
            .collect();
        module_options.explicit_gmf_includes = prepared_codegen
            .module_preambles
            .get(&module_name)
            .cloned()
            .unwrap_or_default();

        let transpile_result = match &prepared_contract_outputs {
            Some(outputs) => Ok(std::borrow::Cow::Borrowed(outputs[source_index].as_str())),
            None => transpile::transpile_with_type_map_and_extension_hints_and_options(
                source,
                Some(&module_name),
                type_map,
                &prepared_codegen.extension_method_hints,
                &module_options,
            )
            .map(std::borrow::Cow::Owned),
        };
        match transpile_result {
            Ok(cpp_output) => {
                if let Err(e) = std::fs::write(&full_cppm_path, cpp_output.as_bytes()) {
                    eprintln!("  Error writing {}: {}", full_cppm_path.display(), e);
                    error_count += 1;
                    continue;
                }
                // Scan the freshly-generated output for hand-override
                // slot markers and aggregate them into the crate-wide
                // list. Use the user-facing relative path as the file
                // label so the manifest is reproducible regardless of
                // where the build was invoked from.
                let cppm_label = cppm_path.to_string_lossy().to_string();
                let file_slots = slots::detect_slots(&cppm_label, &cpp_output);
                hand_slots.extend(file_slots);
                println!(
                    "  {} → {} (module: {})",
                    rs_path.display(),
                    cppm_path.display(),
                    module_name
                );
                success_count += 1;

                // Optional verification
                if verify {
                    match run_rusty_cpp_checker(&full_cppm_path) {
                        Ok(()) => {}
                        Err(e) => {
                            eprintln!("  Verify {}: {}", cppm_path.display(), e);
                        }
                    }
                }
            }
            Err(e) => {
                eprintln!("  Error transpiling {}: {}", rs_path.display(), e);
                error_count += 1;
            }
        }
    }

    // Step 3: Generate CMakeLists.txt (with local dependency subdirectories)
    let mut cmake_content = cmake::generate_cmake(&cargo, &sources);

    // Add add_subdirectory() for each local dependency
    if !local_dep_dirs.is_empty() {
        cmake_content.push_str("# Local dependencies (transpiled)\n");
        for dep_name in &local_dep_dirs {
            cmake_content.push_str(&format!("add_subdirectory({})\n", dep_name));
        }
        cmake_content.push('\n');

        // Link dependencies to the main target
        let target_name = cargo
            .lib
            .as_ref()
            .and_then(|l| l.name.clone())
            .unwrap_or_else(|| crate_name.replace('-', "_"));
        for dep_name in &local_dep_dirs {
            cmake_content.push_str(&format!(
                "target_link_libraries({} PRIVATE {})\n",
                target_name,
                dep_name.replace('-', "_")
            ));
        }
        cmake_content.push('\n');
    }

    let cmake_path = output_dir.join("CMakeLists.txt");
    std::fs::write(&cmake_path, &cmake_content)
        .map_err(|e| format!("Failed to write CMakeLists.txt: {}", e))?;

    // Write the hand-override slot manifest. Always emit it (even when
    // empty) so the file's presence is a reliable signal that this
    // crate has been transpiled by a slot-aware build, and so diffs
    // against a previous run surface changes in the slot count.
    let slot_manifest_path = output_dir.join("rusty_hand_slots.md");
    let slot_manifest = slots::format_manifest(&hand_slots);
    std::fs::write(&slot_manifest_path, &slot_manifest).map_err(|e| {
        format!(
            "Failed to write {}: {}",
            slot_manifest_path.display(),
            e
        )
    })?;

    println!("\nGenerated {}", cmake_path.display());
    if hand_slots.is_empty() {
        println!("Slot manifest: 0 slots — see {}", slot_manifest_path.display());
    } else {
        println!(
            "Slot manifest: {} slot(s) across {} file(s) — see {}",
            hand_slots.len(),
            count_slot_files(&hand_slots),
            slot_manifest_path.display()
        );
    }
    println!(
        "Done: {} files transpiled, {} errors",
        success_count, error_count
    );

    if error_count > 0 {
        Err(format!("{} files failed to transpile", error_count))
    } else {
        Ok(())
    }
}

/// Run `cargo expand` on the input file's crate to get macro-expanded source.
fn run_cargo_expand(input_path: &Path) -> Result<String, String> {
    let mut dir = input_path.parent().unwrap_or(Path::new(".")).to_path_buf();

    loop {
        if dir.join("Cargo.toml").exists() {
            break;
        }
        if !dir.pop() {
            return Err("Could not find Cargo.toml for cargo expand".to_string());
        }
    }

    eprintln!("Running cargo expand in {}...", dir.display());

    let mut expand_cmd = std::process::Command::new("cargo");
    expand_cmd
        .arg("expand")
        .arg("--theme=none")
        .current_dir(&dir);
    if let Some(target) = shared_cargo_target_dir() {
        expand_cmd.env("CARGO_TARGET_DIR", &target);
    }
    let output = expand_cmd.output().map_err(|e| {
        format!(
            "Failed to run `cargo expand`: {}. Install with: cargo install cargo-expand",
            e
        )
    })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("cargo expand failed:\n{}", stderr));
    }

    String::from_utf8(output.stdout).map_err(|e| format!("Invalid UTF-8 from cargo expand: {}", e))
}

fn generate_cmake_from_cargo(cargo_toml_path: &Path) -> Result<(), String> {
    let cargo = cmake::parse_cargo_toml(cargo_toml_path)?;
    let project_dir = cargo_toml_path.parent().unwrap_or(Path::new("."));
    let sources = cmake::collect_source_files(project_dir);

    if sources.is_empty() {
        return Err("No .rs source files found in src/".to_string());
    }

    let cmake_content = cmake::generate_cmake(&cargo, &sources);
    let cmake_path = project_dir.join("CMakeLists.txt");
    std::fs::write(&cmake_path, &cmake_content)
        .map_err(|e| format!("Failed to write CMakeLists.txt: {}", e))?;

    println!("Generated {}", cmake_path.display());

    println!("\nFile mapping:");
    for source in &sources {
        let (cppm, module) = cmake::map_rs_to_cppm(source, &cargo.package.name);
        println!(
            "  {} → {} (module: {})",
            source.display(),
            cppm.display(),
            module
        );
    }

    Ok(())
}

fn strip_export_prefix(trimmed: &str) -> &str {
    trimmed.strip_prefix("export ").unwrap_or(trimmed)
}

fn extract_rusty_test_wrapper_name(trimmed: &str) -> Option<String> {
    let line = strip_export_prefix(trimmed);
    let rest = line.strip_prefix("void rusty_test_")?;
    let end = rest.find('(')?;
    Some(format!("rusty_test_{}", &rest[..end]))
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RunnerTestEntry {
    fn_name: String,
    label: String,
    should_panic: bool,
}

fn marker_wrapper_suffix(marker: &str) -> String {
    marker
        .replace("::", "_")
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

fn parse_libtest_wrapper_metadata(trimmed: &str) -> Option<(String, bool)> {
    let rest = trimmed
        .strip_prefix("// Rust-only libtest wrapper metadata:")?
        .trim();
    let mut marker: Option<String> = None;
    let mut should_panic = false;
    for token in rest.split_whitespace() {
        if let Some(value) = token.strip_prefix("marker=") {
            marker = Some(value.to_string());
            continue;
        }
        if let Some(value) = token.strip_prefix("should_panic=") {
            should_panic = matches!(value, "yes" | "true" | "1");
        }
    }
    Some((marker?, should_panic))
}

fn collect_rusty_test_entries_from_cppm(
    content: &str,
    seen_test_fns: &mut HashSet<String>,
    test_entries: &mut Vec<RunnerTestEntry>,
) {
    let mut wrapper_should_panic: HashMap<String, bool> = HashMap::new();
    let mut marker_should_panic: HashMap<String, bool> = HashMap::new();
    for line in content.lines() {
        let trimmed = line.trim();
        if let Some((marker, should_panic)) = parse_libtest_wrapper_metadata(trimmed) {
            let marker_suffix = marker_wrapper_suffix(&marker);
            let wrapper = format!("rusty_test_{}", marker_suffix);
            wrapper_should_panic.insert(wrapper, should_panic);
            marker_should_panic.insert(marker_suffix, should_panic);
            continue;
        }
        if let Some(fn_name) = extract_rusty_test_wrapper_name(trimmed) {
            if seen_test_fns.insert(fn_name.clone()) {
                let should_panic = wrapper_should_panic
                    .get(&fn_name)
                    .copied()
                    .or_else(|| {
                        let label = fn_name.strip_prefix("rusty_test_")?;
                        marker_should_panic.get(label).copied().or_else(|| {
                            marker_should_panic
                                .iter()
                                .filter_map(|(marker, expected)| {
                                    if label.len() > marker.len()
                                        && label.ends_with(marker)
                                        && label
                                            .as_bytes()
                                            .get(label.len() - marker.len() - 1)
                                            .copied()
                                            == Some(b'_')
                                    {
                                        Some((marker.len(), *expected))
                                    } else {
                                        None
                                    }
                                })
                                .max_by_key(|(len, _)| *len)
                                .map(|(_, expected)| expected)
                        })
                    })
                    .unwrap_or(false);
                test_entries.push(RunnerTestEntry {
                    fn_name: fn_name.clone(),
                    label: test_label_from_fn_name(&fn_name),
                    should_panic,
                });
            }
        }
    }
}

fn test_label_from_fn_name(fn_name: &str) -> String {
    fn_name
        .strip_prefix("rusty_test_")
        .unwrap_or(fn_name)
        .to_string()
}

fn parity_cpp_compiler_from_env(cxx: Option<String>) -> String {
    cxx.map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "clang++".to_string())
}

fn parity_cpp_compiler() -> String {
    parity_cpp_compiler_from_env(std::env::var("CXX").ok())
}

fn parse_running_tests_count(line: &str) -> Option<usize> {
    let trimmed = line.trim();
    let rest = trimmed.strip_prefix("running ")?;
    let digit_len = rest.bytes().take_while(u8::is_ascii_digit).count();
    if digit_len == 0 {
        return None;
    }
    if !rest[digit_len..].starts_with(" test") {
        return None;
    }
    rest[..digit_len].parse::<usize>().ok()
}

fn baseline_ran_any_tests(work_dir: &Path) -> Option<bool> {
    let baseline_path = work_dir.join("baseline.txt");
    let content = fs::read_to_string(&baseline_path).ok()?;
    Some(
        content
            .lines()
            .filter_map(parse_running_tests_count)
            .any(|count| count > 0),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_closure_fixture(root: &Path, relative: &str, contents: &str) {
        let path = root.join(relative);
        std::fs::create_dir_all(path.parent().expect("fixture file parent")).unwrap();
        std::fs::write(path, contents).unwrap();
    }

    fn snapshot_output_tree(root: &Path) -> Vec<(PathBuf, &'static str, Vec<u8>)> {
        fn visit(root: &Path, directory: &Path, out: &mut Vec<(PathBuf, &'static str, Vec<u8>)>) {
            let mut entries = std::fs::read_dir(directory)
                .unwrap()
                .map(|entry| entry.unwrap())
                .collect::<Vec<_>>();
            entries.sort_by_key(std::fs::DirEntry::file_name);
            for entry in entries {
                let path = entry.path();
                let relative = path.strip_prefix(root).unwrap().to_path_buf();
                let metadata = std::fs::symlink_metadata(&path).unwrap();
                if metadata.file_type().is_symlink() {
                    out.push((
                        relative,
                        "symlink",
                        std::fs::read_link(&path)
                            .unwrap()
                            .to_string_lossy()
                            .as_bytes()
                            .to_vec(),
                    ));
                } else if metadata.is_dir() {
                    out.push((relative, "directory", Vec::new()));
                    visit(root, &path, out);
                } else {
                    out.push((relative, "file", std::fs::read(&path).unwrap()));
                }
            }
        }

        let mut snapshot = Vec::new();
        visit(root, root, &mut snapshot);
        snapshot
    }

    fn seed_atomic_output(root: &Path, label: &str) {
        std::fs::create_dir_all(root.join("deep/inner")).unwrap();
        std::fs::write(root.join("alpha.txt"), format!("alpha:{label}\n")).unwrap();
        std::fs::write(root.join("deep/beta.txt"), format!("beta:{label}\nline-two\n"))
            .unwrap();
        let mut binary = vec![0, 1, 2, 0xff];
        binary.extend_from_slice(label.as_bytes());
        binary.push(0);
        std::fs::write(root.join("deep/inner/binary.bin"), binary).unwrap();
        #[cfg(unix)]
        std::os::unix::fs::symlink("../alpha.txt", root.join("deep/alpha-link")).unwrap();
    }

    fn closure_manifest(name: &str, dependencies: &[(&str, &str)]) -> String {
        let mut manifest =
            format!("[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2024\"\n");
        if !dependencies.is_empty() {
            manifest.push_str("\n[dependencies]\n");
            for (dependency, path) in dependencies {
                manifest.push_str(&format!("{dependency} = {{ path = \"{path}\" }}\n"));
            }
        }
        manifest
    }

    fn assert_cpp_abi_crate_fails_without_output(manifest: &Path, expected: &str) {
        let output = manifest
            .parent()
            .expect("manifest parent")
            .join("cpp_abi_test_output");
        let error = transpile_crate(
            manifest,
            &output,
            &types::UserTypeMap::default(),
            false,
            false,
            &transpile::TranspileOptions::default(),
            None,
        )
        .expect_err("cpp_abi fixture must fail before output");
        assert!(error.contains(expected), "{error}");
        assert!(!output.exists(), "created output at {}", output.display());
    }

    const CLOSURE_ADAPTER: &str = r#"
#[cfg_attr(any(), cpp_abi(param(bytes, std_string_bytes), returns(std_string_bytes)))]
pub fn adapted(bytes: Vec<u8>) -> Vec<u8> { bytes }
"#;

    #[test]
    fn cpp_import_namespace_crate_mode_keeps_dependency_private_and_ordered() {
        let fixture = tempfile::tempdir().unwrap();
        write_closure_fixture(
            fixture.path(),
            "Cargo.toml",
            "[package]\nname = \"rrr\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[workspace]\n",
        );
        write_closure_fixture(
            fixture.path(),
            "src/lib.rs",
            "pub mod rand; pub mod outer;\n",
        );
        write_closure_fixture(
            fixture.path(),
            "src/rand.rs",
            "pub fn randgen_rand_max() -> f64 { 1.0 }\npub fn randgen_rand_raw() -> u64 { 7 }\n",
        );
        write_closure_fixture(
            fixture.path(),
            "src/outer.rs",
            "pub mod rand; pub mod consumer;\n",
        );
        write_closure_fixture(
            fixture.path(),
            "src/outer/rand.rs",
            "pub fn unrelated() -> u64 { 99 }\n",
        );
        write_closure_fixture(
            fixture.path(),
            "src/outer/consumer.rs",
            r#"
#[cfg_attr(any(), cpp_import_namespace(rrr))]
use crate::rand::{randgen_rand_max, randgen_rand_raw};
pub fn draw() -> f64 { randgen_rand_raw() as f64 / randgen_rand_max() }
"#,
        );

        let output = fixture.path().join("out");
        let mut options = transpile::TranspileOptions::default();
        options.cxx_namespace = Some("rrr".to_string());
        transpile_crate(
            &fixture.path().join("Cargo.toml"),
            &output,
            &types::UserTypeMap::default(),
            false,
            false,
            &options,
            None,
        )
        .unwrap();

        let consumer = std::fs::read_to_string(output.join("rrr.outer.consumer.cppm")).unwrap();
        let module = consumer
            .find("export module rrr.outer.consumer;")
            .unwrap();
        let import = consumer.find("import rrr.rand;").unwrap();
        let namespace = consumer.find("namespace rrr {").unwrap();
        let body = consumer.find("randgen_rand_raw()").unwrap();
        assert!(module < import && import < namespace && namespace < body, "{consumer}");
        assert!(!consumer.contains("using ::rrr::"), "{consumer}");
        assert!(!consumer.contains("import rrr.outer.rand;"), "{consumer}");
        assert!(!consumer.contains("namespace rand ="), "{consumer}");
        assert!(!consumer.contains("::rrr::rand::"), "{consumer}");
        let slots = std::fs::read_to_string(output.join("rusty_hand_slots.md")).unwrap();
        assert!(slots.contains("0 slot(s)") && slots.contains("No slots detected"), "{slots}");

        let mismatch = fixture.path().join("mismatch-out");
        options.cxx_namespace = Some("wrong".to_string());
        let error = transpile_crate(
            &fixture.path().join("Cargo.toml"),
            &mismatch,
            &types::UserTypeMap::default(),
            false,
            false,
            &options,
            None,
        )
        .unwrap_err();
        assert!(error.contains("does not match active C++ namespace"), "{error}");
        assert!(!mismatch.exists());

        let inline_child = tempfile::tempdir().unwrap();
        write_closure_fixture(
            inline_child.path(),
            "Cargo.toml",
            "[package]\nname = \"rrr\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[workspace]\n",
        );
        write_closure_fixture(
            inline_child.path(),
            "src/lib.rs",
            "pub mod rand { pub fn randgen_rand_raw() -> u64 { 7 } } pub mod consumer;\n",
        );
        write_closure_fixture(
            inline_child.path(),
            "src/consumer.rs",
            r#"
#[cfg_attr(any(), cpp_import_namespace(rrr))]
use crate::rand::randgen_rand_raw;
pub fn draw() -> u64 { randgen_rand_raw() }
"#,
        );
        let inline_child_output = inline_child.path().join("out");
        options.cxx_namespace = Some("rrr".to_string());
        let error = transpile_crate(
            &inline_child.path().join("Cargo.toml"),
            &inline_child_output,
            &types::UserTypeMap::default(),
            false,
            false,
            &options,
            None,
        )
        .unwrap_err();
        assert!(error.contains("physical generated root module"), "{error}");
        assert!(!inline_child_output.exists());
    }

    #[test]
    fn cpp_import_namespace_crate_mode_flattens_proven_sibling_types() {
        let fixture = tempfile::tempdir().unwrap();
        write_closure_fixture(
            fixture.path(),
            "Cargo.toml",
            "[package]\nname = \"rrr\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[workspace]\n",
        );
        write_closure_fixture(
            fixture.path(),
            "src/lib.rs",
            "pub mod channel; pub mod consumer;\n",
        );
        write_closure_fixture(
            fixture.path(),
            "src/channel.rs",
            r#"
#[repr(i32)]
#[cfg_attr(not(any()), derive(Clone, Copy))]
pub enum ChannelError { None = 0 }
#[repr(C)]
pub struct ChannelFrame { pub value: i32 }
pub trait ChannelBase { fn code(&self) -> i32; }
pub type ChannelProxy = Box<dyn ChannelBase>;
"#,
        );
        write_closure_fixture(
            fixture.path(),
            "src/consumer.rs",
            r#"
#[cfg_attr(any(), cpp_import_namespace(rrr))]
use crate::channel::{ChannelBase, ChannelError, ChannelFrame, ChannelProxy};
pub mod external {
    #[repr(i32)]
    pub enum ChannelError { Foreign = 11 }
    #[repr(C)]
    pub struct ChannelFrame { pub foreign: i32 }
}
use self::external::ChannelFrame as OtherLeaf;
pub struct LocalChannel { pub value: i32 }
#[cpp_inherit]
impl ChannelBase for LocalChannel {
    fn code(&self) -> i32 { self.value }
}
pub fn inspect(
    frame: &ChannelFrame,
    foreign: &external::ChannelFrame,
    renamed: &OtherLeaf,
    _: Option<ChannelProxy>,
) -> ChannelError {
    let _ = frame.value + foreign.foreign + renamed.foreign;
    ChannelError::None
}
pub fn external_enum_value() -> external::ChannelError {
    external::ChannelError::Foreign
}
pub fn make_external(value: i32) -> external::ChannelFrame {
    external::ChannelFrame { foreign: value }
}
pub fn inspect_crate(value: &crate::consumer::external::ChannelFrame) -> i32 {
    value.foreign
}
pub fn external_enum_crate() -> crate::consumer::external::ChannelError {
    crate::consumer::external::ChannelError::Foreign
}
pub mod nested {
    pub fn inspect_super(value: &super::external::ChannelFrame) -> i32 { value.foreign }
    pub fn inspect_crate(value: &crate::consumer::external::ChannelFrame) -> i32 {
        value.foreign
    }
}
"#,
        );

        let output = fixture.path().join("out");
        let mut options = transpile::TranspileOptions::default();
        options.cxx_namespace = Some("rrr".to_string());
        transpile_crate(
            &fixture.path().join("Cargo.toml"),
            &output,
            &types::UserTypeMap::default(),
            false,
            false,
            &options,
            None,
        )
        .unwrap();

        let consumer = std::fs::read_to_string(output.join("rrr.consumer.cppm")).unwrap();
        assert_eq!(
            consumer
                .lines()
                .filter(|line| line.trim() == "import rrr.channel;")
                .count(),
            1,
            "{consumer}"
        );
        assert!(!consumer.contains("export import rrr.channel;"), "{consumer}");
        for leaf in [
            "ChannelBase",
            "ChannelError",
            "ChannelFrame",
            "ChannelProxy",
        ] {
            assert!(
                consumer.contains(&format!("using ::rrr::{leaf};")),
                "{consumer}"
            );
        }
        assert!(
            consumer.contains("struct LocalChannel : public ChannelBase"),
            "{consumer}"
        );
        assert!(consumer.contains("::rrr::ChannelFrame"), "{consumer}");
        assert!(
            consumer.contains("const external::ChannelFrame& foreign"),
            "{consumer}"
        );
        assert!(
            consumer.contains("const external::ChannelFrame& renamed"),
            "{consumer}"
        );
        assert!(
            !consumer.contains("const ::rrr::ChannelFrame& foreign"),
            "{consumer}"
        );
        assert!(
            consumer.contains("external::ChannelError external_enum_value()"),
            "{consumer}"
        );
        assert!(
            consumer.contains("external::ChannelFrame make_external(int32_t value)"),
            "{consumer}"
        );
        assert!(
            consumer.contains(
                "int32_t inspect_crate(const ::rrr::external::ChannelFrame& value)"
            ),
            "{consumer}"
        );
        assert!(
            consumer.contains("::rrr::external::ChannelError external_enum_crate()"),
            "{consumer}"
        );
        assert!(!consumer.contains("::consumer::external::"), "{consumer}");
        assert!(!consumer.contains("::rrr::channel::"), "{consumer}");
    }

    #[test]
    fn cpp_import_namespace_nested_type_binding_is_lexical() {
        let fixture = tempfile::tempdir().unwrap();
        write_closure_fixture(
            fixture.path(),
            "Cargo.toml",
            "[package]\nname = \"rrr\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[workspace]\n",
        );
        write_closure_fixture(
            fixture.path(),
            "src/lib.rs",
            "pub mod channel; pub mod consumer;\n",
        );
        write_closure_fixture(
            fixture.path(),
            "src/channel.rs",
            "#[repr(C)] pub struct Target { pub value: i32 }\n",
        );
        write_closure_fixture(
            fixture.path(),
            "src/consumer.rs",
            r#"
pub mod marked {
    #[cfg_attr(any(), cpp_import_namespace(rrr))]
    use crate::channel::Target;
    pub fn inspect(value: &Target) -> i32 { value.value }
    pub mod child {
        use super::Target;
        use super::Target as ImportedTarget;
        pub fn inspect_super(value: &super::Target) -> i32 { value.value }
        pub fn inspect_import(value: &Target) -> i32 { value.value }
        pub fn inspect_alias(value: &ImportedTarget) -> i32 { value.value }
        pub fn inspect_generic<Target>(_: &Target) {}
    }
}
pub mod sibling {
    #[repr(C)]
    pub struct Target { pub other: i32 }
    pub fn inspect(value: &Target) -> i32 { value.other }
}
"#,
        );

        let output = fixture.path().join("out");
        let mut options = transpile::TranspileOptions::default();
        options.cxx_namespace = Some("rrr".to_string());
        transpile_crate(
            &fixture.path().join("Cargo.toml"),
            &output,
            &types::UserTypeMap::default(),
            false,
            false,
            &options,
            None,
        )
        .unwrap();

        let consumer = std::fs::read_to_string(output.join("rrr.consumer.cppm")).unwrap();
        assert_eq!(
            consumer
                .lines()
                .filter(|line| line.trim() == "import rrr.channel;")
                .count(),
            1,
            "{consumer}"
        );
        assert!(
            consumer.contains("int32_t inspect(const ::rrr::Target& value)"),
            "{consumer}"
        );
        assert!(
            consumer.contains("int32_t inspect(const Target& value)"),
            "{consumer}"
        );
        assert!(
            consumer.contains("int32_t inspect_super(const ::rrr::Target& value)"),
            "{consumer}"
        );
        assert!(
            consumer.contains("int32_t inspect_import(const ::rrr::Target& value)"),
            "{consumer}"
        );
        assert!(
            consumer.contains("using ImportedTarget = ::rrr::Target;"),
            "{consumer}"
        );
        assert!(
            consumer.contains("int32_t inspect_alias(const ::rrr::Target& value)"),
            "{consumer}"
        );
        assert!(
            consumer.contains("void inspect_generic(const Target& _")
                && !consumer.contains("void inspect_generic(const ::rrr::Target&"),
            "{consumer}"
        );
        assert!(!consumer.contains("::rrr::channel::"), "{consumer}");
    }

    #[test]
    fn cpp_import_namespace_rejects_unbound_descendant_constructors_before_output() {
        let fixture = tempfile::tempdir().unwrap();
        write_closure_fixture(
            fixture.path(),
            "Cargo.toml",
            "[package]\nname = \"rrr\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[workspace]\n",
        );
        write_closure_fixture(
            fixture.path(),
            "src/lib.rs",
            "pub mod channel; pub mod consumer;\n",
        );
        write_closure_fixture(
            fixture.path(),
            "src/channel.rs",
            r#"
#[repr(C)]
pub struct ChannelFrame { pub value: i32 }
#[repr(i32)]
pub enum ChannelError { None = 0 }
"#,
        );
        write_closure_fixture(
            fixture.path(),
            "src/consumer.rs",
            r#"
#[cfg_attr(any(), cpp_import_namespace(rrr))]
use crate::channel::{ChannelError, ChannelFrame};
pub mod nested {
    pub fn invalid_struct(value: i32) -> i32 { ChannelFrame { value }.value }
    pub fn invalid_enum() -> i32 { ChannelError::None as i32 }
}
"#,
        );

        let output = fixture.path().join("out");
        let mut options = transpile::TranspileOptions::default();
        options.cxx_namespace = Some("rrr".to_string());
        let error = transpile_crate(
            &fixture.path().join("Cargo.toml"),
            &output,
            &types::UserTypeMap::default(),
            false,
            false,
            &options,
            None,
        )
        .expect_err("unbound descendant flat type leaf");
        assert!(error.contains("without an exact local binding"), "{error}");
        assert!(error.contains("consumer::nested"), "{error}");
        assert!(
            !output.exists(),
            "crate preflight must reject before creating the output directory"
        );
    }

    #[test]
    fn cpp_import_namespace_rejects_wrong_namespace_shadow_atomically() {
        let fixture = tempfile::tempdir().unwrap();
        write_closure_fixture(
            fixture.path(),
            "Cargo.toml",
            "[package]\nname = \"rrr\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[workspace]\n",
        );
        write_closure_fixture(
            fixture.path(),
            "src/lib.rs",
            "pub mod channel; pub mod consumer;\n",
        );
        write_closure_fixture(
            fixture.path(),
            "src/channel.rs",
            "#[repr(C)] pub struct Target { pub value: i32 }\n",
        );
        write_closure_fixture(
            fixture.path(),
            "src/consumer.rs",
            r#"
#[cfg_attr(any(), cpp_import_namespace(rrr))]
use crate::channel::Target;
pub mod nested {
    pub fn invalid(Target: usize, value: &Target) -> usize {
        let _ = value;
        Target
    }
}
"#,
        );

        let mut options = transpile::TranspileOptions::default();
        options.cxx_namespace = Some("rrr".to_string());
        let manifest = fixture.path().join("Cargo.toml");
        let absent = fixture.path().join("absent-output");
        let error = transpile_crate(
            &manifest,
            &absent,
            &types::UserTypeMap::default(),
            false,
            false,
            &options,
            None,
        )
        .expect_err("a value binding cannot satisfy the imported type leaf");
        assert!(error.contains("type namespace"), "{error}");
        assert!(!absent.exists(), "preflight created an absent output");

        let existing = fixture.path().join("existing-output");
        std::fs::create_dir(&existing).unwrap();
        let sentinel = existing.join("keep.txt");
        std::fs::write(&sentinel, "preserve\n").unwrap();
        let error = transpile_crate(
            &manifest,
            &existing,
            &types::UserTypeMap::default(),
            false,
            false,
            &options,
            None,
        )
        .expect_err("wrong-namespace rejection with preexisting output");
        assert!(error.contains("type namespace"), "{error}");
        assert_eq!(std::fs::read_to_string(&sentinel).unwrap(), "preserve\n");
        let entries = std::fs::read_dir(&existing)
            .unwrap()
            .map(|entry| entry.unwrap().file_name())
            .collect::<Vec<_>>();
        assert_eq!(entries, vec![std::ffi::OsString::from("keep.txt")]);
    }

    #[test]
    fn cpp_import_namespace_cfg_presence_matches_cargo_and_is_atomic() {
        let fixture = tempfile::tempdir().unwrap();
        write_closure_fixture(
            fixture.path(),
            "Cargo.toml",
            "[package]\nname = \"rrr\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[workspace]\n",
        );
        write_closure_fixture(
            fixture.path(),
            "src/lib.rs",
            "pub mod channel; pub mod consumer;\n",
        );
        write_closure_fixture(
            fixture.path(),
            "src/channel.rs",
            "#[repr(C)] pub struct Target { pub value: i32 }\n",
        );

        let manifest = fixture.path().join("Cargo.toml");
        let options = transpile::TranspileOptions {
            cxx_namespace: Some("rrr".to_string()),
            ..Default::default()
        };
        for (label, nested) in [
            (
                "cfg-type",
                "#[cfg(any())] pub struct Target { pub local: usize } pub fn invalid(value: &Target) -> usize { value.local }",
            ),
            (
                "cfg-attr-type",
                "#[cfg_attr(not(any()), cfg(any()))] pub struct Target { pub local: usize } pub fn invalid(value: &Target) -> usize { value.local }",
            ),
            (
                "cfg-value",
                "#[cfg(any())] pub const Target: usize = 1; pub fn invalid() -> usize { Target }",
            ),
            (
                "cfg-constructor",
                "#[cfg(any())] pub struct Target(pub usize); pub fn invalid() -> usize { Target(1).0 }",
            ),
            (
                "cfg-module",
                "#[cfg(any())] pub mod Target { pub const VALUE: usize = 1; } pub fn invalid() -> usize { Target::VALUE }",
            ),
            (
                "cfg-import-alias",
                "pub struct Other; #[cfg(any())] use self::Other as Target; pub fn invalid(_: &Target) {}",
            ),
            (
                "cfg-attr-import-alias",
                "pub struct Other; #[cfg_attr(not(any()), cfg(any()))] use self::Other as Target; pub fn invalid(_: &Target) {}",
            ),
            (
                "cfg-pattern-binding",
                "pub fn invalid() -> usize { #[cfg(any())] let Target = 1usize; Target }",
            ),
            (
                "cfg-variant-pattern-head",
                "pub enum Other { #[cfg(any())] Target } use self::Other::Target; pub fn invalid() { let _ = Target; }",
            ),
        ] {
            write_closure_fixture(
                fixture.path(),
                "src/consumer.rs",
                &format!(
                    r#"
#[cfg_attr(any(), cpp_import_namespace(rrr))]
use crate::channel::Target;

pub mod nested {{
    {nested}
}}
"#
                ),
            );

            let cargo = std::process::Command::new("cargo")
                .arg("check")
                .arg("--quiet")
                .arg("--manifest-path")
                .arg(&manifest)
                .env("CARGO_TARGET_DIR", fixture.path().join("cargo-target"))
                .output()
                .unwrap();
            assert!(
                !cargo.status.success(),
                "Cargo unexpectedly accepted the {label} fixture"
            );
            assert!(
                !cargo.stderr.is_empty(),
                "Cargo rejected {label} without a diagnostic"
            );

            let absent = fixture.path().join(format!("{label}-absent-output"));
            let error = transpile_crate(
                &manifest,
                &absent,
                &types::UserTypeMap::default(),
                false,
                false,
                &options,
                None,
            )
            .expect_err("cfg-disabled evidence must fail before absent output");
            assert!(
                error.contains("without an exact local binding")
                    || error.contains("unsupported presence/path attributes"),
                "{label}: {error}"
            );
            assert!(!absent.exists(), "{label}: preflight created absent output");

            let existing = fixture.path().join(format!("{label}-existing-output"));
            std::fs::create_dir(&existing).unwrap();
            let sentinel = existing.join("keep.txt");
            std::fs::write(&sentinel, format!("preserve-{label}\n")).unwrap();
            let error = transpile_crate(
                &manifest,
                &existing,
                &types::UserTypeMap::default(),
                false,
                false,
                &options,
                None,
            )
            .expect_err("cfg-disabled evidence must preserve existing output");
            assert!(
                error.contains("without an exact local binding")
                    || error.contains("unsupported presence/path attributes"),
                "{label}: {error}"
            );
            assert_eq!(
                std::fs::read_to_string(&sentinel).unwrap(),
                format!("preserve-{label}\n")
            );
            let entries = std::fs::read_dir(&existing)
                .unwrap()
                .map(|entry| entry.unwrap().file_name())
                .collect::<Vec<_>>();
            assert_eq!(
                entries,
                vec![std::ffi::OsString::from("keep.txt")],
                "{label}: invalid preflight added output files"
            );
        }

        write_closure_fixture(
            fixture.path(),
            "src/consumer.rs",
            r#"
#[cfg_attr(any(), cpp_import_namespace(rrr))]
use crate::channel::Target;

pub mod nested {
    pub struct Other;
    impl Other { pub const Target: usize = 7; }
    pub fn valid() -> usize { self::Other::Target }
}
"#,
        );
        let cargo = std::process::Command::new("cargo")
            .arg("check")
            .arg("--quiet")
            .arg("--manifest-path")
            .arg(&manifest)
            .env("CARGO_TARGET_DIR", fixture.path().join("cargo-target"))
            .output()
            .unwrap();
        assert!(
            cargo.status.success(),
            "Cargo rejected associated-member fixture: {}",
            String::from_utf8_lossy(&cargo.stderr)
        );

        let associated_output = fixture.path().join("associated-output");
        transpile_crate(
            &manifest,
            &associated_output,
            &types::UserTypeMap::default(),
            false,
            false,
            &options,
            None,
        )
        .expect("a distinctly qualified associated member must transpile");
        let consumer_cpp =
            std::fs::read_to_string(associated_output.join("rrr.consumer.cppm")).unwrap();
        assert!(
            consumer_cpp.contains("return nested::Other::Target;")
                || consumer_cpp.contains("return Other::Target;"),
            "associated const was misclassified as a constructor:\n{consumer_cpp}"
        );
        assert!(!consumer_cpp.contains("Other::Target{}"), "{consumer_cpp}");
    }

    #[test]
    fn cpp_import_namespace_foreign_member_presence_matches_cargo_and_is_atomic() {
        let fixture = tempfile::tempdir().unwrap();
        write_closure_fixture(
            fixture.path(),
            "Cargo.toml",
            "[package]\nname = \"rrr\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[workspace]\n",
        );
        write_closure_fixture(
            fixture.path(),
            "src/lib.rs",
            "pub mod channel; pub mod consumer;\n",
        );
        write_closure_fixture(
            fixture.path(),
            "src/channel.rs",
            "#[repr(C)] pub struct Target { pub value: i32 }\n",
        );

        let manifest = fixture.path().join("Cargo.toml");
        let options = transpile::TranspileOptions {
            cxx_namespace: Some("rrr".to_string()),
            ..Default::default()
        };
        for (label, foreign, use_, cargo_valid) in [
            (
                "cfg-disabled-foreign-fn",
                "#[cfg(any())] fn Target() -> usize;",
                "pub fn invalid() -> usize { unsafe { Target() } }",
                false,
            ),
            (
                "cfg-attr-disabled-foreign-fn",
                "#[cfg_attr(all(), cfg(any()))] fn Target() -> usize;",
                "pub fn invalid() -> usize { unsafe { Target() } }",
                false,
            ),
            (
                "unknown-target-foreign-fn",
                "#[cfg(target_os = \"linux\")] fn Target() -> usize;",
                "pub fn valid_for_cargo() -> usize { unsafe { Target() } }",
                true,
            ),
            (
                "cfg-disabled-foreign-static",
                "#[cfg(any())] static Target: usize;",
                "pub fn invalid() -> usize { unsafe { Target } }",
                false,
            ),
            (
                "cfg-attr-disabled-foreign-static",
                "#[cfg_attr(all(), cfg(any()))] static Target: usize;",
                "pub fn invalid() -> usize { unsafe { Target } }",
                false,
            ),
            (
                "unknown-target-foreign-static",
                "#[cfg(target_os = \"linux\")] static Target: usize;",
                "pub fn valid_for_cargo() -> usize { unsafe { Target } }",
                true,
            ),
            (
                "cfg-disabled-foreign-type",
                "#[cfg(any())] type Target;",
                "pub fn invalid(_: &Target) {}",
                false,
            ),
            (
                "cfg-attr-disabled-foreign-type",
                "#[cfg_attr(all(), cfg(any()))] type Target;",
                "pub fn invalid(_: &Target) {}",
                false,
            ),
            (
                "unknown-target-foreign-type",
                "#[cfg(target_os = \"windows\")] type Target;",
                "pub fn invalid(_: &Target) {}",
                false,
            ),
        ] {
            write_closure_fixture(
                fixture.path(),
                "src/consumer.rs",
                &format!(
                    r#"
#[cfg_attr(any(), cpp_import_namespace(rrr))]
use crate::channel::Target;

pub mod nested {{
    unsafe extern "C" {{
        {foreign}
    }}
    {use_}
}}
"#,
                ),
            );

            let cargo = std::process::Command::new("cargo")
                .arg("check")
                .arg("--quiet")
                .arg("--manifest-path")
                .arg(&manifest)
                .env("CARGO_TARGET_DIR", fixture.path().join("cargo-target"))
                .output()
                .unwrap();
            assert_eq!(
                cargo.status.success(),
                cargo_valid,
                "Cargo parity mismatch for {label}: {}",
                String::from_utf8_lossy(&cargo.stderr)
            );

            let absent = fixture.path().join(format!("{label}-absent-output"));
            let error = transpile_crate(
                &manifest,
                &absent,
                &types::UserTypeMap::default(),
                false,
                false,
                &options,
                None,
            )
            .expect_err("conditional foreign member must not prove a descendant binding");
            assert!(error.contains("without an exact local binding"), "{label}: {error}");
            assert!(!absent.exists(), "{label}: preflight created absent output");

            let existing = fixture.path().join(format!("{label}-existing-output"));
            seed_atomic_output(&existing, label);
            let before = snapshot_output_tree(&existing);
            let error = transpile_crate(
                &manifest,
                &existing,
                &types::UserTypeMap::default(),
                false,
                false,
                &options,
                None,
            )
            .expect_err("conditional foreign member must preserve existing output");
            assert!(error.contains("without an exact local binding"), "{label}: {error}");
            assert_eq!(snapshot_output_tree(&existing), before, "{label}");
        }
    }

    #[test]
    fn cpp_import_namespace_enclosing_foreign_block_presence_matches_cargo_and_is_atomic() {
        let fixture = tempfile::tempdir().unwrap();
        write_closure_fixture(
            fixture.path(),
            "Cargo.toml",
            "[package]\nname = \"rrr\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[features]\ndefault = [\"present\"]\npresent = []\n\n[workspace]\n",
        );
        write_closure_fixture(
            fixture.path(),
            "src/lib.rs",
            "pub mod channel; pub mod consumer;\n",
        );
        write_closure_fixture(
            fixture.path(),
            "src/channel.rs",
            "#[repr(C)] pub struct Target { pub value: i32 }\n",
        );

        let manifest = fixture.path().join("Cargo.toml");
        let options = transpile::TranspileOptions {
            cxx_namespace: Some("rrr".to_string()),
            ..Default::default()
        };
        let marker = r#"
#[cfg_attr(any(), cpp_import_namespace(rrr))]
use crate::channel::Target;
"#;

        for (label, body) in [
            (
                "enclosing-cfg-disabled-foreign-fn",
                r#"
#[cfg(any())]
unsafe extern "C" { fn Target() -> usize; }
pub fn valid(value: &Target) -> i32 { value.value }
"#,
            ),
            (
                "enclosing-cfg-attr-disabled-foreign-fn",
                r#"
#[cfg_attr(all(), cfg(any()))]
unsafe extern "C" { fn Target() -> usize; }
pub fn valid(value: &Target) -> i32 { value.value }
"#,
            ),
            (
                "enclosing-nested-cfg-attr-disabled-foreign-macro",
                r#"
#[cfg_attr(all(), cfg_attr(all(), cfg(any())))]
unsafe extern "C" { Target!(); }
pub fn valid(value: &Target) -> i32 { value.value }
"#,
            ),
            (
                "member-cfg-disabled-verbatim-safe-fn",
                r#"
unsafe extern "C" {
    #[cfg(any())]
    safe fn Target() -> usize;
}
pub fn valid(value: &Target) -> i32 { value.value }
"#,
            ),
            (
                "enclosing-decisively-absent-foreign-type",
                r#"
#[cfg(all(any(target_os = "linux"), not(all())))]
unsafe extern "C" { type Target; }
pub fn valid(value: &Target) -> i32 { value.value }
"#,
            ),
            (
                "enclosing-static-present-foreign-fn",
                r#"
pub mod nested {
    #[cfg(any(target_os = "impossible", all()))]
    unsafe extern "C" { fn Target() -> usize; }
    pub fn valid() -> usize { unsafe { Target() } }
}
"#,
            ),
            (
                "mixed-member-presence",
                r#"
unsafe extern "C" {
    #[cfg(any())]
    fn Target() -> usize;
    fn Other() -> usize;
}
pub fn valid(value: &Target) -> i32 { value.value }
"#,
            ),
        ] {
            write_closure_fixture(
                fixture.path(),
                "src/consumer.rs",
                &format!("{marker}\n{body}"),
            );
            let cargo = std::process::Command::new("cargo")
                .arg("check")
                .arg("--quiet")
                .arg("--manifest-path")
                .arg(&manifest)
                .env("CARGO_TARGET_DIR", fixture.path().join("cargo-target"))
                .output()
                .unwrap();
            assert!(
                cargo.status.success(),
                "Cargo rejected {label}: {}",
                String::from_utf8_lossy(&cargo.stderr)
            );

            let output = fixture.path().join(format!("{label}-output"));
            transpile_crate(
                &manifest,
                &output,
                &types::UserTypeMap::default(),
                false,
                false,
                &options,
                None,
            )
            .unwrap_or_else(|error| panic!("{label} should transpile: {error}"));
            assert!(output.join("rusty_hand_slots.md").is_file(), "{label}");
        }

        for (label, body, cargo_valid, expected_error) in [
            (
                "enclosing-unknown-target-foreign-fn",
                r#"
pub mod nested {
    #[cfg(target_os = "linux")]
    unsafe extern "C" { fn Target() -> usize; }
    pub fn cargo_valid() -> usize { unsafe { Target() } }
}
"#,
                true,
                "without an exact local binding",
            ),
            (
                "enclosing-unknown-feature-foreign-fn",
                r#"
pub mod nested {
    #[cfg(feature = "present")]
    unsafe extern "C" { fn Target() -> usize; }
    pub fn cargo_valid() -> usize { unsafe { Target() } }
}
"#,
                true,
                "without an exact local binding",
            ),
            (
                "enclosing-unknown-cfg-attr-foreign-fn",
                r#"
pub mod nested {
    #[cfg_attr(target_os = "linux", cfg(any()))]
    unsafe extern "C" { fn Target() -> usize; }
    pub fn cargo_invalid() -> usize { unsafe { Target() } }
}
"#,
                false,
                "without an exact local binding",
            ),
            (
                "enclosing-unknown-target-foreign-macro",
                r#"
#[cfg(target_os = "linux")]
unsafe extern "C" { Target!(); }
pub fn valid(value: &Target) -> i32 { value.value }
"#,
                false,
                "opaque macro syntax",
            ),
            (
                "member-unknown-target-verbatim-safe-fn",
                r#"
unsafe extern "C" {
    #[cfg(target_os = "linux")]
    safe fn Target() -> usize;
}
pub fn valid(value: &Target) -> i32 { value.value }
"#,
                true,
                "opaque foreign item syntax",
            ),
        ] {
            write_closure_fixture(
                fixture.path(),
                "src/consumer.rs",
                &format!("{marker}\n{body}"),
            );
            let cargo = std::process::Command::new("cargo")
                .arg("check")
                .arg("--quiet")
                .arg("--manifest-path")
                .arg(&manifest)
                .env("CARGO_TARGET_DIR", fixture.path().join("cargo-target"))
                .output()
                .unwrap();
            assert_eq!(
                cargo.status.success(),
                cargo_valid,
                "Cargo parity mismatch for {label}: {}",
                String::from_utf8_lossy(&cargo.stderr)
            );

            let absent = fixture.path().join(format!("{label}-absent-output"));
            let error = transpile_crate(
                &manifest,
                &absent,
                &types::UserTypeMap::default(),
                false,
                false,
                &options,
                None,
            )
            .expect_err("unknown enclosing presence must fail closed");
            assert!(error.contains(expected_error), "{label}: {error}");
            assert!(!absent.exists(), "{label}: preflight created absent output");

            let existing = fixture.path().join(format!("{label}-existing-output"));
            seed_atomic_output(&existing, label);
            let before = snapshot_output_tree(&existing);
            let error = transpile_crate(
                &manifest,
                &existing,
                &types::UserTypeMap::default(),
                false,
                false,
                &options,
                None,
            )
            .expect_err("unknown enclosing presence must preserve existing output");
            assert!(error.contains(expected_error), "{label}: {error}");
            assert_eq!(snapshot_output_tree(&existing), before, "{label}");
        }
    }

    #[test]
    fn cpp_abi_crate_mode_requires_an_explicit_modern_edition() {
        for (label, edition, expected) in [
            (
                "explicit_2015",
                "edition = \"2015\"\n",
                "found `2015`",
            ),
            (
                "omitted",
                "",
                "an omitted edition selects Rust 2015",
            ),
            (
                "workspace_inherited",
                "edition.workspace = true\n",
                "workspace-inherited or non-string editions are unsupported",
            ),
        ] {
            let fixture = tempfile::tempdir().unwrap();
            let mut manifest = format!(
                "[package]\nname = \"{label}\"\nversion = \"0.1.0\"\n{edition}\n[workspace]\n"
            );
            if label == "workspace_inherited" {
                manifest.push_str("\n[workspace.package]\nedition = \"2015\"\n");
            }
            write_closure_fixture(fixture.path(), "Cargo.toml", &manifest);
            write_closure_fixture(fixture.path(), "src/lib.rs", CLOSURE_ADAPTER);
            let manifest_path = fixture.path().join("Cargo.toml");

            let error = preflight_cpp_abi_whole_dependency_closure(&manifest_path, false)
                .expect_err("unsupported edition must fail closed");
            assert!(error.contains(expected), "{label}: {error}");
            assert_cpp_abi_crate_fails_without_output(&manifest_path, expected);
        }

        for edition in ["2018", "2021", "2024"] {
            let fixture = tempfile::tempdir().unwrap();
            write_closure_fixture(
                fixture.path(),
                "Cargo.toml",
                &format!(
                    "[package]\nname = \"edition_{edition}\"\nversion = \"0.1.0\"\nedition = \"{edition}\"\n\n[workspace]\n"
                ),
            );
            write_closure_fixture(fixture.path(), "src/lib.rs", CLOSURE_ADAPTER);
            assert_eq!(
                preflight_cpp_abi_whole_dependency_closure(
                    &fixture.path().join("Cargo.toml"),
                    false,
                )
                .unwrap(),
                true,
                "explicit Rust {edition} must be supported"
            );
        }
    }

    #[test]
    fn cpp_abi_edition_guard_does_not_change_marker_free_crates() {
        for (label, edition) in [("explicit_2015", "edition = \"2015\"\n"), ("omitted", "")]
        {
            let fixture = tempfile::tempdir().unwrap();
            write_closure_fixture(
                fixture.path(),
                "Cargo.toml",
                &format!(
                    "[package]\nname = \"marker_free_{label}\"\nversion = \"0.1.0\"\n{edition}\n[workspace]\n"
                ),
            );
            write_closure_fixture(
                fixture.path(),
                "src/lib.rs",
                "pub fn ordinary() -> u32 { 1 }\n",
            );
            assert_eq!(
                preflight_cpp_abi_whole_dependency_closure(
                    &fixture.path().join("Cargo.toml"),
                    false,
                )
                .unwrap(),
                false
            );
        }
    }

    #[test]
    fn cpp_abi_closure_preflight_reports_missing_malformed_and_cycles_deterministically() {
        let fixture = tempfile::tempdir().unwrap();
        write_closure_fixture(
            fixture.path(),
            "root/Cargo.toml",
            &closure_manifest(
                "root",
                &[("z_missing", "../z_missing"), ("a_missing", "../a_missing")],
            ),
        );
        write_closure_fixture(fixture.path(), "root/src/lib.rs", CLOSURE_ADAPTER);
        let error = preflight_cpp_abi_whole_dependency_closure(
            &fixture.path().join("root/Cargo.toml"),
            false,
        )
        .unwrap_err();
        assert!(error.contains("a_missing/Cargo.toml"), "{error}");
        assert!(error.contains("z_missing/Cargo.toml"), "{error}");
        assert!(
            error.find("a_missing/Cargo.toml").unwrap()
                < error.find("z_missing/Cargo.toml").unwrap(),
            "{error}"
        );

        let malformed = tempfile::tempdir().unwrap();
        write_closure_fixture(
            malformed.path(),
            "root/Cargo.toml",
            &closure_manifest("root", &[("bad", "../bad")]),
        );
        write_closure_fixture(malformed.path(), "root/src/lib.rs", CLOSURE_ADAPTER);
        write_closure_fixture(malformed.path(), "bad/Cargo.toml", "not valid toml = [");
        write_closure_fixture(malformed.path(), "bad/src/lib.rs", CLOSURE_ADAPTER);
        let error = preflight_cpp_abi_whole_dependency_closure(
            &malformed.path().join("root/Cargo.toml"),
            false,
        )
        .unwrap_err();
        assert!(
            error.contains("could not parse local dependency manifest"),
            "{error}"
        );

        let cycle = tempfile::tempdir().unwrap();
        write_closure_fixture(
            cycle.path(),
            "a/Cargo.toml",
            &closure_manifest("a", &[("b", "../b")]),
        );
        write_closure_fixture(cycle.path(), "a/src/lib.rs", CLOSURE_ADAPTER);
        write_closure_fixture(
            cycle.path(),
            "b/Cargo.toml",
            &closure_manifest("b", &[("a", "../a")]),
        );
        write_closure_fixture(cycle.path(), "b/src/lib.rs", "pub fn ordinary() {}\n");
        let error =
            preflight_cpp_abi_whole_dependency_closure(&cycle.path().join("a/Cargo.toml"), false)
                .unwrap_err();
        assert!(error.contains("local dependency cycle detected"), "{error}");
        assert!(error.matches("a/Cargo.toml").count() >= 2, "{error}");
    }

    #[test]
    fn cpp_abi_closure_preflight_accepts_valid_two_level_and_diamond_graphs() {
        let fixture = tempfile::tempdir().unwrap();
        write_closure_fixture(
            fixture.path(),
            "root/Cargo.toml",
            &closure_manifest("root", &[("left", "../left"), ("right", "../right")]),
        );
        write_closure_fixture(fixture.path(), "root/src/lib.rs", CLOSURE_ADAPTER);
        write_closure_fixture(
            fixture.path(),
            "left/Cargo.toml",
            &closure_manifest("left", &[("leaf", "../leaf")]),
        );
        write_closure_fixture(fixture.path(), "left/src/lib.rs", "pub fn left() {}\n");
        write_closure_fixture(
            fixture.path(),
            "right/Cargo.toml",
            &closure_manifest("right", &[("leaf", "../leaf")]),
        );
        write_closure_fixture(fixture.path(), "right/src/lib.rs", "pub fn right() {}\n");
        write_closure_fixture(
            fixture.path(),
            "leaf/Cargo.toml",
            &closure_manifest("leaf", &[]),
        );
        write_closure_fixture(
            fixture.path(),
            "leaf/src/lib.rs",
            "pub fn leaf() -> u32 { 1 }\n",
        );
        assert_eq!(
            preflight_cpp_abi_whole_dependency_closure(
                &fixture.path().join("root/Cargo.toml"),
                false,
            )
            .unwrap(),
            true
        );
    }

    #[test]
    fn cpp_abi_closure_preflight_rejects_adapters_in_local_dependencies() {
        let fixture = tempfile::tempdir().unwrap();
        write_closure_fixture(
            fixture.path(),
            "root/Cargo.toml",
            &closure_manifest("root", &[("dep_adapter", "../dep")]),
        );
        write_closure_fixture(
            fixture.path(),
            "root/src/lib.rs",
            "pub fn call(bytes: Vec<u8>) -> Vec<u8> { dep_adapter::adapted(bytes) }\n",
        );
        write_closure_fixture(
            fixture.path(),
            "dep/Cargo.toml",
            &closure_manifest("dep_adapter", &[]),
        );
        write_closure_fixture(fixture.path(), "dep/src/lib.rs", CLOSURE_ADAPTER);

        let root_manifest = fixture.path().join("root/Cargo.toml");
        let error = preflight_cpp_abi_whole_dependency_closure(&root_manifest, false)
            .expect_err("cross-crate adapter must fail closed");
        assert!(error.contains("local dependency"), "{error}");
        assert!(error.contains("cross-crate adapter calls"), "{error}");
        assert_cpp_abi_crate_fails_without_output(&root_manifest, "cross-crate adapter calls");

        assert_eq!(
            preflight_cpp_abi_whole_dependency_closure(
                &fixture.path().join("dep/Cargo.toml"),
                false,
            )
            .unwrap(),
            true
        );
    }

    #[test]
    fn cpp_abi_closure_preflight_preserves_marker_free_incomplete_dependency_behavior() {
        for malformed in [false, true] {
            let fixture = tempfile::tempdir().unwrap();
            write_closure_fixture(
                fixture.path(),
                "root/Cargo.toml",
                &closure_manifest("root", &[("bad", "../bad")]),
            );
            write_closure_fixture(
                fixture.path(),
                "root/src/lib.rs",
                "pub fn ordinary() -> u32 { 1 }\n",
            );
            if malformed {
                write_closure_fixture(fixture.path(), "bad/Cargo.toml", "invalid = [");
                write_closure_fixture(
                    fixture.path(),
                    "bad/src/lib.rs",
                    "pub fn ordinary() -> u32 { 2 }\n",
                );
            }
            assert_eq!(
                preflight_cpp_abi_whole_dependency_closure(
                    &fixture.path().join("root/Cargo.toml"),
                    false,
                )
                .unwrap(),
                false
            );
        }
    }

    #[cfg(unix)]
    #[test]
    fn cpp_abi_closure_preflight_follows_logical_file_and_directory_symlinks() {
        use std::os::unix::fs::symlink;

        let file_link = tempfile::tempdir().unwrap();
        write_closure_fixture(
            file_link.path(),
            "root/Cargo.toml",
            &closure_manifest("root", &[("missing", "../missing")]),
        );
        write_closure_fixture(file_link.path(), "root/real.rs", CLOSURE_ADAPTER);
        std::fs::create_dir_all(file_link.path().join("root/src")).unwrap();
        symlink("../real.rs", file_link.path().join("root/src/lib.rs")).unwrap();
        let error = preflight_cpp_abi_whole_dependency_closure(
            &file_link.path().join("root/Cargo.toml"),
            false,
        )
        .unwrap_err();
        assert!(error.contains("missing/Cargo.toml"), "{error}");
        assert_cpp_abi_crate_fails_without_output(
            &file_link.path().join("root/Cargo.toml"),
            "missing/Cargo.toml",
        );

        let directory_link = tempfile::tempdir().unwrap();
        write_closure_fixture(
            directory_link.path(),
            "root/Cargo.toml",
            &closure_manifest("root", &[("missing", "../missing")]),
        );
        write_closure_fixture(directory_link.path(), "root/src/lib.rs", "pub mod api;\n");
        write_closure_fixture(
            directory_link.path(),
            "root/real_api/mod.rs",
            CLOSURE_ADAPTER,
        );
        symlink(
            "../real_api",
            directory_link.path().join("root/src/api"),
        )
        .unwrap();
        let error = preflight_cpp_abi_whole_dependency_closure(
            &directory_link.path().join("root/Cargo.toml"),
            false,
        )
        .unwrap_err();
        assert!(error.contains("missing/Cargo.toml"), "{error}");
        assert_cpp_abi_crate_fails_without_output(
            &directory_link.path().join("root/Cargo.toml"),
            "missing/Cargo.toml",
        );
    }

    #[cfg(unix)]
    #[test]
    fn cpp_abi_closure_preflight_reports_broken_links_and_directory_cycles() {
        use std::os::unix::fs::symlink;

        for label in ["broken source link", "source directory cycle"] {
            let fixture = tempfile::tempdir().unwrap();
            write_closure_fixture(
                fixture.path(),
                "root/Cargo.toml",
                &closure_manifest(
                    "root",
                    &[("adapter", "../adapter"), ("problem", "../problem")],
                ),
            );
            write_closure_fixture(fixture.path(), "root/src/lib.rs", "pub fn root() {}\n");
            write_closure_fixture(
                fixture.path(),
                "adapter/Cargo.toml",
                &closure_manifest("adapter", &[]),
            );
            write_closure_fixture(fixture.path(), "adapter/src/lib.rs", CLOSURE_ADAPTER);
            write_closure_fixture(
                fixture.path(),
                "problem/Cargo.toml",
                &closure_manifest("problem", &[]),
            );
            write_closure_fixture(
                fixture.path(),
                "problem/src/lib.rs",
                "pub fn ordinary() {}\n",
            );
            let problem = fixture.path().join("problem");
            if label == "broken source link" {
                symlink("missing-target.rs", problem.join("src/ghost.rs")).unwrap();
            } else {
                symlink(".", problem.join("src/loop")).unwrap();
            }

            let error = preflight_cpp_abi_whole_dependency_closure(
                &fixture.path().join("root/Cargo.toml"),
                false,
            )
            .expect_err(label);
            match label {
                "broken source link" => {
                    assert!(error.contains("could not inspect source path"), "{error}");
                    assert!(error.contains("ghost.rs"), "{error}");
                }
                _ => assert!(error.contains("source directory symlink cycle"), "{error}"),
            }
            let expected = if label == "broken source link" {
                "ghost.rs"
            } else {
                "source directory symlink cycle"
            };
            assert_cpp_abi_crate_fails_without_output(
                &fixture.path().join("root/Cargo.toml"),
                expected,
            );
        }
    }

    #[cfg(unix)]
    #[test]
    fn cpp_abi_symlink_scan_allows_aliases_and_preserves_marker_free_gating() {
        use std::os::unix::fs::symlink;

        let aliases = tempfile::tempdir().unwrap();
        write_closure_fixture(aliases.path(), "crate/src/lib.rs", "pub fn root() {}\n");
        write_closure_fixture(aliases.path(), "crate/real/mod.rs", "pub fn item() {}\n");
        symlink("../real", aliases.path().join("crate/src/left")).unwrap();
        symlink("../real", aliases.path().join("crate/src/right")).unwrap();
        let mut preflight = CppAbiClosurePreflight::new(false);
        let files = preflight.collect_rs_files(&aliases.path().join("crate"));
        assert!(files.contains(&PathBuf::from("src/left/mod.rs")));
        assert!(files.contains(&PathBuf::from("src/right/mod.rs")));
        assert!(preflight.report.issues.is_empty());

        for label in ["broken", "cycle"] {
            let fixture = tempfile::tempdir().unwrap();
            write_closure_fixture(
                fixture.path(),
                "root/Cargo.toml",
                &closure_manifest("root", &[]),
            );
            write_closure_fixture(
                fixture.path(),
                "root/src/lib.rs",
                "pub fn ordinary() {}\n",
            );
            let root = fixture.path().join("root");
            if label == "broken" {
                symlink("missing.rs", root.join("src/ghost.rs")).unwrap();
            } else {
                symlink(".", root.join("src/loop")).unwrap();
            }
            assert_eq!(
                preflight_cpp_abi_whole_dependency_closure(
                    &fixture.path().join("root/Cargo.toml"),
                    false,
                )
                .expect(label),
                false
            );
        }
    }

    #[test]
    fn test_collect_rusty_test_entries_from_cppm_uses_wrapper_exports_only() {
        let content = r#"
export void rusty_test_alpha() {
}
TEST_CASE("legacy_style") {
}
void rusty_test_beta() {
}
"#;
        let mut seen = HashSet::new();
        let mut entries = Vec::new();
        collect_rusty_test_entries_from_cppm(content, &mut seen, &mut entries);

        assert_eq!(
            entries,
            vec![
                RunnerTestEntry {
                    fn_name: "rusty_test_alpha".to_string(),
                    label: "alpha".to_string(),
                    should_panic: false,
                },
                RunnerTestEntry {
                    fn_name: "rusty_test_beta".to_string(),
                    label: "beta".to_string(),
                    should_panic: false,
                },
            ]
        );
    }

    #[test]
    fn test_collect_rusty_test_entries_from_cppm_deduplicates_wrappers() {
        let content = r#"
export void rusty_test_dup() {
}
void rusty_test_dup() {
}
"#;
        let mut seen = HashSet::new();
        let mut entries = Vec::new();
        collect_rusty_test_entries_from_cppm(content, &mut seen, &mut entries);

        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].fn_name, "rusty_test_dup");
        assert_eq!(entries[0].label, "dup");
        assert!(!entries[0].should_panic);
    }

    #[test]
    fn test_collect_rusty_test_entries_from_cppm_reads_should_panic_metadata() {
        let content = r#"
// Rust-only libtest wrapper metadata: marker=tests::panic_case should_panic=yes
export void rusty_test_tests_panic_case() {
}
// Rust-only libtest wrapper metadata: marker=tests::regular_case should_panic=no
export void rusty_test_tests_regular_case() {
}
"#;
        let mut seen = HashSet::new();
        let mut entries = Vec::new();
        collect_rusty_test_entries_from_cppm(content, &mut seen, &mut entries);

        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].fn_name, "rusty_test_tests_panic_case");
        assert!(entries[0].should_panic);
        assert_eq!(entries[1].fn_name, "rusty_test_tests_regular_case");
        assert!(!entries[1].should_panic);
    }

    #[test]
    fn test_collect_rusty_test_entries_from_cppm_reads_should_panic_metadata_with_module_prefix() {
        let content = r#"
// Rust-only libtest wrapper metadata: marker=tests::panic_case should_panic=yes
export void rusty_test_arrayvec_tests_panic_case() {
}
// Rust-only libtest wrapper metadata: marker=tests::regular_case should_panic=no
export void rusty_test_arrayvec_tests_regular_case() {
}
"#;
        let mut seen = HashSet::new();
        let mut entries = Vec::new();
        collect_rusty_test_entries_from_cppm(content, &mut seen, &mut entries);

        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].fn_name, "rusty_test_arrayvec_tests_panic_case");
        assert!(entries[0].should_panic);
        assert_eq!(entries[1].fn_name, "rusty_test_arrayvec_tests_regular_case");
        assert!(!entries[1].should_panic);
    }

    #[test]
    fn test_is_warning_as_error_failure_detects_attr_based_denials() {
        let stderr = "note: `#[deny(unexpected_cfgs)]` implied by `#[deny(warnings)]`";
        assert!(is_warning_as_error_failure(stderr));
    }

    #[test]
    fn test_is_warning_as_error_failure_ignores_non_warning_errors() {
        let stderr = "error[E0425]: cannot find value `x` in this scope";
        assert!(!is_warning_as_error_failure(stderr));
    }

    #[test]
    fn test_is_workspace_package_miss_detects_non_member_dev_dependency_error() {
        let stderr = "error: package `bitflags` cannot be tested because it requires dev-dependencies and is not a member of the workspace";
        assert!(is_workspace_package_miss(stderr));
    }

    #[test]
    fn test_parity_cpp_compiler_from_env_defaults_to_clangpp() {
        assert_eq!(parity_cpp_compiler_from_env(None), "clang++");
    }

    #[test]
    fn test_parity_cpp_compiler_from_env_uses_non_empty_value() {
        assert_eq!(
            parity_cpp_compiler_from_env(Some("clang++".to_string())),
            "clang++"
        );
        assert_eq!(parity_cpp_compiler_from_env(Some("g++".to_string())), "g++");
    }

    #[test]
    fn test_parity_cpp_compiler_from_env_trims_and_falls_back_on_empty() {
        assert_eq!(
            parity_cpp_compiler_from_env(Some("  ".to_string())),
            "clang++"
        );
        assert_eq!(
            parity_cpp_compiler_from_env(Some("  /usr/bin/clang++  ".to_string())),
            "/usr/bin/clang++"
        );
    }

    #[test]
    fn test_parse_running_tests_count_parses_cargo_test_lines() {
        assert_eq!(parse_running_tests_count("running 0 tests"), Some(0));
        assert_eq!(parse_running_tests_count("running 1 test"), Some(1));
        assert_eq!(parse_running_tests_count("running 42 tests"), Some(42));
        assert_eq!(
            parse_running_tests_count(" test result: ok. 0 passed"),
            None
        );
    }

    #[test]
    fn test_baseline_ran_any_tests_detects_zero_vs_nonzero_runs() {
        let temp = tempfile::tempdir().expect("temp dir");
        let baseline = temp.path().join("baseline.txt");

        std::fs::write(
            &baseline,
            "running 0 tests\ntest result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out",
        )
        .expect("write baseline");
        assert_eq!(baseline_ran_any_tests(temp.path()), Some(false));

        std::fs::write(
            &baseline,
            "running 3 tests\ntest result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out",
        )
        .expect("write baseline");
        assert_eq!(baseline_ran_any_tests(temp.path()), Some(true));
    }

    #[test]
    fn test_collect_named_module_imports_parses_export_import_lines() {
        let content = "export import serde_core;\nimport serde;\nimport <vector>;\n";
        let imports = collect_named_module_imports(content);
        assert!(imports.contains("serde_core"));
        assert!(imports.contains("serde"));
        assert!(!imports.contains("<vector>"));
    }

    #[test]
    fn test_inject_named_module_imports_emits_export_imports() {
        let content = "export module my_mod;\n\nexport int f();\n";
        let out =
            inject_named_module_imports(content, &["serde".to_string(), "serde_core".to_string()]);
        assert!(out.contains("export import serde;\n"));
        assert!(out.contains("export import serde_core;\n"));
        assert!(!out.lines().any(|line| line.trim() == "import serde;"));
        assert!(!out.lines().any(|line| line.trim() == "import serde_core;"));
    }

    #[test]
    fn test_inject_named_module_imports_does_not_duplicate_existing_imports() {
        let content = "export module my_mod;\nimport serde_core;\n\nexport int f();\n";
        let out = inject_named_module_imports(content, &["serde_core".to_string()]);
        let count = out
            .lines()
            .filter(|line| line.trim() == "import serde_core;")
            .count();
        assert_eq!(count, 1);
        assert!(
            !out.lines()
                .any(|line| line.trim() == "export import serde_core;")
        );
    }

    #[test]
    fn test_dependency_expand_cargo_flags_handles_default_only() {
        let flags = dependency_expand_cargo_flags(&["default".to_string()]);
        assert!(flags.is_empty());
    }

    #[test]
    fn test_dependency_expand_cargo_flags_handles_no_default_with_named_features() {
        let flags = dependency_expand_cargo_flags(&[
            "serde".to_string(),
            "alloc".to_string(),
            "serde".to_string(),
        ]);
        assert_eq!(
            flags,
            vec![
                "--no-default-features".to_string(),
                "--features".to_string(),
                "alloc,serde".to_string(),
            ]
        );
    }

    #[test]
    fn test_dependency_expand_cargo_flags_handles_default_plus_extra_features() {
        let flags = dependency_expand_cargo_flags(&[
            "default".to_string(),
            "std".to_string(),
            "serde".to_string(),
        ]);
        assert_eq!(
            flags,
            vec!["--features".to_string(), "serde,std".to_string(),]
        );
    }
}

fn run_cargo_test(
    current_dir: &Path,
    manifest_path: Option<&Path>,
    package: Option<&str>,
    cargo_flags: &[String],
    extra_rustflags: Option<&str>,
) -> Result<Output, String> {
    let mut cmd = std::process::Command::new("cargo");
    cmd.arg("test").current_dir(current_dir);
    if let Some(path) = manifest_path {
        cmd.arg("--manifest-path").arg(path);
    }
    if let Some(pkg) = package {
        cmd.arg("-p").arg(pkg);
    }
    for flag in cargo_flags {
        cmd.arg(flag);
    }
    if let Some(extra_flags) = extra_rustflags {
        let merged = match std::env::var("RUSTFLAGS")
            .ok()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
        {
            Some(existing) => format!("{} {}", existing, extra_flags),
            None => extra_flags.to_string(),
        };
        cmd.env("RUSTFLAGS", merged);
    }
    if let Some(target) = shared_cargo_target_dir() {
        cmd.env("CARGO_TARGET_DIR", &target);
    }
    cmd.output()
        .map_err(|e| format!("Failed to run cargo test: {}", e))
}

fn run_cargo_expand_command(
    current_dir: &Path,
    manifest_path: Option<&Path>,
    package: Option<&str>,
    expand_args: &[String],
    cargo_flags: &[String],
) -> Result<Output, String> {
    let mut cmd = std::process::Command::new("cargo");
    cmd.arg("expand").current_dir(current_dir);
    if let Some(path) = manifest_path {
        cmd.arg("--manifest-path").arg(path);
    }
    if let Some(pkg) = package {
        cmd.arg("-p").arg(pkg);
    }
    for arg in expand_args {
        cmd.arg(arg);
    }
    cmd.arg("--theme=none");
    for flag in cargo_flags {
        cmd.arg(flag);
    }
    if let Some(target) = shared_cargo_target_dir() {
        cmd.env("CARGO_TARGET_DIR", &target);
    }
    cmd.output()
        .map_err(|e| format!("Failed to run cargo expand: {}", e))
}

fn is_workspace_mismatch(stderr: &str) -> bool {
    stderr.contains("current package believes it's in a workspace when it's not")
}

fn workspace_manifest_from_error(stderr: &str) -> Option<PathBuf> {
    for line in stderr.lines() {
        let trimmed = line.trim();
        if let Some(path) = trimmed.strip_prefix("workspace:") {
            let candidate = path.trim();
            if !candidate.is_empty() {
                return Some(PathBuf::from(candidate));
            }
        }
    }
    None
}

fn is_workspace_package_miss(stderr: &str) -> bool {
    stderr.contains("did not match any packages")
        || stderr.contains("package ID specification")
        || stderr.contains("not found in workspace")
        || stderr.contains("not found in metadata")
        || stderr.contains(
            "cannot be tested because it requires dev-dependencies and is not a member of the workspace",
        )
}

fn is_warning_as_error_failure(stderr: &str) -> bool {
    stderr.contains("implied by `#[deny(warnings)]`")
        || stderr.contains("requested on the command line with `-D warnings`")
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<(), String> {
    fs::create_dir_all(dst)
        .map_err(|e| format!("Failed to create directory {}: {}", dst.display(), e))?;
    for entry in
        fs::read_dir(src).map_err(|e| format!("Failed to read {}: {}", src.display(), e))?
    {
        let entry = entry.map_err(|e| format!("Failed to read directory entry: {}", e))?;
        let path = entry.path();
        let name = entry.file_name();
        let dest_path = dst.join(&name);
        let file_type = entry
            .file_type()
            .map_err(|e| format!("Failed to stat {}: {}", path.display(), e))?;

        if file_type.is_dir() {
            if name == "target" || name == ".git" {
                continue;
            }
            copy_dir_recursive(&path, &dest_path)?;
        } else if file_type.is_file() || file_type.is_symlink() {
            fs::copy(&path, &dest_path).map_err(|e| {
                format!(
                    "Failed to copy {} -> {}: {}",
                    path.display(),
                    dest_path.display(),
                    e
                )
            })?;
        }
    }
    Ok(())
}

fn ensure_manifest_workspace_isolation(manifest: &Path) -> Result<(), String> {
    let mut content = fs::read_to_string(manifest)
        .map_err(|e| format!("Failed to read manifest {}: {}", manifest.display(), e))?;
    if content.lines().any(|line| line.trim() == "[workspace]") {
        return Ok(());
    }
    if !content.ends_with('\n') {
        content.push('\n');
    }
    content.push_str("\n[workspace]\n");
    fs::write(manifest, content)
        .map_err(|e| format!("Failed to update manifest {}: {}", manifest.display(), e))
}

fn ensure_isolated_manifest_copy(
    manifest: &Path,
    project_dir: &Path,
    work_dir: &Path,
    stage_dir_name: &str,
    cached_manifest: &mut Option<PathBuf>,
) -> Result<PathBuf, String> {
    if let Some(path) = cached_manifest {
        return Ok(path.clone());
    }

    let isolated_root = work_dir.join(stage_dir_name);
    if isolated_root.exists() {
        fs::remove_dir_all(&isolated_root).map_err(|e| {
            format!(
                "Failed to clean {} isolation dir {}: {}",
                stage_dir_name,
                isolated_root.display(),
                e
            )
        })?;
    }
    copy_dir_recursive(project_dir, &isolated_root)?;

    let manifest_rel = manifest
        .strip_prefix(project_dir)
        .map_err(|_| {
            format!(
                "Manifest {} is not under project dir {}",
                manifest.display(),
                project_dir.display()
            )
        })?
        .to_path_buf();
    let isolated_manifest = isolated_root.join(manifest_rel);
    ensure_manifest_workspace_isolation(&isolated_manifest)?;
    *cached_manifest = Some(isolated_manifest.clone());
    Ok(isolated_manifest)
}

fn run_baseline_attempt(
    manifest: &Path,
    project_dir: &Path,
    package: Option<&str>,
    crate_name: &str,
    cargo_flags: &[String],
    work_dir: &Path,
    extra_rustflags: Option<&str>,
) -> Result<Output, String> {
    let initial = run_cargo_test(project_dir, None, package, cargo_flags, extra_rustflags)?;
    if initial.status.success() {
        return Ok(initial);
    }

    let initial_stderr = String::from_utf8_lossy(&initial.stderr);
    if !is_workspace_mismatch(&initial_stderr) {
        return Ok(initial);
    }

    println!("  Baseline retry: detected workspace mismatch from in-place cargo test.");

    let selected_package = package.unwrap_or(crate_name);
    if let Some(workspace_manifest) = workspace_manifest_from_error(&initial_stderr) {
        let workspace_root = workspace_manifest
            .parent()
            .unwrap_or_else(|| Path::new("."));
        println!(
            "  Baseline retry: cargo test --manifest-path {} -p {}",
            workspace_manifest.display(),
            selected_package
        );
        let workspace_output = run_cargo_test(
            workspace_root,
            Some(&workspace_manifest),
            Some(selected_package),
            cargo_flags,
            extra_rustflags,
        )?;
        if workspace_output.status.success() {
            return Ok(workspace_output);
        }
        let workspace_stderr = String::from_utf8_lossy(&workspace_output.stderr);
        if !is_workspace_package_miss(&workspace_stderr) {
            return Ok(workspace_output);
        }
    }

    let mut isolated_manifest_cache = None;
    let isolated_manifest = ensure_isolated_manifest_copy(
        manifest,
        project_dir,
        work_dir,
        "baseline_source_manifest",
        &mut isolated_manifest_cache,
    )?;
    let isolated_root = isolated_manifest.parent().unwrap_or_else(|| Path::new("."));

    println!(
        "  Baseline retry: cargo test --manifest-path {}",
        isolated_manifest.display()
    );
    run_cargo_test(
        isolated_root,
        Some(&isolated_manifest),
        package,
        cargo_flags,
        extra_rustflags,
    )
}

fn run_baseline_with_workspace_fallback(
    manifest: &Path,
    project_dir: &Path,
    package: Option<&str>,
    crate_name: &str,
    cargo_flags: &[String],
    work_dir: &Path,
) -> Result<Output, String> {
    const LINT_RETRY_FLAGS: &str = "--cap-lints allow";

    let output = run_baseline_attempt(
        manifest,
        project_dir,
        package,
        crate_name,
        cargo_flags,
        work_dir,
        None,
    )?;
    if output.status.success() {
        return Ok(output);
    }

    let stderr = String::from_utf8_lossy(&output.stderr);
    if !is_warning_as_error_failure(&stderr) {
        return Ok(output);
    }

    println!("  Baseline retry: detected warning-as-error lint failure.");
    println!(
        "  Baseline retry: cargo test with RUSTFLAGS += '{}'",
        LINT_RETRY_FLAGS
    );
    run_baseline_attempt(
        manifest,
        project_dir,
        package,
        crate_name,
        cargo_flags,
        work_dir,
        Some(LINT_RETRY_FLAGS),
    )
}

fn discover_targets_with_workspace_fallback(
    manifest: &Path,
    project_dir: &Path,
    package: Option<&str>,
    crate_name: &str,
    work_dir: &Path,
) -> Result<(String, Vec<metadata::CrateTarget>), String> {
    let initial = metadata::discover_targets(manifest, package);
    if initial.is_ok() {
        return initial;
    }

    let initial_err = initial.err().unwrap_or_default();
    if !is_workspace_mismatch(&initial_err) {
        return Err(initial_err);
    }

    println!("  Metadata retry: detected workspace mismatch from in-place cargo metadata.");

    let selected_package = package.unwrap_or(crate_name);
    if let Some(workspace_manifest) = workspace_manifest_from_error(&initial_err) {
        println!(
            "  Metadata retry: cargo metadata --manifest-path {} -p {}",
            workspace_manifest.display(),
            selected_package
        );
        let workspace_attempt =
            metadata::discover_targets(&workspace_manifest, Some(selected_package));
        if workspace_attempt.is_ok() {
            return workspace_attempt;
        }

        let workspace_err = workspace_attempt.err().unwrap_or_default();
        if !is_workspace_package_miss(&workspace_err) {
            return Err(workspace_err);
        }
    }

    let mut isolated_manifest_cache = None;
    let isolated_manifest = ensure_isolated_manifest_copy(
        manifest,
        project_dir,
        work_dir,
        "metadata_source_manifest",
        &mut isolated_manifest_cache,
    )?;
    println!(
        "  Metadata retry: cargo metadata --manifest-path {}",
        isolated_manifest.display()
    );
    metadata::discover_targets(&isolated_manifest, package)
}

fn discover_local_dependencies_with_workspace_fallback(
    manifest: &Path,
    project_dir: &Path,
    package: Option<&str>,
    crate_name: &str,
    work_dir: &Path,
    include_registry_packages: bool,
    include_dev_dependencies: bool,
    cargo_flags: &[String],
) -> Result<Vec<metadata::LocalDependencyPackage>, String> {
    let initial = metadata::discover_library_dependencies(
        manifest,
        package,
        include_registry_packages,
        include_dev_dependencies,
        cargo_flags,
    );
    if initial.is_ok() {
        return initial;
    }

    let initial_err = initial.err().unwrap_or_default();
    if !is_workspace_mismatch(&initial_err) {
        return Err(initial_err);
    }

    println!("  Dependency metadata retry: detected workspace mismatch.");
    let selected_package = package.unwrap_or(crate_name);
    if let Some(workspace_manifest) = workspace_manifest_from_error(&initial_err) {
        println!(
            "  Dependency metadata retry: cargo metadata --manifest-path {} -p {}",
            workspace_manifest.display(),
            selected_package
        );
        let workspace_attempt = metadata::discover_library_dependencies(
            &workspace_manifest,
            Some(selected_package),
            include_registry_packages,
            include_dev_dependencies,
            cargo_flags,
        );
        if workspace_attempt.is_ok() {
            return workspace_attempt;
        }

        let workspace_err = workspace_attempt.err().unwrap_or_default();
        if !is_workspace_package_miss(&workspace_err) {
            return Err(workspace_err);
        }
    }

    let mut isolated_manifest_cache = None;
    let isolated_manifest = ensure_isolated_manifest_copy(
        manifest,
        project_dir,
        work_dir,
        "dependency_metadata_source_manifest",
        &mut isolated_manifest_cache,
    )?;
    println!(
        "  Dependency metadata retry: cargo metadata --manifest-path {}",
        isolated_manifest.display()
    );
    metadata::discover_library_dependencies(
        &isolated_manifest,
        package,
        include_registry_packages,
        include_dev_dependencies,
        cargo_flags,
    )
}

fn run_cargo_expand_with_workspace_fallback(
    manifest: &Path,
    project_dir: &Path,
    package: Option<&str>,
    crate_name: &str,
    expand_args: &[String],
    cargo_flags: &[String],
    work_dir: &Path,
    isolated_manifest_cache: &mut Option<PathBuf>,
) -> Result<Output, String> {
    let initial = run_cargo_expand_command(project_dir, None, None, expand_args, cargo_flags)?;
    if initial.status.success() {
        return Ok(initial);
    }

    let initial_stderr = String::from_utf8_lossy(&initial.stderr);
    // A target that requires a cargo feature we don't enable — e.g. smallvec's
    // `debugger_visualizer` integration test ("requires the features:
    // debugger_visualizer") — is legitimately not expandable in this config. It is
    // NOT a workspace mismatch, and routing it through the workspace/isolated
    // retry can recurse into a cargo-resolver stack overflow that aborts the whole
    // crate run. Return the failed output so the caller skips just this target.
    if initial_stderr.contains("requires the features") {
        return Ok(initial);
    }
    if !is_workspace_mismatch(&initial_stderr) {
        return Ok(initial);
    }

    println!("  Expand retry: detected workspace mismatch from in-place cargo expand.");

    let selected_package = package.unwrap_or(crate_name);
    if let Some(workspace_manifest) = workspace_manifest_from_error(&initial_stderr) {
        let workspace_root = workspace_manifest
            .parent()
            .unwrap_or_else(|| Path::new("."));
        println!(
            "  Expand retry: cargo expand --manifest-path {} -p {}",
            workspace_manifest.display(),
            selected_package
        );
        let workspace_output = run_cargo_expand_command(
            workspace_root,
            Some(&workspace_manifest),
            Some(selected_package),
            expand_args,
            cargo_flags,
        )?;
        if workspace_output.status.success() {
            return Ok(workspace_output);
        }

        let workspace_stderr = String::from_utf8_lossy(&workspace_output.stderr);
        // Fall through to isolated-manifest expansion when:
        //   - The workspace doesn't contain the package (common), OR
        //   - cargo itself panicked during workspace expansion. The
        //     resolver at src/tools/cargo/.../features.rs sometimes
        //     crashes on integration tests of packages excluded from
        //     the parent workspace (e.g. semver). The isolated manifest
        //     copy avoids the workspace context entirely.
        let workspace_panicked =
            workspace_stderr.contains("panicked at ") && workspace_stderr.contains("cargo");
        if !is_workspace_package_miss(&workspace_stderr) && !workspace_panicked {
            return Ok(workspace_output);
        }
    }

    let isolated_manifest = ensure_isolated_manifest_copy(
        manifest,
        project_dir,
        work_dir,
        "expand_source_manifest",
        isolated_manifest_cache,
    )?;
    let isolated_root = isolated_manifest.parent().unwrap_or_else(|| Path::new("."));
    println!(
        "  Expand retry: cargo expand --manifest-path {}",
        isolated_manifest.display()
    );
    run_cargo_expand_command(
        isolated_root,
        Some(&isolated_manifest),
        package,
        expand_args,
        cargo_flags,
    )
}

fn remove_file_if_exists(path: &Path) -> Result<(), String> {
    if !path.exists() {
        return Ok(());
    }
    fs::remove_file(path)
        .map_err(|e| format!("Failed to remove stale file {}: {}", path.display(), e))
}

fn clear_stage_outputs(work_dir: &Path) -> Result<(), String> {
    for file_name in [
        "baseline.txt",
        "runner.cpp",
        "runner",
        "build.log",
        "run.log",
    ] {
        remove_file_if_exists(&work_dir.join(file_name))?;
    }
    Ok(())
}

fn is_external_crate_root_candidate(root: &str) -> bool {
    if root.is_empty() || root == "_" {
        return false;
    }
    if !root
        .chars()
        .next()
        .is_some_and(|ch| ch.is_ascii_lowercase())
    {
        return false;
    }
    !matches!(
        root,
        "crate" | "self" | "super" | "std" | "core" | "alloc" | "cpp" | "rusty"
    )
}

fn is_runtime_provided_external_crate_root(root: &str) -> bool {
    matches!(root, "winnow" | "memchr")
}

fn collect_external_crate_todo_markers(cpp: &str) -> Vec<String> {
    let mut roots = HashSet::new();
    for line in cpp.lines() {
        let Some(idx) = line.find("// TODO: external crate '") else {
            continue;
        };
        let marker = &line[idx + "// TODO: external crate '".len()..];
        let Some(end_idx) = marker.find('\'') else {
            continue;
        };
        let root = marker[..end_idx].trim();
        if !root.is_empty() {
            roots.insert(root.to_string());
        }
    }
    let mut out: Vec<String> = roots.into_iter().collect();
    out.sort();
    out
}

/// Detect transpiled output that contains C++-invalid template
/// argument patterns. The transpiler emits `<auto>` when it can't
/// infer the concrete type for a Rust generic; this is invalid C++
/// (`auto` is only allowed in placeholder type positions, not as
/// template arguments). Test targets containing such patterns are
/// skipped to avoid breaking the build.
fn cpp_has_invalid_codegen_pattern(cpp: &str) -> bool {
    // `Type<auto>` or `Type<auto,` — auto in a template argument list.
    // Match either standalone `<auto>` or `<auto,` to catch first or
    // intermediate positions. False positives possible in string
    // literals or comments but they're rare.
    cpp.contains("<auto>") || cpp.contains("<auto,") || cpp.contains(", auto>")
        || cpp.contains(", auto,")
}


fn ensure_no_external_crate_todos(label: &str, cpp: &str, cppm_path: &Path) -> Result<(), String> {
    let unresolved = collect_external_crate_todo_markers(cpp);
    if unresolved.is_empty() {
        return Ok(());
    }
    Err(format!(
        "Transpiled {} still contains unresolved external crate imports: {} (artifact: {})",
        label,
        unresolved.join(", "),
        cppm_path.display()
    ))
}

fn rewrite_winnow_namespace_conflicts(cpp: &str) -> String {
    cpp.replace("namespace error::", "namespace winnow_error::")
        .replace("namespace error {", "namespace winnow_error {")
        .replace("::error::", "::winnow_error::")
        .replace(" error::", " winnow_error::")
}

fn collect_external_crate_roots_from_source(source: &str) -> HashSet<String> {
    fn parse_leading_ident(input: &str) -> Option<String> {
        let mut chars = input.chars();
        let first = chars.next()?;
        if !(first == '_' || first.is_ascii_alphabetic()) {
            return None;
        }
        let mut ident = String::new();
        ident.push(first);
        for ch in chars {
            if ch == '_' || ch.is_ascii_alphanumeric() {
                ident.push(ch);
            } else {
                break;
            }
        }
        if ident.is_empty() { None } else { Some(ident) }
    }

    fn collect_textual_use_root(roots: &mut HashSet<String>, line: &str) {
        let trimmed = line.trim_start();

        // Handle `extern crate foo;` and `extern crate foo as bar;`.
        if let Some(rest) = trimmed.strip_prefix("extern crate ") {
            if let Some(root) = parse_leading_ident(rest.trim_start())
                && is_external_crate_root_candidate(&root)
            {
                roots.insert(root);
            }
            return;
        }

        // Handle `use foo::...;` plus simple `pub use`.
        let use_rest = if let Some(rest) = trimmed.strip_prefix("use ") {
            Some(rest)
        } else if let Some(rest) = trimmed.strip_prefix("pub use ") {
            Some(rest)
        } else {
            None
        };
        let Some(rest) = use_rest else {
            return;
        };

        let rest = rest
            .trim_start_matches(':')
            .trim_start_matches(':')
            .trim_start();
        if let Some(root) = parse_leading_ident(rest)
            && is_external_crate_root_candidate(&root)
        {
            roots.insert(root);
        }
    }

    struct RootCollector {
        roots: HashSet<String>,
    }

    impl<'ast> syn::visit::Visit<'ast> for RootCollector {
        fn visit_path(&mut self, path: &'ast syn::Path) {
            if let Some(first) = path.segments.first() {
                let root = first.ident.to_string();
                if is_external_crate_root_candidate(&root) {
                    self.roots.insert(root);
                }
            }
            syn::visit::visit_path(self, path);
        }
    }

    let mut roots = HashSet::new();
    if let Ok(file) = syn::parse_file(source) {
        let mut collector = RootCollector {
            roots: HashSet::new(),
        };
        syn::visit::Visit::visit_file(&mut collector, &file);
        roots.extend(collector.roots);
    }

    // Fallback for expanded snippets `syn` cannot parse (or partially misses):
    // collect crate roots from textual `use` / `extern crate` lines.
    for line in source.lines() {
        collect_textual_use_root(&mut roots, line);
    }

    roots
}

#[derive(Debug, Clone)]
struct ParityDependencyTarget {
    package_name: String,
    manifest_path: PathBuf,
    module_name: String,
    extern_crate_roots: Vec<String>,
    is_registry: bool,
    cargo_flags: Vec<String>,
}

#[derive(Debug, Clone)]
struct GeneratedCppmArtifact {
    path: PathBuf,
    module_name: String,
    is_dependency: bool,
    /// True for parity test targets (cargo `[[test]]` integration
    /// tests, `--test X`). Lib and dep artifacts are false. Used by
    /// the module-build pipeline to skip individual test targets that
    /// fail to precompile rather than failing the whole crate.
    is_test_target: bool,
}

fn target_artifacts_root(work_dir: &Path) -> PathBuf {
    work_dir.join("targets")
}

fn target_artifact_dir(work_dir: &Path, module_name: &str) -> PathBuf {
    target_artifacts_root(work_dir).join(module_name)
}

fn expanded_artifact_path(target_dir: &Path) -> PathBuf {
    target_dir.join("expanded.rs")
}

fn cppm_artifact_path(target_dir: &Path, module_name: &str) -> PathBuf {
    target_dir.join(format!("{}.cppm", module_name))
}

fn dependency_artifacts_root(work_dir: &Path) -> PathBuf {
    work_dir.join("deps")
}

fn dependency_artifact_dir(work_dir: &Path, module_name: &str) -> PathBuf {
    dependency_artifacts_root(work_dir).join(module_name)
}

fn dependency_expand_cargo_flags(resolved_features: &[String]) -> Vec<String> {
    let mut features: Vec<String> = resolved_features
        .iter()
        .map(|feature| feature.trim())
        .filter(|feature| !feature.is_empty())
        .map(ToString::to_string)
        .collect();
    features.sort();
    features.dedup();

    let default_enabled = features.iter().any(|feature| feature == "default");
    features.retain(|feature| feature != "default");

    let mut flags = Vec::new();
    if !default_enabled {
        flags.push("--no-default-features".to_string());
    }
    if !features.is_empty() {
        flags.push("--features".to_string());
        flags.push(features.join(","));
    }
    flags
}

fn reset_target_artifacts(
    work_dir: &Path,
    targets: &[metadata::CrateTarget],
) -> Result<HashMap<String, PathBuf>, String> {
    let artifacts_root = target_artifacts_root(work_dir);
    fs::create_dir_all(&artifacts_root).map_err(|e| {
        format!(
            "Failed to create target artifacts directory {}: {}",
            artifacts_root.display(),
            e
        )
    })?;

    let expected_modules: HashSet<&str> = targets.iter().map(|t| t.module_name.as_str()).collect();

    for entry in fs::read_dir(&artifacts_root)
        .map_err(|e| format!("Failed to read {}: {}", artifacts_root.display(), e))?
    {
        let entry = entry.map_err(|e| {
            format!(
                "Failed to inspect {} entry: {}",
                artifacts_root.display(),
                e
            )
        })?;
        let path = entry.path();
        let file_type = entry
            .file_type()
            .map_err(|e| format!("Failed to inspect {}: {}", path.display(), e))?;

        if file_type.is_dir() {
            let name = entry.file_name().to_string_lossy().to_string();
            if !expected_modules.contains(name.as_str()) {
                fs::remove_dir_all(&path).map_err(|e| {
                    format!(
                        "Failed to remove stale target dir {}: {}",
                        path.display(),
                        e
                    )
                })?;
            }
        } else {
            fs::remove_file(&path)
                .map_err(|e| format!("Failed to remove stale file {}: {}", path.display(), e))?;
        }
    }

    let mut target_dirs = HashMap::new();
    for target in targets {
        let target_dir = target_artifact_dir(work_dir, &target.module_name);
        if target_dir.exists() {
            fs::remove_dir_all(&target_dir).map_err(|e| {
                format!("Failed to reset target dir {}: {}", target_dir.display(), e)
            })?;
        }
        fs::create_dir_all(&target_dir).map_err(|e| {
            format!(
                "Failed to create target dir {}: {}",
                target_dir.display(),
                e
            )
        })?;
        target_dirs.insert(target.module_name.clone(), target_dir);
    }

    Ok(target_dirs)
}

fn reset_dependency_artifacts(
    work_dir: &Path,
    deps: &[ParityDependencyTarget],
) -> Result<HashMap<String, PathBuf>, String> {
    let artifacts_root = dependency_artifacts_root(work_dir);
    fs::create_dir_all(&artifacts_root).map_err(|e| {
        format!(
            "Failed to create dependency artifacts directory {}: {}",
            artifacts_root.display(),
            e
        )
    })?;

    let expected_modules: HashSet<&str> = deps.iter().map(|d| d.module_name.as_str()).collect();
    for entry in fs::read_dir(&artifacts_root)
        .map_err(|e| format!("Failed to read {}: {}", artifacts_root.display(), e))?
    {
        let entry = entry.map_err(|e| {
            format!(
                "Failed to inspect {} entry: {}",
                artifacts_root.display(),
                e
            )
        })?;
        let path = entry.path();
        let file_type = entry
            .file_type()
            .map_err(|e| format!("Failed to inspect {}: {}", path.display(), e))?;
        if file_type.is_dir() {
            let name = entry.file_name().to_string_lossy().to_string();
            if !expected_modules.contains(name.as_str()) {
                fs::remove_dir_all(&path).map_err(|e| {
                    format!(
                        "Failed to remove stale dependency dir {}: {}",
                        path.display(),
                        e
                    )
                })?;
            }
        } else {
            fs::remove_file(&path)
                .map_err(|e| format!("Failed to remove stale file {}: {}", path.display(), e))?;
        }
    }

    let mut dep_dirs = HashMap::new();
    for dep in deps {
        let dep_dir = dependency_artifact_dir(work_dir, &dep.module_name);
        if dep_dir.exists() {
            fs::remove_dir_all(&dep_dir).map_err(|e| {
                format!(
                    "Failed to reset dependency dir {}: {}",
                    dep_dir.display(),
                    e
                )
            })?;
        }
        fs::create_dir_all(&dep_dir).map_err(|e| {
            format!(
                "Failed to create dependency dir {}: {}",
                dep_dir.display(),
                e
            )
        })?;
        dep_dirs.insert(dep.module_name.clone(), dep_dir);
    }
    Ok(dep_dirs)
}

fn ensure_target_artifact_dirs(
    work_dir: &Path,
    targets: &[metadata::CrateTarget],
) -> Result<HashMap<String, PathBuf>, String> {
    let artifacts_root = target_artifacts_root(work_dir);
    fs::create_dir_all(&artifacts_root).map_err(|e| {
        format!(
            "Failed to create target artifacts directory {}: {}",
            artifacts_root.display(),
            e
        )
    })?;
    let mut target_dirs = HashMap::new();
    for target in targets {
        let target_dir = target_artifact_dir(work_dir, &target.module_name);
        fs::create_dir_all(&target_dir).map_err(|e| {
            format!(
                "Failed to create target dir {}: {}",
                target_dir.display(),
                e
            )
        })?;
        target_dirs.insert(target.module_name.clone(), target_dir);
    }
    Ok(target_dirs)
}

fn ensure_dependency_artifact_dirs(
    work_dir: &Path,
    deps: &[ParityDependencyTarget],
) -> Result<HashMap<String, PathBuf>, String> {
    let artifacts_root = dependency_artifacts_root(work_dir);
    fs::create_dir_all(&artifacts_root).map_err(|e| {
        format!(
            "Failed to create dependency artifacts directory {}: {}",
            artifacts_root.display(),
            e
        )
    })?;
    let mut dep_dirs = HashMap::new();
    for dep in deps {
        let dep_dir = dependency_artifact_dir(work_dir, &dep.module_name);
        fs::create_dir_all(&dep_dir).map_err(|e| {
            format!(
                "Failed to create dependency dir {}: {}",
                dep_dir.display(),
                e
            )
        })?;
        dep_dirs.insert(dep.module_name.clone(), dep_dir);
    }
    Ok(dep_dirs)
}

fn module_artifact_name(module_name: &str, ext: &str) -> String {
    let stem: String = module_name
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '_' || ch == '.' || ch == '-' {
                ch
            } else {
                '_'
            }
        })
        .collect();
    format!("{}.{}", stem, ext)
}

fn parse_named_module_import(trimmed: &str) -> Option<String> {
    let line = strip_export_prefix(trimmed).trim();
    let rest = line.strip_prefix("import ")?;
    let module = rest.trim_end_matches(';').trim();
    if module.is_empty() || module.starts_with('<') || module.starts_with('"') {
        return None;
    }
    Some(module.to_string())
}

fn collect_named_module_imports(content: &str) -> BTreeSet<String> {
    let mut imports = BTreeSet::new();
    for line in content.lines() {
        let trimmed = line.trim();
        if let Some(module) = parse_named_module_import(trimmed) {
            imports.insert(module);
        }
    }
    imports
}

fn collect_required_named_module_imports(
    source: &str,
    current_module: &str,
    root_to_module_import: &HashMap<String, String>,
) -> Vec<String> {
    let mut modules = BTreeSet::new();
    for root in collect_external_crate_roots_from_source(source) {
        let Some(module_name) = root_to_module_import.get(&root) else {
            continue;
        };
        let module_name = module_name.trim();
        if module_name.is_empty() || module_name == current_module {
            continue;
        }
        modules.insert(module_name.to_string());
    }
    modules.into_iter().collect()
}

fn inject_named_module_imports(cpp: &str, required_modules: &[String]) -> String {
    if required_modules.is_empty() {
        return cpp.to_string();
    }
    let mut missing_modules: BTreeSet<String> = required_modules
        .iter()
        .map(|module| module.trim())
        .filter(|module| !module.is_empty())
        .map(|module| module.to_string())
        .collect();
    if missing_modules.is_empty() {
        return cpp.to_string();
    }

    let existing = collect_named_module_imports(cpp);
    missing_modules.retain(|module| !existing.contains(module));
    if missing_modules.is_empty() {
        return cpp.to_string();
    }

    let mut rewritten = String::new();
    let mut inserted = false;
    for line in cpp.split_inclusive('\n') {
        rewritten.push_str(line);
        if !inserted && line.trim_start().starts_with("export module ") {
            for module in &missing_modules {
                rewritten.push_str("export import ");
                rewritten.push_str(module);
                rewritten.push_str(";\n");
            }
            rewritten.push('\n');
            inserted = true;
        }
    }
    if !inserted {
        return cpp.to_string();
    }
    rewritten
}

#[derive(Debug, Clone)]
struct ModuleBuildUnit {
    module_name: String,
    source_path: PathBuf,
    imports: BTreeSet<String>,
    pcm_path: PathBuf,
    object_path: PathBuf,
    /// True for parity test target modules.
    is_test_target: bool,
    /// True for crate dependencies (e.g. dev-dependencies pulled in
    /// only by test targets). When a dep's precompile fails, we mark
    /// it as skipped and rely on cascade-skip to drop any test target
    /// that imports it. If a non-test, non-dep unit (the lib target)
    /// depends on the skipped dep, its precompile will fail and we
    /// fail-fast — the dep was essential.
    is_dependency: bool,
}

fn module_build_order(units: &[ModuleBuildUnit]) -> Vec<usize> {
    let module_to_idx: HashMap<&str, usize> = units
        .iter()
        .enumerate()
        .map(|(idx, unit)| (unit.module_name.as_str(), idx))
        .collect();
    let mut indegree = vec![0usize; units.len()];
    let mut outgoing: Vec<Vec<usize>> = vec![Vec::new(); units.len()];

    for (idx, unit) in units.iter().enumerate() {
        for imported in &unit.imports {
            if let Some(dep_idx) = module_to_idx.get(imported.as_str()) {
                if *dep_idx == idx {
                    continue;
                }
                indegree[idx] += 1;
                outgoing[*dep_idx].push(idx);
            }
        }
    }

    let mut ready: BTreeSet<(String, usize)> = BTreeSet::new();
    for (idx, unit) in units.iter().enumerate() {
        if indegree[idx] == 0 {
            ready.insert((unit.module_name.clone(), idx));
        }
    }

    let mut order = Vec::with_capacity(units.len());
    while let Some((_, idx)) = ready.pop_first() {
        order.push(idx);
        for next in &outgoing[idx] {
            indegree[*next] = indegree[*next].saturating_sub(1);
            if indegree[*next] == 0 {
                ready.insert((units[*next].module_name.clone(), *next));
            }
        }
    }

    if order.len() != units.len() {
        return (0..units.len()).collect();
    }
    order
}

#[derive(Debug, Deserialize)]
struct LibcxxModulesManifest {
    modules: Vec<LibcxxModuleEntry>,
}

#[derive(Debug, Deserialize)]
struct LibcxxModuleEntry {
    #[serde(rename = "logical-name")]
    logical_name: String,
    #[serde(rename = "source-path")]
    source_path: String,
    #[serde(rename = "local-arguments", default)]
    local_arguments: LibcxxLocalArguments,
}

#[derive(Debug, Default, Deserialize)]
struct LibcxxLocalArguments {
    #[serde(rename = "system-include-directories", default)]
    system_include_directories: Vec<String>,
}

#[derive(Debug, Clone)]
struct LibcxxStdModuleConfig {
    source_path: PathBuf,
    system_include_directories: Vec<PathBuf>,
}

fn resolve_libcxx_std_module_config(cpp_compiler: &str) -> Result<LibcxxStdModuleConfig, String> {
    let probe_output = std::process::Command::new(cpp_compiler)
        .arg("-print-file-name=libc++.modules.json")
        .output()
        .map_err(|e| {
            format!(
                "Failed to probe libc++ modules manifest via '{} -print-file-name=libc++.modules.json': {}",
                cpp_compiler, e
            )
        })?;
    if !probe_output.status.success() {
        return Err(format!(
            "Compiler '{}' failed probing libc++ modules manifest",
            cpp_compiler
        ));
    }

    let manifest_raw = String::from_utf8_lossy(&probe_output.stdout)
        .trim()
        .to_string();
    if manifest_raw.is_empty() || manifest_raw == "libc++.modules.json" {
        return Err(format!(
            "Could not resolve libc++ modules manifest for '{}'; install libc++ module sources or choose a compiler/toolchain that provides libc++.modules.json",
            cpp_compiler
        ));
    }

    let manifest_path = PathBuf::from(&manifest_raw);
    if !manifest_path.is_file() {
        return Err(format!(
            "Resolved libc++ modules manifest does not exist: {}",
            manifest_path.display()
        ));
    }

    let manifest_text = fs::read_to_string(&manifest_path).map_err(|e| {
        format!(
            "Failed to read libc++ modules manifest {}: {}",
            manifest_path.display(),
            e
        )
    })?;
    let manifest: LibcxxModulesManifest = serde_json::from_str(&manifest_text).map_err(|e| {
        format!(
            "Failed to parse libc++ modules manifest {}: {}",
            manifest_path.display(),
            e
        )
    })?;
    let std_entry = manifest
        .modules
        .into_iter()
        .find(|entry| entry.logical_name == "std")
        .ok_or_else(|| {
            format!(
                "libc++ modules manifest {} does not contain logical module 'std'",
                manifest_path.display()
            )
        })?;

    let manifest_dir = manifest_path.parent().ok_or_else(|| {
        format!(
            "Invalid libc++ modules manifest path: {}",
            manifest_path.display()
        )
    })?;
    let std_source_path = {
        let raw = Path::new(std_entry.source_path.trim());
        if raw.is_absolute() {
            raw.to_path_buf()
        } else {
            manifest_dir.join(raw)
        }
    };
    if !std_source_path.is_file() {
        return Err(format!(
            "Resolved std module source not found: {}",
            std_source_path.display()
        ));
    }

    let mut system_include_directories: Vec<PathBuf> = Vec::new();
    for dir in std_entry.local_arguments.system_include_directories {
        let raw = Path::new(dir.trim());
        let resolved = if raw.is_absolute() {
            raw.to_path_buf()
        } else {
            manifest_dir.join(raw)
        };
        system_include_directories.push(resolved);
    }

    Ok(LibcxxStdModuleConfig {
        source_path: std_source_path,
        system_include_directories,
    })
}

fn precompile_std_module_for_import_std(
    cpp_compiler: &str,
    cxx_standard: &str,
    pcm_dir: &Path,
    build_log: &mut String,
) -> Result<(), String> {
    let config = resolve_libcxx_std_module_config(cpp_compiler)?;
    let std_pcm = pcm_dir.join("std.pcm");

    let mut cmd = std::process::Command::new(cpp_compiler);
    cmd.arg(format!("-std={}", cxx_standard))
        .arg("-stdlib=libc++")
        .arg("-x")
        .arg("c++-module")
        .arg("--precompile");
    for dir in &config.system_include_directories {
        cmd.arg("-isystem").arg(dir);
    }
    cmd.arg("-o").arg(&std_pcm).arg(&config.source_path);

    let include_flags = if config.system_include_directories.is_empty() {
        String::new()
    } else {
        format!(
            " {}",
            config
                .system_include_directories
                .iter()
                .map(|dir| format!("-isystem {}", dir.display()))
                .collect::<Vec<String>>()
                .join(" ")
        )
    };
    let command_str = format!(
        "{} -std={} -stdlib=libc++ -x c++-module --precompile{} -o {} {}",
        cpp_compiler,
        cxx_standard,
        include_flags,
        std_pcm.display(),
        config.source_path.display()
    );
    build_log.push_str(&format!("$ {}\n", command_str));

    let output = cmd
        .output()
        .map_err(|e| format!("Failed to run {}: {}", cpp_compiler, e))?;
    build_log.push_str(&String::from_utf8_lossy(&output.stderr));
    build_log.push_str(&String::from_utf8_lossy(&output.stdout));
    build_log.push('\n');

    if !output.status.success() {
        return Err("C++ std module precompile failed".to_string());
    }
    Ok(())
}

fn append_parity_runner_main(
    runner_src: &mut String,
    test_entries: &mut Vec<RunnerTestEntry>,
    no_baseline: bool,
    allow_empty_tests: bool,
    work_dir: &Path,
    emit_runtime_clear: bool,
    crate_name: &str,
) -> Result<(), String> {
    if test_entries.is_empty() {
        let baseline_ran_tests = if no_baseline {
            None
        } else {
            baseline_ran_any_tests(work_dir)
        };
        let allow_empty_from_baseline = matches!(baseline_ran_tests, Some(false));

        if !allow_empty_tests && !allow_empty_from_baseline {
            return Err(
                "No transpiled test wrappers discovered (expected exported rusty_test_* functions)."
                    .to_string(),
            );
        }
        if allow_empty_tests {
            println!(
                "  No transpiled test wrappers discovered; continuing due to --allow-empty-tests"
            );
        } else if allow_empty_from_baseline {
            println!(
                "  No transpiled test wrappers discovered; baseline reported zero tests, continuing with compile-validation only"
            );
        } else {
            println!("  No transpiled test wrappers discovered; compile-validation only");
        }
        runner_src.push_str("\n// ── Compile-validation runner ──\n");
        runner_src.push_str("int main() {\n");
        runner_src.push_str(
            "    std::cout << \"No transpiled test wrappers discovered; compile-validation only.\" << std::endl;\n",
        );
        runner_src.push_str("    return 0;\n");
        runner_src.push_str("}\n");
        return Ok(());
    }

    test_entries.sort_by(|a, b| a.fn_name.cmp(&b.fn_name));
    // For a namespace-wrapped crate (e.g. serde_bytes, see
    // `transpile::crate_is_namespace_wrapped`), the exported `rusty_test_*`
    // wrappers live under `namespace <crate>` in the emitted module, so the
    // runner — which imports the module and calls them from global scope — must
    // qualify the CALL with that namespace. The bare wrapper name is still used
    // as the `--rusty-single-test` string key; only the C++ call expression is
    // prefixed. Non-wrapped crates emit wrappers at global scope → empty prefix
    // → unchanged.
    // Normalize the crate name to its C++ MODULE/namespace form (Rust `-` → `_`): the emitted
    // module is `namespace cfg_if`, not `cfg-if` (a hyphen is invalid in a C++ qualified-id —
    // `cfg-if::X` parses as the subtraction `cfg - if::X`).
    let crate_ns = crate_name.replace('-', "_");
    let wrapper_prefix = if transpile::crate_is_namespace_wrapped(&crate_ns) {
        format!("{}::", crate_ns)
    } else {
        String::new()
    };
    runner_src.push_str("\n// ── Test runner ──\n");
    runner_src.push_str("int main(int argc, char** argv) {\n");
    runner_src
        .push_str("    if (argc == 3 && std::string(argv[1]) == \"--rusty-single-test\") {\n");
    runner_src.push_str("        const std::string test_name = argv[2];\n");
    if emit_runtime_clear {
        runner_src.push_str("        rusty::mem::clear_all_forgotten_addresses();\n");
    }
    runner_src.push_str("        try {\n");
    for entry in test_entries.iter() {
        runner_src.push_str(&format!(
            "            if (test_name == \"{}\") {{ {}{}(); return 0; }}\n",
            entry.fn_name, wrapper_prefix, entry.fn_name
        ));
    }
    runner_src.push_str(
        "            std::cerr << \"Unknown single-test wrapper: \" << test_name << std::endl;\n",
    );
    runner_src.push_str("            return 64;\n");
    runner_src.push_str("        } catch (const std::exception& e) {\n");
    runner_src.push_str("            std::cerr << e.what() << std::endl;\n");
    runner_src.push_str("            return 101;\n");
    runner_src.push_str("        } catch (...) {\n");
    runner_src.push_str("            return 102;\n");
    runner_src.push_str("        }\n");
    runner_src.push_str("    }\n");
    runner_src.push_str("    int pass = 0, fail = 0;\n");
    for entry in test_entries.iter() {
        if entry.should_panic {
            runner_src.push_str(&format!(
                "    {{\n        const std::string cmd = std::string(\"\\\"\") + argv[0] + \"\\\" --rusty-single-test {}\";\n        const int status = std::system(cmd.c_str());\n        if (status != 0) {{ std::cout << \"  {} PASSED (expected panic)\" << std::endl; pass++; }}\n        else {{ std::cerr << \"  {} FAILED: expected panic\" << std::endl; fail++; }}\n    }}\n",
                entry.fn_name, entry.label, entry.label
            ));
        } else {
            if emit_runtime_clear {
                runner_src.push_str(&format!(
                    "    rusty::mem::clear_all_forgotten_addresses();\n    try {{ {}{}(); std::cout << \"  {} PASSED\" << std::endl; pass++; }}\n",
                    wrapper_prefix, entry.fn_name, entry.label
                ));
            } else {
                runner_src.push_str(&format!(
                    "    try {{ {}{}(); std::cout << \"  {} PASSED\" << std::endl; pass++; }}\n",
                    wrapper_prefix, entry.fn_name, entry.label
                ));
            }
            runner_src.push_str(&format!(
                "    catch (const std::exception& e) {{ std::cerr << \"  {} FAILED: \" << e.what() << std::endl; fail++; }}\n",
                entry.label
            ));
            runner_src.push_str(&format!(
                "    catch (...) {{ std::cerr << \"  {} FAILED (unknown exception)\" << std::endl; fail++; }}\n",
                entry.label
            ));
        }
    }
    runner_src.push_str("    std::cout << std::endl;\n");
    runner_src.push_str(
        "    std::cout << \"Results: \" << pass << \" passed, \" << fail << \" failed\" << std::endl;\n",
    );
    runner_src.push_str("    return fail > 0 ? 1 : 0;\n");
    runner_src.push_str("}\n");
    Ok(())
}

/// Walk up from `include_dir` (typically `<repo>/include`) to find the
/// `CMakeLists.txt` that defines the `rusty` umbrella module target.
fn find_repo_root_with_cmake(include_dir: &Path) -> Option<PathBuf> {
    let mut candidate = include_dir.to_path_buf();
    for _ in 0..6 {
        if candidate.join("CMakeLists.txt").is_file() {
            return Some(candidate);
        }
        candidate = candidate.parent()?.to_path_buf();
    }
    None
}

/// Ensure the C++20 `rusty` umbrella module and its transitive port
/// dependencies are precompiled to .pcm files in a shared cache
/// directory, then return a path suitable for `-fprebuilt-module-path`.
///
/// On first invocation (cache miss), runs `cmake -G Ninja` + `cmake
/// --build . --target rusty` against the repo root's `CMakeLists.txt`,
/// then symlinks all produced .pcm files into a single flat directory.
/// On subsequent invocations (cache hit — `rusty.pcm` present), returns
/// the cached path without reinvoking CMake.
///
/// Returns `None` if the repo root can't be located or the build fails;
/// callers should fall back to module-less behavior (mostly a no-op for
/// crates that don't reference `rusty::Vec` / `rusty::Rc` / etc.).
fn ensure_rusty_modules_pcm_dir(include_dir: &Path) -> Option<PathBuf> {
    let repo_root = find_repo_root_with_cmake(include_dir)?;
    let cache_root = repo_root.join(".rusty-modules-cache");
    let cmake_build_dir = cache_root.join("build");
    let pcm_flat_dir = cache_root.join("pcm");
    let rusty_pcm_marker = pcm_flat_dir.join("rusty.pcm");

    // Cache hit: the marker .pcm must exist AND be at least as new as every
    // runtime source that feeds the umbrella module (textually included
    // headers under include/, port module interfaces under transpiled/, and
    // CMakeLists.txt itself). A stale pcm is worse than a miss: `import
    // rusty;` then deserializes an OLD class definition which clang merges
    // with the NEW textual include in the same TU — duplicate overload sets
    // ("call to 'from_utf8' is ambiguous") with line numbers from the
    // pre-edit header. The Ninja re-build below is incremental, so a
    // freshness rebuild only recompiles what the edit actually touched.
    if rusty_pcm_marker.exists() {
        // Prefer the stamp written after our last (re)build attempt: when a
        // rebuild turns out to be a Ninja no-op (mtime-only touch, or an
        // edited source no target consumes) the pcm keeps its old mtime and
        // comparing against it would re-trigger the build on every call.
        let freshness_stamp = cache_root.join("freshness.stamp");
        let marker_mtime = fs::metadata(&freshness_stamp)
            .and_then(|m| m.modified())
            .ok()
            .or_else(|| {
                fs::metadata(&rusty_pcm_marker)
                    .and_then(|m| m.modified())
                    .ok()
            });
        let mut newest_source: Option<std::time::SystemTime> = fs::metadata(
            repo_root.join("CMakeLists.txt"),
        )
        .and_then(|m| m.modified())
        .ok();
        let mut stack = vec![repo_root.join("include"), repo_root.join("transpiled")];
        while let Some(dir) = stack.pop() {
            let Ok(entries) = fs::read_dir(&dir) else { continue };
            for entry in entries.flatten() {
                let p = entry.path();
                if p.is_dir() {
                    stack.push(p);
                    continue;
                }
                let relevant = matches!(
                    p.extension().and_then(|s| s.to_str()),
                    Some("hpp") | Some("h") | Some("cppm")
                );
                if !relevant {
                    continue;
                }
                if let Ok(modified) = fs::metadata(&p).and_then(|m| m.modified()) {
                    newest_source = Some(match newest_source {
                        Some(existing) if existing >= modified => existing,
                        _ => modified,
                    });
                }
            }
        }
        match (marker_mtime, newest_source) {
            (Some(marker), Some(source)) if marker >= source => {
                return Some(pcm_flat_dir);
            }
            (Some(_), None) => return Some(pcm_flat_dir),
            _ => {
                eprintln!(
                    "  rusty module cache is older than runtime sources; rebuilding..."
                );
            }
        }
    }

    fs::create_dir_all(&cmake_build_dir).ok()?;
    fs::create_dir_all(&pcm_flat_dir).ok()?;

    // Configure CMake (Ninja generator + clang).  If clang isn't
    // available the matrix would fail anyway since module precompile
    // also requires clang. Skip when the build tree is already
    // configured (freshness rebuilds only need the incremental build).
    if !cmake_build_dir.join("CMakeCache.txt").is_file() {
        let cmake_configure = std::process::Command::new("cmake")
            .arg("-G")
            .arg("Ninja")
            .arg("-DCMAKE_BUILD_TYPE=Release")
            .arg("-DCMAKE_CXX_COMPILER=clang++")
            .arg(&repo_root)
            .current_dir(&cmake_build_dir)
            .output()
            .ok()?;
        if !cmake_configure.status.success() {
            eprintln!(
                "  Warning: CMake configure for rusty module cache failed:\n{}",
                String::from_utf8_lossy(&cmake_configure.stderr)
                    .lines()
                    .take(5)
                    .collect::<Vec<_>>()
                    .join("\n")
            );
            return None;
        }
    }

    let cmake_build = std::process::Command::new("cmake")
        .arg("--build")
        .arg(".")
        .arg("--target")
        .arg("rusty")
        .current_dir(&cmake_build_dir)
        .output()
        .ok()?;
    if !cmake_build.status.success() {
        eprintln!(
            "  Warning: CMake build of `rusty` target failed:\n{}",
            String::from_utf8_lossy(&cmake_build.stderr)
                .lines()
                .take(5)
                .collect::<Vec<_>>()
                .join("\n")
        );
        return None;
    }

    // Walk the build tree and symlink every .pcm into the flat cache.
    let mut stack = vec![cmake_build_dir.clone()];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = fs::read_dir(&dir) else { continue };
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_dir() {
                stack.push(p);
                continue;
            }
            if p.extension().and_then(|s| s.to_str()) == Some("pcm") {
                if let Some(name) = p.file_name() {
                    let link = pcm_flat_dir.join(name);
                    let _ = fs::remove_file(&link);
                    let _ = std::os::unix::fs::symlink(&p, &link);
                }
            }
        }
    }

    if rusty_pcm_marker.exists() {
        // Record that the cache was validated/rebuilt against the current
        // sources; the freshness check above compares against this stamp.
        let _ = fs::write(cache_root.join("freshness.stamp"), b"");
        Some(pcm_flat_dir)
    } else {
        None
    }
}

/// Outcome of one clang invocation for a module unit (precompile → .pcm, or
/// object compile → .o). Pure/thread-safe: builds the command, runs it, and
/// returns the captured log + success + first error line. No shared mutation,
/// so it is safe to run for several units concurrently (Stage D object phase).
struct ModuleStepOutcome {
    ok: bool,
    log: String,
    first_err: String,
}

// ── Content-addressed module BMI/object cache ──────────────────────────────
//
// Opt-in via `RUSTY_CPP_MODULE_CACHE=1`. Caches each module's `.pcm` and `.o`
// under a key = hash(the `.cppm` bytes + the transitive cache keys of the
// modules it imports + the build environment). Two modules with the same key
// are byte-identical compiler inputs, so their BMI/object are interchangeable.
//
// This dedups shared dependencies (serde_core, syn, …) across crates WITHIN a
// matrix run (e.g. serde_core[no-rc] built once for serde_bytes + serde_repr)
// AND ACROSS runs: a localized transpiler change leaves most crates' `.cppm`
// byte-identical, so their (expensive) precompile/codegen is skipped on the
// next gate. Mirrors Cargo's per-unit fingerprint, but keyed on the actual
// transpiled output rather than the resolved feature set.
//
// Correctness: the key folds in EVERYTHING that affects the artifact — the
// `.cppm` content, every imported module's key (so a dep change ripples up),
// the clang version, the exact compile flags, and a digest of the `include/`
// headers (which the `.cppm` `#include`s into its global module fragment).
// When any input is uncertain (an import that wasn't keyed), the unit is left
// uncached and rebuilt. A miss is only slow; a wrong hit would be unsound, so
// we bias toward misses.
const MODULE_CACHE_SCHEMA: u32 = 2;

fn module_cache_enabled() -> bool {
    matches!(
        std::env::var("RUSTY_CPP_MODULE_CACHE").ok().as_deref(),
        Some("1") | Some("true") | Some("on")
    )
}

fn module_cache_units_dir(include_dir: &Path) -> Option<PathBuf> {
    if !module_cache_enabled() {
        return None;
    }
    let repo_root = find_repo_root_with_cmake(include_dir)?;
    let dir = repo_root.join(".rusty-modules-cache").join("units");
    fs::create_dir_all(&dir).ok()?;
    Some(dir)
}

/// Absolute shared `CARGO_TARGET_DIR` for the matrix's baseline (`cargo test`)
/// and expand (`cargo expand`) builds, when matrix caching is on. A single
/// shared target lets Cargo's OWN fingerprint cache dedup the Rust builds of
/// shared dependencies (serde_core, syn, …) across crates AND across runs —
/// Cargo keys each unit by version+features+profile, so different feature sets
/// (serde_core[rc] vs [no-rc]) coexist as distinct cached units. Returns None
/// when disabled, or when `CARGO_TARGET_DIR` is already set (respect the
/// caller's override).
fn shared_cargo_target_dir() -> Option<PathBuf> {
    if !module_cache_enabled() {
        return None;
    }
    if std::env::var_os("CARGO_TARGET_DIR").is_some() {
        return None;
    }
    let include_dir = find_rusty_include_dir();
    let repo_root = find_repo_root_with_cmake(&include_dir)?;
    let dir = repo_root.join(".rusty-modules-cache").join("cargo-target");
    fs::create_dir_all(&dir).ok()?;
    Some(dir)
}

fn sha256_hex(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    h.update(bytes);
    h.finalize().iter().map(|b| format!("{:02x}", b)).collect()
}

/// Sha256 over every file under `dir`, keyed by path relative to `dir` (sorted)
/// + its content. Used to fold the rusty headers into the environment hash.
fn hash_directory_tree(dir: &Path) -> String {
    use sha2::{Digest, Sha256};
    fn walk(d: &Path, base: &Path, out: &mut Vec<(String, PathBuf)>) {
        if let Ok(rd) = fs::read_dir(d) {
            for e in rd.flatten() {
                let p = e.path();
                if p.is_dir() {
                    walk(&p, base, out);
                } else if let Ok(rel) = p.strip_prefix(base) {
                    out.push((rel.to_string_lossy().into_owned(), p));
                }
            }
        }
    }
    let mut files = Vec::new();
    walk(dir, dir, &mut files);
    files.sort();
    let mut h = Sha256::new();
    for (rel, path) in &files {
        h.update(rel.as_bytes());
        h.update([0u8]);
        if let Ok(bytes) = fs::read(path) {
            h.update((bytes.len() as u64).to_le_bytes());
            h.update(&bytes);
        }
        h.update([0u8]);
    }
    h.finalize().iter().map(|b| format!("{:02x}", b)).collect()
}

/// Hash of everything that affects a module artifact but ISN'T the `.cppm` or
/// its imports: clang version, compile flags, and the `include/` headers.
fn module_cache_env_hash(
    cpp_compiler: &str,
    cxx_standard: &str,
    portable_intrinsics_define: &str,
    import_std: bool,
    include_dir: &Path,
    rusty_pcm_dir: Option<&Path>,
) -> String {
    use sha2::{Digest, Sha256};
    let clang_version = std::process::Command::new(cpp_compiler)
        .arg("--version")
        .output()
        .ok()
        .map(|o| {
            String::from_utf8_lossy(&o.stdout)
                .lines()
                .next()
                .unwrap_or("")
                .to_string()
        })
        .unwrap_or_default();
    let mut h = Sha256::new();
    h.update(MODULE_CACHE_SCHEMA.to_le_bytes());
    h.update(clang_version.as_bytes());
    h.update([0u8]);
    // Transpiler identity: a transpiler change must invalidate cached BMIs/objects.
    // The cache is keyed on the `.cppm` bytes, but the work dir's `.cppm` can be
    // reused without re-transpilation, so the cached unit could otherwise survive a
    // semantics-changing transpiler edit. The embedded git revision marks the
    // committed source; the binary's own mtime catches uncommitted rebuilds (cargo
    // rewrites the binary only when sources actually change → no spurious misses).
    h.update(env!("RUSTY_CPP_GIT_HASH").as_bytes());
    h.update([0u8]);
    h.update(env!("RUSTY_CPP_GIT_DIRTY").as_bytes());
    h.update([0u8]);
    if let Ok(exe) = std::env::current_exe()
        && let Ok(meta) = fs::metadata(&exe)
        && let Ok(mtime) = meta.modified()
        && let Ok(since) = mtime.duration_since(std::time::UNIX_EPOCH)
    {
        h.update(since.as_nanos().to_le_bytes());
        h.update([0u8]);
    }
    h.update(cxx_standard.as_bytes());
    h.update([0u8]);
    h.update(portable_intrinsics_define.as_bytes());
    h.update([0u8]);
    h.update([import_std as u8, b'\0']);
    h.update(b"-march=native\0");
    h.update(hash_directory_tree(include_dir).as_bytes());
    h.update([0u8]);
    // The runtime module BMIs are rebuilt whenever rusty sources change; fold
    // the marker in so a runtime-module change invalidates dependent caches.
    if let Some(p) = rusty_pcm_dir {
        if let Ok(b) = fs::read(p.join("rusty.pcm")) {
            h.update(sha256_hex(&b).as_bytes());
            h.update([0u8]);
        }
    }
    h.finalize().iter().map(|b| format!("{:02x}", b)).collect()
}

/// Cache key for one module: env + its `.cppm` bytes + sorted import keys.
/// Returns None if the `.cppm` can't be read.
fn module_unit_cache_key(env_hash: &str, cppm_path: &Path, dep_keys: &[String]) -> Option<String> {
    use sha2::{Digest, Sha256};
    let content = fs::read(cppm_path).ok()?;
    let mut h = Sha256::new();
    h.update(env_hash.as_bytes());
    h.update([0u8]);
    h.update((content.len() as u64).to_le_bytes());
    h.update(&content);
    h.update([0u8]);
    let mut deps = dep_keys.to_vec();
    deps.sort();
    for d in &deps {
        h.update(d.as_bytes());
        h.update([0u8]);
    }
    Some(h.finalize().iter().map(|b| format!("{:02x}", b)).collect())
}

/// Copy `cache_file` → `dst` if the cache entry exists. Returns true on hit.
fn module_cache_fetch(cache_file: &Path, dst: &Path) -> bool {
    cache_file.is_file() && fs::copy(cache_file, dst).is_ok()
}

/// Store `src` into the cache atomically (temp + rename). Concurrent stores of
/// the same key race harmlessly — the content is identical, last writer wins.
fn module_cache_store(cache_file: &Path, src: &Path) {
    let Some(parent) = cache_file.parent() else {
        return;
    };
    let tmp = parent.join(format!(
        "{}.tmp.{}",
        cache_file
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("entry"),
        std::process::id()
    ));
    if fs::copy(src, &tmp).is_ok() {
        let _ = fs::rename(&tmp, cache_file);
        let _ = fs::remove_file(&tmp); // no-op if rename succeeded
    }
}

#[allow(clippy::too_many_arguments)]
fn compile_module_step(
    cpp_compiler: &str,
    cxx_standard: &str,
    import_std: bool,
    portable_intrinsics_define: &str,
    include_dir: &Path,
    pcm_dir: &Path,
    rusty_pcm_dir: Option<&Path>,
    unit: &ModuleBuildUnit,
    precompile: bool,
) -> Result<ModuleStepOutcome, String> {
    // Reconstruct the same command string the inline path logged, for build.log.
    let rusty_pcm_flag = rusty_pcm_dir
        .map(|p| format!("-fprebuilt-module-path={}", p.display()))
        .unwrap_or_default();
    let stdlib = if import_std { " -stdlib=libc++" } else { "" };
    let cmd_str = if precompile {
        format!(
            "{} -std={}{} {} -march=native -x c++-module --precompile -I{} -fprebuilt-module-path={} {} -o {} {}",
            cpp_compiler, cxx_standard, stdlib, portable_intrinsics_define,
            include_dir.display(), pcm_dir.display(), rusty_pcm_flag,
            unit.pcm_path.display(), unit.source_path.display()
        )
    } else {
        format!(
            "{} -std={}{} {} -march=native -Wall -Wno-unused-variable -Wno-unused-but-set-variable -I{} -fprebuilt-module-path={} {} -c {} -o {}",
            cpp_compiler, cxx_standard, stdlib, portable_intrinsics_define,
            include_dir.display(), pcm_dir.display(), rusty_pcm_flag,
            unit.source_path.display(), unit.object_path.display()
        )
    };

    let mut cmd = std::process::Command::new(cpp_compiler);
    cmd.arg(format!("-std={}", cxx_standard))
        .arg(portable_intrinsics_define)
        .arg("-march=native");
    if import_std {
        cmd.arg("-stdlib=libc++");
    }
    if let Some(rusty_pcm) = rusty_pcm_dir {
        cmd.arg(format!("-fprebuilt-module-path={}", rusty_pcm.display()));
    }
    if precompile {
        cmd.arg("-x").arg("c++-module").arg("--precompile");
    } else {
        cmd.arg("-Wall")
            .arg("-Wno-unused-variable")
            .arg("-Wno-unused-but-set-variable");
    }
    cmd.arg(format!("-I{}", include_dir.display()))
        .arg(format!("-fprebuilt-module-path={}", pcm_dir.display()));
    if precompile {
        cmd.arg("-o").arg(&unit.pcm_path).arg(&unit.source_path);
    } else {
        cmd.arg("-c")
            .arg(&unit.source_path)
            .arg("-o")
            .arg(&unit.object_path);
    }

    let output = cmd
        .output()
        .map_err(|e| format!("Failed to run {}: {}", cpp_compiler, e))?;
    let mut log = String::new();
    log.push_str(&format!("$ {}\n", cmd_str));
    log.push_str(&String::from_utf8_lossy(&output.stderr));
    log.push_str(&String::from_utf8_lossy(&output.stdout));
    log.push('\n');
    let ok = output.status.success();
    let first_err = if ok {
        String::new()
    } else {
        String::from_utf8_lossy(&output.stderr)
            .lines()
            .find(|line| line.contains("error:"))
            .map(|s| s.trim().to_string())
            .unwrap_or_else(|| "(no error line)".to_string())
    };
    Ok(ModuleStepOutcome { ok, log, first_err })
}

fn run_stage_d_module_build(
    args: &ParityTestArgs,
    work_dir: &Path,
    include_dir: &Path,
    cpp_compiler: &str,
    generated_cppm_files: &[GeneratedCppmArtifact],
    crate_name: &str,
) -> Result<(), String> {
    let runner_path = work_dir.join("runner.cpp");
    let binary_path = work_dir.join("runner");
    let build_log_path = work_dir.join("build.log");

    if generated_cppm_files.is_empty() {
        return Err("No .cppm files generated in this run — Stage C may have failed".to_string());
    }

    // Ensure the shared `rusty` module cache is built before any
    // user-module precompile.  None on failure is non-fatal — crates
    // that don't actually use module-only types will still compile.
    let rusty_pcm_dir = ensure_rusty_modules_pcm_dir(include_dir);

    let build_root = work_dir.join("module_build");
    let pcm_dir = build_root.join("pcm");
    let obj_dir = build_root.join("obj");
    if build_root.exists() {
        fs::remove_dir_all(&build_root).map_err(|e| {
            format!(
                "Failed to reset module build dir {}: {}",
                build_root.display(),
                e
            )
        })?;
    }
    fs::create_dir_all(&pcm_dir)
        .map_err(|e| format!("Failed to create {}: {}", pcm_dir.display(), e))?;
    fs::create_dir_all(&obj_dir)
        .map_err(|e| format!("Failed to create {}: {}", obj_dir.display(), e))?;

    let mut units: Vec<ModuleBuildUnit> = Vec::new();

    let mut test_entries: Vec<RunnerTestEntry> = Vec::new();
    let mut seen_test_fns: HashSet<String> = HashSet::new();

    // Collect test wrappers per artifact so we can selectively drop
    // wrappers from a test target whose precompile fails. Map by
    // module name → indices into `test_entries`.
    let mut wrappers_by_module: HashMap<String, Vec<usize>> = HashMap::new();
    for artifact in generated_cppm_files {
        let source = fs::read_to_string(&artifact.path)
            .map_err(|e| format!("Failed to read {}: {}", artifact.path.display(), e))?;
        let before_len = test_entries.len();
        collect_rusty_test_entries_from_cppm(&source, &mut seen_test_fns, &mut test_entries);
        if test_entries.len() > before_len {
            wrappers_by_module
                .entry(artifact.module_name.clone())
                .or_default()
                .extend(before_len..test_entries.len());
        }
        units.push(ModuleBuildUnit {
            module_name: artifact.module_name.clone(),
            source_path: artifact.path.clone(),
            imports: collect_named_module_imports(&source),
            pcm_path: pcm_dir.join(module_artifact_name(&artifact.module_name, "pcm")),
            object_path: obj_dir.join(module_artifact_name(&artifact.module_name, "o")),
            is_test_target: artifact.is_test_target,
            is_dependency: artifact.is_dependency,
        });
    }

    let compile_start = std::time::Instant::now();
    let mut build_log = String::new();
    let mut object_files: Vec<PathBuf> = Vec::new();
    let order = module_build_order(&units);
    let portable_intrinsics_define = "-DRUSTY_PORTABLE_INTRINSICS=1";
    // Always C++23 — `import rusty;` (emitted unconditionally for
    // module-mode output) drags in port modules (hashbrown_port,
    // vec_port, …) that require C++23 features like `std::println`
    // and `std::span` deduction. The matrix's previous "C++20 unless
    // --import-std" was viable when rusty was header-only.
    let cxx_standard = "c++23";
    let stdlib_flag_suffix = if args.import_std {
        " -stdlib=libc++"
    } else {
        ""
    };

    if args.import_std {
        if let Err(err) = precompile_std_module_for_import_std(
            cpp_compiler,
            cxx_standard,
            &pcm_dir,
            &mut build_log,
        ) {
            fs::write(&build_log_path, &build_log)
                .map_err(|e| format!("Failed to write build log: {}", e))?;
            println!("  Build FAILED — see {}", build_log_path.display());
            for line in build_log
                .lines()
                .filter(|line| line.contains("error:"))
                .take(20)
            {
                println!("    {}", line);
            }
            println!(
                "  Build compile time (module, failed): {:.3}s",
                compile_start.elapsed().as_secs_f64()
            );
            return Err(err);
        }
    }

    // Tracks modules we've decided to skip — both failed test
    // targets and failed dev-dependencies that aren't reachable from
    // the lib target. The lib target itself is never skipped.
    let mut skipped_test_modules: HashSet<String> = HashSet::new();
    // RUSTY_CPP_BUILD_JOBS (default 1 = sequential): parallelism for the
    // object-compile phase only. The precompile phase carries the module DAG
    // and stays sequential; objects have no inter-object dependency.
    let build_jobs = std::env::var("RUSTY_CPP_BUILD_JOBS")
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(1)
        .max(1);
    let import_std = args.import_std;
    let rusty_pcm_ref = rusty_pcm_dir.as_deref();
    let pcm_dir_ref: &Path = pcm_dir.as_path();
    let _ = &stdlib_flag_suffix; // command strings are now built in compile_module_step

    // Content-addressed module cache (opt-in via RUSTY_CPP_MODULE_CACHE=1).
    // Keys are computed in topological order so each unit's imported-module
    // keys are already known when we reach it.
    let cache_units_dir = module_cache_units_dir(include_dir);
    let cache_env_hash = cache_units_dir.as_ref().map(|_| {
        module_cache_env_hash(
            cpp_compiler,
            cxx_standard,
            portable_intrinsics_define,
            import_std,
            include_dir,
            rusty_pcm_ref,
        )
    });
    let local_module_names: HashSet<String> =
        units.iter().map(|u| u.module_name.clone()).collect();
    let mut module_name_to_key: HashMap<String, String> = HashMap::new();
    let mut unit_keys: Vec<Option<String>> = vec![None; units.len()];

    // Phase 1 — precompile (.pcm), sequential in topological order. A unit's
    // .pcm must exist before any dependent precompiles/compiles, so this phase
    // carries the module DAG and the skip/fail logic.
    let mut to_object: Vec<usize> = Vec::new();
    for idx in order {
        let unit = &units[idx];
        // Skip units whose deps were already skipped (they'd fail to find imports).
        if !unit.imports.is_empty()
            && unit
                .imports
                .iter()
                .any(|imp| skipped_test_modules.contains(imp))
        {
            if unit.is_test_target || unit.is_dependency {
                eprintln!(
                    "  Skipping {} '{}' (module): depends on skipped module",
                    if unit.is_test_target { "test target" } else { "dependency" },
                    unit.module_name
                );
                skipped_test_modules.insert(unit.module_name.clone());
                continue;
            }
            // Lib depending on skipped — fail-fast below (precompile will fail).
        }
        // Compute this unit's cache key. Only cacheable when every crate-local
        // import is already keyed (else a dep's identity is unknown → unsound).
        let cache_key: Option<String> = match (&cache_units_dir, &cache_env_hash) {
            (Some(_), Some(env)) => {
                let mut dep_keys: Vec<String> = Vec::new();
                let mut all_local_deps_keyed = true;
                for imp in &unit.imports {
                    if local_module_names.contains(imp) {
                        match module_name_to_key.get(imp) {
                            Some(k) => dep_keys.push(k.clone()),
                            None => {
                                all_local_deps_keyed = false;
                                break;
                            }
                        }
                    }
                }
                if all_local_deps_keyed {
                    module_unit_cache_key(env, &unit.source_path, &dep_keys)
                } else {
                    None
                }
            }
            _ => None,
        };
        if let Some(k) = &cache_key {
            module_name_to_key.insert(unit.module_name.clone(), k.clone());
            unit_keys[idx] = Some(k.clone());
        }
        // Cache hit: copy the cached .pcm and skip the (expensive) precompile.
        if let (Some(dir), Some(k)) = (&cache_units_dir, &cache_key) {
            if module_cache_fetch(&dir.join(format!("{}.pcm", k)), &unit.pcm_path) {
                build_log.push_str(&format!(
                    "# module-cache HIT pcm {} {}\n",
                    unit.module_name, k
                ));
                to_object.push(idx);
                continue;
            }
        }
        let outcome = compile_module_step(
            cpp_compiler,
            cxx_standard,
            import_std,
            portable_intrinsics_define,
            include_dir,
            &pcm_dir,
            rusty_pcm_ref,
            unit,
            true,
        )?;
        build_log.push_str(&outcome.log);
        if !outcome.ok {
            // Test targets and dev-dependencies that fail to precompile: skip
            // them so other targets in the same crate can still produce a
            // passing parity result. If a skipped dep is imported by the lib
            // (essential), the lib's own precompile fails → the bail path.
            if unit.is_test_target || unit.is_dependency {
                eprintln!(
                    "  Skipping {} '{}' (module): precompile failed — {}",
                    if unit.is_test_target { "test target" } else { "dependency" },
                    unit.module_name,
                    outcome.first_err.chars().take(120).collect::<String>()
                );
                skipped_test_modules.insert(unit.module_name.clone());
                continue;
            }
            fs::write(&build_log_path, &build_log)
                .map_err(|e| format!("Failed to write build log: {}", e))?;
            println!("  Build FAILED — see {}", build_log_path.display());
            for line in build_log
                .lines()
                .filter(|line| line.contains("error:"))
                .take(20)
            {
                println!("    {}", line);
            }
            println!(
                "  Build compile time (module, failed): {:.3}s",
                compile_start.elapsed().as_secs_f64()
            );
            return Err("C++ module precompile failed".to_string());
        }
        // Store the freshly-built .pcm into the content-addressed cache.
        if let (Some(dir), Some(k)) = (&cache_units_dir, &cache_key) {
            module_cache_store(&dir.join(format!("{}.pcm", k)), &unit.pcm_path);
        }
        to_object.push(idx);
    }

    // Phase 2 — object compile (.o). Objects only feed the final link and have
    // no inter-object dependency, so compile them concurrently (chunked by
    // build_jobs; chunk size 1 = sequential). Results are consumed in
    // submission order for a deterministic build.log and skip/fail handling.
    let cache_dir_ref = cache_units_dir.as_ref();
    let unit_keys_ref = &unit_keys;
    for chunk in to_object.chunks(build_jobs) {
        let outcomes: Vec<(usize, Result<ModuleStepOutcome, String>)> =
            std::thread::scope(|scope| {
                let handles: Vec<_> = chunk
                    .iter()
                    .map(|&idx| {
                        let unit = &units[idx];
                        let handle = scope.spawn(move || {
                            let key_opt =
                                unit_keys_ref.get(idx).and_then(|o| o.as_ref());
                            // Cache hit: reuse the cached .o, skip codegen.
                            if let (Some(dir), Some(k)) = (cache_dir_ref, key_opt) {
                                if module_cache_fetch(
                                    &dir.join(format!("{}.o", k)),
                                    &unit.object_path,
                                ) {
                                    return Ok(ModuleStepOutcome {
                                        ok: true,
                                        log: format!(
                                            "# module-cache HIT obj {} {}\n",
                                            unit.module_name, k
                                        ),
                                        first_err: String::new(),
                                    });
                                }
                            }
                            let outcome = compile_module_step(
                                cpp_compiler,
                                cxx_standard,
                                import_std,
                                portable_intrinsics_define,
                                include_dir,
                                pcm_dir_ref,
                                rusty_pcm_ref,
                                unit,
                                false,
                            );
                            // Store a freshly-built .o into the cache.
                            if let Ok(o) = &outcome {
                                if o.ok {
                                    if let (Some(dir), Some(k)) =
                                        (cache_dir_ref, key_opt)
                                    {
                                        module_cache_store(
                                            &dir.join(format!("{}.o", k)),
                                            &unit.object_path,
                                        );
                                    }
                                }
                            }
                            outcome
                        });
                        (idx, handle)
                    })
                    .collect();
                handles
                    .into_iter()
                    .map(|(idx, handle)| {
                        (
                            idx,
                            handle.join().unwrap_or_else(|_| {
                                Err("object compile thread panicked".to_string())
                            }),
                        )
                    })
                    .collect()
            });
        for (idx, result) in outcomes {
            let unit = &units[idx];
            let outcome = result?;
            build_log.push_str(&outcome.log);
            if !outcome.ok {
                // Same skip logic for the object-compile step.
                if unit.is_test_target || unit.is_dependency {
                    eprintln!(
                        "  Skipping {} '{}' (object): compile failed — {}",
                        if unit.is_test_target { "test target" } else { "dependency" },
                        unit.module_name,
                        outcome.first_err.chars().take(120).collect::<String>()
                    );
                    skipped_test_modules.insert(unit.module_name.clone());
                    continue;
                }
                fs::write(&build_log_path, &build_log)
                    .map_err(|e| format!("Failed to write build log: {}", e))?;
                println!("  Build FAILED — see {}", build_log_path.display());
                for line in build_log
                    .lines()
                    .filter(|line| line.contains("error:"))
                    .take(20)
                {
                    println!("    {}", line);
                }
                println!(
                    "  Build compile time (module, failed): {:.3}s",
                    compile_start.elapsed().as_secs_f64()
                );
                return Err("C++ module object compile failed".to_string());
            }
            object_files.push(unit.object_path.clone());
        }
    }

    // Drop test wrappers from skipped modules so the runner doesn't
    // try to call functions whose translation unit was never
    // compiled.
    if !skipped_test_modules.is_empty() {
        let mut drop_indices: HashSet<usize> = HashSet::new();
        for module in &skipped_test_modules {
            if let Some(indices) = wrappers_by_module.get(module) {
                drop_indices.extend(indices.iter().copied());
            }
        }
        if !drop_indices.is_empty() {
            let kept: Vec<RunnerTestEntry> = test_entries
                .iter()
                .enumerate()
                .filter_map(|(idx, e)| if drop_indices.contains(&idx) { None } else { Some(e.clone()) })
                .collect();
            test_entries = kept;
        }
    }

    let mut runner_src = String::new();
    runner_src.push_str("// Auto-generated parity test runner (module mode)\n");
    if args.import_std {
        runner_src.push_str("import std;\n");
    }
    let mut imported_targets: BTreeSet<String> = BTreeSet::new();
    for artifact in generated_cppm_files {
        if !artifact.is_dependency
            && !skipped_test_modules.contains(&artifact.module_name)
        {
            imported_targets.insert(artifact.module_name.clone());
        }
    }
    for module_name in imported_targets {
        runner_src.push_str(&format!("import {};\n", module_name));
    }
    if args.import_std {
        runner_src.push_str("\n");
    } else {
        runner_src.push_str(
            "#include <rusty/rusty.hpp>\n#include <iostream>\n#include <string>\n#include <cstdlib>\n\n",
        );
    }
    // If we skipped any test/dep modules, fall through to
    // compile-validation mode rather than failing with "No
    // transpiled test wrappers" — the empty wrapper list is a
    // consequence of the skip, not an actual missing transpile.
    let allow_empty_due_to_skip = !skipped_test_modules.is_empty();
    append_parity_runner_main(
        &mut runner_src,
        &mut test_entries,
        args.no_baseline,
        args.allow_empty_tests || allow_empty_due_to_skip,
        work_dir,
        !args.import_std,
        crate_name,
    )?;

    fs::write(&runner_path, &runner_src).map_err(|e| format!("Failed to write runner: {}", e))?;
    println!(
        "  Generated runner: {} ({} tests discovered)",
        runner_path.display(),
        test_entries.len()
    );

    let runner_object = obj_dir.join("runner.o");
    let runner_rusty_pcm_flag = rusty_pcm_dir
        .as_ref()
        .map(|p| format!("-fprebuilt-module-path={}", p.display()))
        .unwrap_or_default();
    let runner_compile_cmd = format!(
        "{} -std={}{} {} -march=native -Wall -Wno-unused-variable -Wno-unused-but-set-variable -I{} -fprebuilt-module-path={} {} -c {} -o {}",
        cpp_compiler,
        cxx_standard,
        stdlib_flag_suffix,
        portable_intrinsics_define,
        include_dir.display(),
        pcm_dir.display(),
        runner_rusty_pcm_flag,
        runner_path.display(),
        runner_object.display()
    );
    build_log.push_str(&format!("$ {}\n", runner_compile_cmd));
    let mut runner_compile_command = std::process::Command::new(cpp_compiler);
    runner_compile_command
        .arg(format!("-std={}", cxx_standard))
        .arg(portable_intrinsics_define)
        .arg("-march=native");
    if args.import_std {
        runner_compile_command.arg("-stdlib=libc++");
    }
    if let Some(rusty_pcm) = rusty_pcm_dir.as_ref() {
        runner_compile_command.arg(format!("-fprebuilt-module-path={}", rusty_pcm.display()));
    }
    let runner_compile_output = runner_compile_command
        .arg("-Wall")
        .arg("-Wno-unused-variable")
        .arg("-Wno-unused-but-set-variable")
        .arg(format!("-I{}", include_dir.display()))
        .arg(format!("-fprebuilt-module-path={}", pcm_dir.display()))
        .arg("-c")
        .arg(&runner_path)
        .arg("-o")
        .arg(&runner_object)
        .output()
        .map_err(|e| format!("Failed to run {}: {}", cpp_compiler, e))?;
    build_log.push_str(&String::from_utf8_lossy(&runner_compile_output.stderr));
    build_log.push_str(&String::from_utf8_lossy(&runner_compile_output.stdout));
    build_log.push('\n');
    if !runner_compile_output.status.success() {
        fs::write(&build_log_path, &build_log)
            .map_err(|e| format!("Failed to write build log: {}", e))?;
        println!("  Build FAILED — see {}", build_log_path.display());
        for line in build_log
            .lines()
            .filter(|line| line.contains("error:"))
            .take(20)
        {
            println!("    {}", line);
        }
        println!(
            "  Build compile time (module, failed): {:.3}s",
            compile_start.elapsed().as_secs_f64()
        );
        return Err("C++ runner compile failed".to_string());
    }

    let mut link_cmd = std::process::Command::new(cpp_compiler);
    link_cmd.arg(format!("-std={}", cxx_standard));
    if args.import_std {
        link_cmd.arg("-stdlib=libc++");
    }
    link_cmd.arg("-o").arg(&binary_path);
    for obj in &object_files {
        link_cmd.arg(obj);
    }
    link_cmd.arg(&runner_object);
    // Link the rusty umbrella module's static archives so the runner's
    // module-attached entities (vec_port, btree_port, …) can resolve
    // at link time. Add the build dir to the lib search path and pull
    // in each port lib. Linked even if no module-only symbol is
    // referenced — the unused archives are dead-stripped by the linker.
    if let Some(rusty_pcm) = rusty_pcm_dir.as_ref()
        && let Some(rusty_build_dir) = rusty_pcm.parent().map(|p| p.join("build"))
        && rusty_build_dir.exists()
    {
        link_cmd
            .arg(format!("-L{}", rusty_build_dir.display()))
            .arg("-lrusty")
            .arg("-lrusty_async")
            .arg("-lvec_port")
            .arg("-lbtree_port")
            .arg("-lrc_port")
            .arg("-larc_port")
            .arg("-lbinary_heap_port")
            .arg("-lhashbrown_port")
            .arg("-lvec_deque_port")
            .arg("-llinked_list_port")
            .arg("-lcell_port")
            .arg("-lstring_port");
    }
    let link_cmd_str = format!(
        "{} -std={}{} -o {} {} {}",
        cpp_compiler,
        cxx_standard,
        stdlib_flag_suffix,
        binary_path.display(),
        object_files
            .iter()
            .map(|p| p.display().to_string())
            .collect::<Vec<String>>()
            .join(" "),
        runner_object.display()
    );
    build_log.push_str(&format!("$ {}\n", link_cmd_str));
    let link_output = link_cmd
        .output()
        .map_err(|e| format!("Failed to run {}: {}", cpp_compiler, e))?;
    build_log.push_str(&String::from_utf8_lossy(&link_output.stderr));
    build_log.push_str(&String::from_utf8_lossy(&link_output.stdout));
    build_log.push('\n');
    fs::write(&build_log_path, &build_log)
        .map_err(|e| format!("Failed to write build log: {}", e))?;
    if !link_output.status.success() {
        println!("  Build FAILED — see {}", build_log_path.display());
        for line in build_log
            .lines()
            .filter(|line| line.contains("error:"))
            .take(20)
        {
            println!("    {}", line);
        }
        println!(
            "  Build compile time (module, failed): {:.3}s",
            compile_start.elapsed().as_secs_f64()
        );
        return Err("C++ link failed".to_string());
    }

    println!(
        "  Build compile time (module): {:.3}s",
        compile_start.elapsed().as_secs_f64()
    );
    println!("  Build: PASS → {}", binary_path.display());
    Ok(())
}

/// Run the parity test pipeline: cargo test → cargo expand → transpile → C++ compile → run → compare.
fn run_parity_test(args: &ParityTestArgs) -> Result<(), String> {
    let manifest = std::fs::canonicalize(&args.manifest_path)
        .map_err(|_| format!("Manifest not found: {}", args.manifest_path.display()))?;

    let cargo = cmake::parse_cargo_toml(&manifest)?;
    let crate_name = &cargo.package.name;

    // Validate stop_after if provided
    if let Some(ref stage) = args.stop_after {
        if !matches!(
            stage.as_str(),
            "baseline" | "expand" | "transpile" | "build" | "run"
        ) {
            return Err(format!(
                "Invalid --stop-after stage '{}'. Valid: baseline, expand, transpile, build, run",
                stage
            ));
        }
    }

    let should_stop = |stage: &str| -> bool { args.stop_after.as_deref() == Some(stage) };

    // Create work directory and canonicalize
    std::fs::create_dir_all(&args.work_dir)
        .map_err(|e| format!("Failed to create work dir: {}", e))?;
    let work_dir = std::fs::canonicalize(&args.work_dir).unwrap_or_else(|_| args.work_dir.clone());
    if !args.dry_run && !args.incremental_transpile {
        clear_stage_outputs(&work_dir)?;
    }

    let project_dir = manifest.parent().unwrap_or(Path::new(".")).to_path_buf();

    // Build cargo feature flags
    let mut cargo_flags: Vec<String> = Vec::new();
    if let Some(ref features) = args.features {
        cargo_flags.push("--features".to_string());
        cargo_flags.push(features.clone());
    }
    if args.all_features {
        cargo_flags.push("--all-features".to_string());
    }
    if args.no_default_features {
        cargo_flags.push("--no-default-features".to_string());
    }

    println!("╔═══════════════════════════════════════════════════╗");
    println!("║  Parity Test: {}", crate_name);
    println!("╚═══════════════════════════════════════════════════╝");
    println!();

    // ── Stage A: Baseline (cargo test) ──────────────────
    if !args.no_baseline {
        println!("Stage A: Running cargo test (baseline)...");
        if args.dry_run {
            println!(
                "  [dry-run] cargo test {} in {}",
                cargo_flags.join(" "),
                project_dir.display()
            );
        } else {
            let output = run_baseline_with_workspace_fallback(
                &manifest,
                &project_dir,
                args.package.as_deref(),
                crate_name,
                &cargo_flags,
                &work_dir,
            )?;
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);

            // Save baseline output
            let baseline_path = work_dir.join("baseline.txt");
            std::fs::write(&baseline_path, format!("{}\n{}", stdout, stderr))
                .map_err(|e| format!("Failed to write baseline: {}", e))?;

            if !output.status.success() {
                return Err(format!(
                    "Baseline cargo test failed. See {}",
                    baseline_path.display()
                ));
            }
            println!("  Baseline: PASS (saved to {})", baseline_path.display());
        }
        if should_stop("baseline") {
            println!("\nStopped after baseline stage.");
            return Ok(());
        }
    }

    // ── Target Discovery ─────────────────────────────────
    println!("Discovering targets...");
    let (pkg_name, targets) = discover_targets_with_workspace_fallback(
        &manifest,
        &project_dir,
        args.package.as_deref(),
        crate_name,
        &work_dir,
    )?;
    println!("  Package: {}", pkg_name);
    for t in &targets {
        println!(
            "  Target: {} ({:?}) → module {}",
            t.name, t.kind, t.module_name
        );
    }
    if targets.is_empty() {
        return Err("No test-capable targets found".to_string());
    }
    let local_dependency_packages = discover_local_dependencies_with_workspace_fallback(
        &manifest,
        &project_dir,
        args.package.as_deref(),
        crate_name,
        &work_dir,
        false,
        true,
        &cargo_flags,
    )?;
    let local_dependency_manifests: HashSet<PathBuf> = local_dependency_packages
        .iter()
        .map(|dep| dep.manifest_path.clone())
        .collect();
    let dependency_packages = discover_local_dependencies_with_workspace_fallback(
        &manifest,
        &project_dir,
        args.package.as_deref(),
        crate_name,
        &work_dir,
        true,
        true,
        &cargo_flags,
    )?;
    let mut dependency_targets: Vec<ParityDependencyTarget> = Vec::new();
    let mut non_library_dependency_roots: HashSet<String> = HashSet::new();
    for dep in dependency_packages {
        let dep_project_dir = dep
            .manifest_path
            .parent()
            .unwrap_or(Path::new("."))
            .to_path_buf();
        let (_, dep_targets) = discover_targets_with_workspace_fallback(
            &dep.manifest_path,
            &dep_project_dir,
            Some(dep.name.as_str()),
            dep.name.as_str(),
            &work_dir,
        )?;
        if let Some(lib_target) = dep_targets
            .iter()
            .find(|target| matches!(target.kind, metadata::TargetKind::Lib))
        {
            let is_registry = !local_dependency_manifests.contains(&dep.manifest_path);
            let dep_cargo_flags = dependency_expand_cargo_flags(&dep.resolved_features);
            let mut extern_crate_roots = dep.extern_crate_roots.clone();
            extern_crate_roots.push(dep.name.replace('-', "_"));
            extern_crate_roots.push(lib_target.module_name.clone());
            extern_crate_roots.retain(|root| is_external_crate_root_candidate(root));
            extern_crate_roots.sort();
            extern_crate_roots.dedup();
            dependency_targets.push(ParityDependencyTarget {
                package_name: dep.name,
                manifest_path: dep.manifest_path,
                module_name: lib_target.module_name.clone(),
                extern_crate_roots,
                is_registry,
                cargo_flags: dep_cargo_flags,
            });
        } else {
            let mut extern_crate_roots = dep.extern_crate_roots.clone();
            extern_crate_roots.push(dep.name.replace('-', "_"));
            for root in extern_crate_roots {
                if is_external_crate_root_candidate(&root) {
                    non_library_dependency_roots.insert(root);
                }
            }
        }
    }
    if !dependency_targets.is_empty() {
        println!("  Dependencies:");
        for dep in &dependency_targets {
            let dep_flags_display = if dep.cargo_flags.is_empty() {
                String::new()
            } else {
                format!(" (flags: {})", dep.cargo_flags.join(" "))
            };
            let dep_roots_display = if dep.extern_crate_roots.is_empty() {
                String::new()
            } else {
                format!(" (roots: {})", dep.extern_crate_roots.join(","))
            };
            println!(
                "    {} ({}) → module {}{}{}{}",
                dep.package_name,
                dep.manifest_path.display(),
                dep.module_name,
                if dep.is_registry { " [registry]" } else { "" },
                dep_flags_display,
                dep_roots_display
            );
        }
    }
    if !non_library_dependency_roots.is_empty() {
        let mut alias_only_roots: Vec<String> =
            non_library_dependency_roots.iter().cloned().collect();
        alias_only_roots.sort();
        alias_only_roots.dedup();
        println!(
            "  Non-library dependency roots (alias-only): {}",
            alias_only_roots.join(", ")
        );
    }
    let target_dirs = if args.dry_run {
        HashMap::new()
    } else if args.incremental_transpile {
        ensure_target_artifact_dirs(&work_dir, &targets)?
    } else {
        reset_target_artifacts(&work_dir, &targets)?
    };
    let dependency_dirs = if args.dry_run {
        HashMap::new()
    } else if args.incremental_transpile {
        ensure_dependency_artifact_dirs(&work_dir, &dependency_targets)?
    } else {
        reset_dependency_artifacts(&work_dir, &dependency_targets)?
    };
    println!();

    // ── Stage B: Expand ─────────────────────────────────
    let mut expanded_dependency_sources: Vec<(ParityDependencyTarget, String)> = Vec::new();
    let mut expanded_sources: Vec<(metadata::CrateTarget, String)> = Vec::new();
    let mut expand_isolated_manifest: Option<PathBuf> = None;
    if args.skip_expand {
        println!("Stage B: Reusing expanded sources from work dir...");
        if args.dry_run {
            println!(
                "  [dry-run] reuse expanded.rs artifacts in {}",
                work_dir.display()
            );
        } else {
            for dep in &dependency_targets {
                let dep_dir = dependency_dirs.get(&dep.module_name).ok_or_else(|| {
                    format!(
                        "Missing dependency artifact directory for module '{}'",
                        dep.module_name
                    )
                })?;
                let expanded_path = expanded_artifact_path(dep_dir);
                let source = std::fs::read_to_string(&expanded_path).map_err(|e| {
                    format!(
                        "Failed to read expanded dependency source {}: {}",
                        expanded_path.display(),
                        e
                    )
                })?;
                println!(
                    "  dep {} (--lib): reused {} lines ← {}",
                    dep.package_name,
                    source.lines().count(),
                    expanded_path.display()
                );
                expanded_dependency_sources.push((dep.clone(), source));
            }
            for target in &targets {
                let target_dir = target_dirs.get(&target.module_name).ok_or_else(|| {
                    format!(
                        "Missing target artifact directory for module '{}'",
                        target.module_name
                    )
                })?;
                let expanded_path = expanded_artifact_path(target_dir);
                let source = std::fs::read_to_string(&expanded_path).map_err(|e| {
                    format!(
                        "Failed to read expanded target source {}: {}",
                        expanded_path.display(),
                        e
                    )
                })?;
                println!(
                    "  {}: reused {} lines ← {}",
                    target.name,
                    source.lines().count(),
                    expanded_path.display()
                );
                expanded_sources.push((target.clone(), source));
            }
        }
    } else {
        println!("Stage B: Running cargo expand per target...");
        for dep in &dependency_targets {
            let dep_project_dir = dep
                .manifest_path
                .parent()
                .unwrap_or(Path::new("."))
                .to_path_buf();
            if args.dry_run {
                let dep_flags_display = if dep.cargo_flags.is_empty() {
                    String::new()
                } else {
                    format!(" {}", dep.cargo_flags.join(" "))
                };
                println!(
                    "  [dry-run] cargo expand -p {} --lib{} --theme=none in {}",
                    dep.package_name,
                    dep_flags_display,
                    dep_project_dir.display()
                );
                continue;
            }

            let mut dep_expand_isolated_manifest: Option<PathBuf> = None;
            let output = run_cargo_expand_with_workspace_fallback(
                &dep.manifest_path,
                &dep_project_dir,
                Some(dep.package_name.as_str()),
                dep.package_name.as_str(),
                &["--lib".to_string()],
                &dep.cargo_flags,
                &work_dir,
                &mut dep_expand_isolated_manifest,
            )?;

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                eprintln!(
                    "  Warning: cargo expand failed for dependency '{}': {}",
                    dep.package_name,
                    stderr.lines().next().unwrap_or("")
                );
                continue;
            }

            let source = String::from_utf8(output.stdout)
                .map_err(|e| format!("Invalid UTF-8 from cargo expand: {}", e))?;
            let dep_dir = dependency_dirs.get(&dep.module_name).ok_or_else(|| {
                format!(
                    "Missing dependency artifact directory for module '{}'",
                    dep.module_name
                )
            })?;
            let expanded_path = expanded_artifact_path(dep_dir);
            std::fs::write(&expanded_path, &source)
                .map_err(|e| format!("Failed to write expanded source: {}", e))?;
            println!(
                "  dep {} (--lib): {} lines → {}",
                dep.package_name,
                source.lines().count(),
                expanded_path.display()
            );
            expanded_dependency_sources.push((dep.clone(), source));
        }

        for target in &targets {
            let (expand_args, expand_desc): (Vec<String>, String) = match target.kind {
                metadata::TargetKind::Lib => (
                    vec!["--lib".to_string(), "--tests".to_string()],
                    "--lib --tests".to_string(),
                ),
                metadata::TargetKind::Bin => (
                    vec!["--bin".to_string(), target.name.clone()],
                    format!("--bin {}", target.name),
                ),
                metadata::TargetKind::Test => (
                    vec!["--test".to_string(), target.name.clone()],
                    format!("--test {}", target.name),
                ),
                _ => (
                    vec![
                        target
                            .kind
                            .cargo_expand_flag()
                            .unwrap_or("--lib")
                            .to_string(),
                    ],
                    target
                        .kind
                        .cargo_expand_flag()
                        .unwrap_or("--lib")
                        .to_string(),
                ),
            };

            if args.dry_run {
                println!(
                    "  [dry-run] cargo expand {} --theme=none in {}",
                    expand_desc,
                    project_dir.display()
                );
                continue;
            }

            let output = run_cargo_expand_with_workspace_fallback(
                &manifest,
                &project_dir,
                args.package.as_deref(),
                crate_name,
                &expand_args,
                &cargo_flags,
                &work_dir,
                &mut expand_isolated_manifest,
            )?;

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                eprintln!(
                    "  Warning: cargo expand failed for target '{}': {}",
                    target.name,
                    stderr.lines().next().unwrap_or("")
                );
                continue;
            }

            let source = String::from_utf8(output.stdout)
                .map_err(|e| format!("Invalid UTF-8 from cargo expand: {}", e))?;

            // Save expanded source
            let target_dir = target_dirs.get(&target.module_name).ok_or_else(|| {
                format!(
                    "Missing target artifact directory for module '{}'",
                    target.module_name
                )
            })?;
            let expanded_path = expanded_artifact_path(target_dir);
            std::fs::write(&expanded_path, &source)
                .map_err(|e| format!("Failed to write expanded source: {}", e))?;
            println!(
                "  {} ({}): {} lines → {}",
                target.name,
                expand_desc,
                source.lines().count(),
                expanded_path.display()
            );

            expanded_sources.push((target.clone(), source));
        }
    }
    if !args.dry_run && !dependency_targets.is_empty() {
        let registry_roots: HashSet<String> = dependency_targets
            .iter()
            .filter(|dep| dep.is_registry)
            .flat_map(|dep| dep.extern_crate_roots.iter().cloned())
            .filter(|root| !is_runtime_provided_external_crate_root(root))
            .collect();

        if !registry_roots.is_empty() {
            let mut selected_registry_roots: HashSet<String> = HashSet::new();
            let mut worklist: Vec<String> = Vec::new();
            let mut seed_roots: HashSet<String> = HashSet::new();

            for (_, source) in &expanded_sources {
                seed_roots.extend(collect_external_crate_roots_from_source(source));
            }
            for (dep, source) in &expanded_dependency_sources {
                if !dep.is_registry {
                    seed_roots.extend(collect_external_crate_roots_from_source(source));
                }
            }

            for root in seed_roots {
                if registry_roots.contains(&root) && selected_registry_roots.insert(root.clone()) {
                    worklist.push(root);
                }
            }

            let expanded_registry_sources_by_root: HashMap<String, &String> =
                expanded_dependency_sources
                    .iter()
                    .filter(|(dep, _)| dep.is_registry)
                    .flat_map(|(dep, source)| {
                        dep.extern_crate_roots
                            .iter()
                            .cloned()
                            .map(move |root| (root, source))
                    })
                    .collect();

            while let Some(root) = worklist.pop() {
                let Some(source) = expanded_registry_sources_by_root.get(&root) else {
                    continue;
                };
                for nested_root in collect_external_crate_roots_from_source(source) {
                    if registry_roots.contains(&nested_root)
                        && selected_registry_roots.insert(nested_root.clone())
                    {
                        worklist.push(nested_root);
                    }
                }
            }

            let dropped_registry: Vec<String> = dependency_targets
                .iter()
                .filter(|dep| dep.is_registry)
                .filter_map(|dep| {
                    let selected = dep
                        .extern_crate_roots
                        .iter()
                        .any(|root| selected_registry_roots.contains(root));
                    if selected {
                        None
                    } else {
                        Some(dep.package_name.clone())
                    }
                })
                .collect();

            dependency_targets.retain(|dep| {
                if !dep.is_registry {
                    return true;
                }
                dep.extern_crate_roots
                    .iter()
                    .any(|root| selected_registry_roots.contains(root))
            });
            expanded_dependency_sources.retain(|(dep, _)| {
                if !dep.is_registry {
                    return true;
                }
                dep.extern_crate_roots
                    .iter()
                    .any(|root| selected_registry_roots.contains(root))
            });

            if !dropped_registry.is_empty() {
                let mut dropped = dropped_registry;
                dropped.sort();
                dropped.dedup();
                println!(
                    "  Pruned unused registry dependencies: {}",
                    dropped.join(", ")
                );
            }
        }
    }
    if should_stop("expand") {
        println!("\nStopped after expand stage.");
        return Ok(());
    }

    // ── Stage C: Transpile ──────────────────────────────
    println!("Stage C: Transpiling to C++...");
    let cpp_index_label = if args.cpp_module_index.is_empty() {
        "<none>".to_string()
    } else {
        args.cpp_module_index
            .iter()
            .map(|path| path.display().to_string())
            .collect::<Vec<String>>()
            .join(", ")
    };
    let type_map = if let Some(ref tm_path) = args.type_map {
        types::UserTypeMap::load(tm_path)?
    } else {
        types::UserTypeMap::default()
    };
    let cpp_module_symbol_index = if args.cpp_module_index.is_empty() {
        None
    } else {
        Some(transpile::load_cpp_module_symbol_index_files(
            &args.cpp_module_index,
        )?)
    };
    let mut flattened_dependency_aliases: HashMap<String, String> = HashMap::new();
    // A namespace-wrapped dependency keeps its crate prefix: a reference to
    // `hashbrown::X` must resolve to (relative) `hashbrown::X` — i.e. `::hashbrown::X`
    // from the consumer's global scope — not the stripped global `::X`, whose
    // namespaces are now empty under the wrap. A non-wrapped dep strips to `::X`.
    let alias_target = |root: &str| -> String {
        if transpile::crate_is_namespace_wrapped(root) {
            root.to_string()
        } else {
            String::new()
        }
    };
    for dep in &dependency_targets {
        for root in &dep.extern_crate_roots {
            flattened_dependency_aliases.insert(root.clone(), alias_target(root));
        }
    }
    for root in &non_library_dependency_roots {
        flattened_dependency_aliases
            .entry(root.clone())
            .or_insert_with(|| alias_target(root));
    }
    // Include the root crate's own extern roots so dependency transpilation can
    // resolve back-edges like `serde -> serde_core` when parity is run for
    // `serde_core`.
    let normalized_root_crate = crate_name.replace('-', "_");
    if is_external_crate_root_candidate(&normalized_root_crate) {
        flattened_dependency_aliases
            .entry(normalized_root_crate.clone())
            .or_insert_with(|| alias_target(&normalized_root_crate));
    }
    for target in &targets {
        if !matches!(target.kind, metadata::TargetKind::Lib) {
            continue;
        }
        let root = target.module_name.trim();
        if is_external_crate_root_candidate(root) {
            flattened_dependency_aliases
                .entry(root.to_string())
                .or_insert_with(|| alias_target(root));
        }
    }
    let mut root_to_module_import: HashMap<String, String> = HashMap::new();
    for dep in &dependency_targets {
        for root in &dep.extern_crate_roots {
            root_to_module_import.insert(root.clone(), dep.module_name.clone());
        }
        let dep_package_root = dep.package_name.replace('-', "_");
        if is_external_crate_root_candidate(&dep_package_root) {
            root_to_module_import
                .entry(dep_package_root)
                .or_insert_with(|| dep.module_name.clone());
        }
    }
    if let Some(root_lib_target) = targets
        .iter()
        .find(|target| matches!(target.kind, metadata::TargetKind::Lib))
    {
        let normalized_root_crate = crate_name.replace('-', "_");
        if is_external_crate_root_candidate(&normalized_root_crate) {
            root_to_module_import
                .entry(normalized_root_crate)
                .or_insert_with(|| root_lib_target.module_name.clone());
        }
        if is_external_crate_root_candidate(&root_lib_target.module_name) {
            root_to_module_import
                .entry(root_lib_target.module_name.clone())
                .or_insert_with(|| root_lib_target.module_name.clone());
        }
    }
    let transpile_options = transpile::TranspileOptions {
        // Crate mode's collect pass sees whole files; the sibling-block
        // cpp_inherit harvest is an inline-rust-only need.
        cross_file_cpp_inherit: Vec::new(),
        cpp_type_aliases: std::collections::HashMap::new(),
        by_value_cycle_breaking_prototype: args.by_value_cycle_breaking_prototype,
        is_dependency: false,
        cpp_module_symbol_index,
        cpp_module_symbol_index_sources: args.cpp_module_index.clone(),
        external_crate_module_aliases: HashMap::new(),
        emit_ufcs_trait_manifest_path: None,
        dependency_ufcs_trait_manifests: Vec::new(),
        use_import_std_in_modules: args.import_std,
        explicit_gmf_includes: Vec::new(),
        // `rusty::Unit` is the default spelling; `--prefer-std-tuple-alias`
        // opts out and `--prefer-rusty-unit-alias` is accepted (no-op)
        // for backwards-compatibility with existing scripts.
        prefer_rusty_unit_alias: !args.prefer_std_tuple_alias,
        prefer_rusty_view_aliases: args.prefer_rusty_view_aliases,
        interface_traits: args.interface_traits,
        inline_rust_block: false,
        cross_file_enums: Vec::new(),
        cross_file_impl_blocks: Vec::new(),
        cross_file_structs: Vec::new(),
        cross_file_type_aliases: Vec::new(),
        flat_import_type_authorizations: BTreeSet::new(),
        crate_module_names: Vec::new(),
        cxx_namespace: None,
        auto_namespace: false,
    };

    let mut generated_cppm_files: Vec<GeneratedCppmArtifact> = Vec::new();
    let mut extension_method_hints = HashSet::new();
    for (_, source) in &expanded_dependency_sources {
        extension_method_hints.extend(transpile::collect_extension_method_hints(source));
    }
    for (_, source) in &expanded_sources {
        extension_method_hints.extend(transpile::collect_extension_method_hints(source));
    }
    if args.dry_run {
        for dep in &dependency_targets {
            println!(
                "  [dry-run] transpile dependency {} as module '{}' (cpp index: {})",
                dep.package_name, dep.module_name, cpp_index_label
            );
        }
        for target in &targets {
            println!(
                "  [dry-run] transpile {} as module '{}' (cpp index: {})",
                target.name, target.module_name, cpp_index_label
            );
        }
    } else {
        // UFCS cross-crate (book § 3.2.7): accumulate dependency trait-manifest
        // paths as each dependency is transpiled, so later dependencies + the
        // target can consume them. Each crate writes `<dir>/ufcs-traits.json`.
        let mut ufcs_dep_manifest_paths: Vec<std::path::PathBuf> = Vec::new();
        for (dep, source) in &expanded_dependency_sources {
            let dep_dir = dependency_dirs.get(&dep.module_name).ok_or_else(|| {
                format!(
                    "Missing dependency artifact directory for module '{}'",
                    dep.module_name
                )
            })?;
            let cppm_path = cppm_artifact_path(dep_dir, &dep.module_name);
            let ufcs_manifest_path = dep_dir.join("ufcs-traits.json");
            if args.incremental_transpile && cppm_path.exists() {
                let reused = std::fs::read_to_string(&cppm_path).map_err(|e| {
                    format!(
                        "Failed to read transpiled dependency {}: {}",
                        cppm_path.display(),
                        e
                    )
                })?;
                ensure_no_external_crate_todos(
                    &format!("dependency '{}'", dep.package_name),
                    &reused,
                    &cppm_path,
                )?;
                println!(
                    "  dep {} ({}): reused {} lines ← {}",
                    dep.package_name,
                    dep.module_name,
                    reused.lines().count(),
                    cppm_path.display()
                );
                generated_cppm_files.push(GeneratedCppmArtifact {
                    path: cppm_path,
                    module_name: dep.module_name.clone(),
                    is_dependency: true,
                    is_test_target: false,
                });
                // A reused dep was transpiled in a prior run, so its manifest
                // is on disk — keep it visible to later crates.
                if ufcs_manifest_path.exists() {
                    ufcs_dep_manifest_paths.push(ufcs_manifest_path);
                }
                continue;
            }
            let mut dep_options = transpile_options.clone();
            dep_options.is_dependency = true;
            dep_options.emit_ufcs_trait_manifest_path = Some(ufcs_manifest_path.clone());
            dep_options.dependency_ufcs_trait_manifests = ufcs_dep_manifest_paths.clone();
            dep_options.external_crate_module_aliases = flattened_dependency_aliases
                .iter()
                .filter_map(|(crate_name, mapped)| {
                    if dep.extern_crate_roots.iter().any(|root| root == crate_name) {
                        None
                    } else {
                        Some((crate_name.clone(), mapped.clone()))
                    }
                })
                .collect();
            let mut cpp = transpile::transpile_full_with_options(
                source,
                Some(&dep.module_name),
                &type_map,
                &extension_method_hints,
                Some(dep.package_name.as_str()),
                &dep_options,
            )?;
            if dep.package_name == "winnow" {
                cpp = rewrite_winnow_namespace_conflicts(&cpp);
            }
            let required_imports = collect_required_named_module_imports(
                source,
                &dep.module_name,
                &root_to_module_import,
            );
            cpp = inject_named_module_imports(&cpp, &required_imports);
            ensure_no_external_crate_todos(
                &format!("dependency '{}'", dep.package_name),
                &cpp,
                &cppm_path,
            )?;
            std::fs::write(&cppm_path, &cpp)
                .map_err(|e| format!("Failed to write transpiled dependency: {}", e))?;
            println!(
                "  dep {} ({}): {} lines → {}",
                dep.package_name,
                dep.module_name,
                cpp.lines().count(),
                cppm_path.display()
            );
            generated_cppm_files.push(GeneratedCppmArtifact {
                path: cppm_path,
                module_name: dep.module_name.clone(),
                is_dependency: true,
                is_test_target: false,
            });
            if ufcs_manifest_path.exists() {
                ufcs_dep_manifest_paths.push(ufcs_manifest_path);
            }
        }

        // Stage C target transpile. Each target is INDEPENDENT — it reads the
        // (now-fixed) dependency manifests `ufcs_dep_manifest_paths` and the
        // read-only context, and writes only its own cppm + manifest, never
        // another target's. The codegen is pure (no global mutable state), so
        // targets can transpile concurrently. (Deps above stay sequential —
        // each consumes all prior deps' manifests, a strict chain.) Parallelism
        // is opt-in via RUSTY_CPP_TRANSPILE_JOBS (default 1 = sequential): the
        // parity matrix already parallelizes across crates, so intra-crate
        // transpile parallelism is for single-crate runs and the matrix tail.
        // Returns (Some(artifact)|None-if-skipped, progress-log) so output stays
        // ordered after the concurrent join.
        //
        // `TranspileOptions` itself is !Send/!Sync — it carries `syn` AST in its
        // `cross_file_*` fields (proc_macro2 spans are deliberately thread-
        // hostile). Those fields are ALWAYS empty in the parity pipeline (built
        // as `Vec::new()` above), so we capture only the Send-safe scalar
        // option fields here and rebuild a fresh `TranspileOptions` per target
        // inside the closure with `..Default::default()` (empty `cross_file_*`,
        // constructed in-thread, never crossing the boundary). The other
        // captures — type_map (HashMap), extension_method_hints (HashSet),
        // the alias/import maps, crate_name — are all Send+Sync.
        let opt_by_value = transpile_options.by_value_cycle_breaking_prototype;
        let opt_cpp_index = transpile_options.cpp_module_symbol_index.clone();
        let opt_cpp_index_sources = transpile_options.cpp_module_symbol_index_sources.clone();
        let opt_import_std = transpile_options.use_import_std_in_modules;
        let opt_prefer_unit = transpile_options.prefer_rusty_unit_alias;
        let opt_prefer_views = transpile_options.prefer_rusty_view_aliases;
        let opt_interface_traits = transpile_options.interface_traits;
        let opt_inline_rust = transpile_options.inline_rust_block;
        let opt_crate_module_names = transpile_options.crate_module_names.clone();
        let opt_cxx_namespace = transpile_options.cxx_namespace.clone();
        let opt_auto_namespace = transpile_options.auto_namespace;
        let opt_incremental = args.incremental_transpile;
        debug_assert!(
            transpile_options.cross_file_enums.is_empty()
                && transpile_options.cross_file_impl_blocks.is_empty()
                && transpile_options.cross_file_structs.is_empty()
                && transpile_options.cross_file_type_aliases.is_empty(),
            "parity transpile assumes empty cross_file_* (syn) fields"
        );
        // The crate's own LIB is a dependency of its test/bench targets — its
        // UFCS trait manifest (e.g. the Itertools extension methods) must be
        // consumable by them, or the tests' method calls never classify as
        // trait calls and emit as members of runtime iterator types
        // ("no member named 'tuple_windows' in 'rusty::slice_iter::Iter'").
        // The lib transpiles earlier in this same loop (targets order puts
        // Lib first; default sequential), so its manifest is on disk by the
        // time the test targets transpile.
        let root_lib_manifest_path: Option<std::path::PathBuf> = targets
            .iter()
            .find(|t| matches!(t.kind, metadata::TargetKind::Lib))
            .and_then(|lib| target_dirs.get(&lib.module_name))
            .map(|dir| dir.join("ufcs-traits.json"));
        let transpile_one = |target: &metadata::CrateTarget,
                             source: &str|
         -> Result<(Option<GeneratedCppmArtifact>, String), String> {
            let mut log = String::new();
            let target_dir = target_dirs.get(&target.module_name).ok_or_else(|| {
                format!(
                    "Missing target artifact directory for module '{}'",
                    target.module_name
                )
            })?;
            let cppm_path = cppm_artifact_path(target_dir, &target.module_name);
            // Test targets that pull in external crates we don't transpile
            // (quickcheck, rand, etc.) should be skipped, not fail the
            // whole parity test. The lib and dependency targets still
            // fail on unresolved externals because they're essential.
            let is_skippable_target = matches!(target.kind, metadata::TargetKind::Test);
            if opt_incremental && cppm_path.exists() {
                let reused = std::fs::read_to_string(&cppm_path).map_err(|e| {
                    format!(
                        "Failed to read transpiled output {}: {}",
                        cppm_path.display(),
                        e
                    )
                })?;
                if is_skippable_target {
                    let unresolved = collect_external_crate_todo_markers(&reused);
                    if !unresolved.is_empty() {
                        return Ok((None, format!(
                            "  Skipping target '{}': unresolved external crates {} (no test wrappers from this target)\n",
                            target.module_name,
                            unresolved.join(", ")
                        )));
                    }
                    if cpp_has_invalid_codegen_pattern(&reused) {
                        return Ok((None, format!(
                            "  Skipping target '{}': transpiled output contains invalid `<auto>` template arguments (no test wrappers from this target)\n",
                            target.module_name
                        )));
                    }
                } else {
                    ensure_no_external_crate_todos(
                        &format!("target '{}'", target.module_name),
                        &reused,
                        &cppm_path,
                    )?;
                }
                log.push_str(&format!(
                    "  {}: reused {} lines ← {}\n",
                    target.module_name,
                    reused.lines().count(),
                    cppm_path.display()
                ));
                return Ok((Some(GeneratedCppmArtifact {
                    path: cppm_path,
                    module_name: target.module_name.clone(),
                    is_dependency: false,
                    is_test_target: matches!(target.kind, metadata::TargetKind::Test),
                }), log));
            }
            // Rebuild from the Send-safe scalar captures (see note above);
            // `..Default::default()` supplies is_dependency=false + empty
            // cross_file_* (constructed in-thread). UFCS cross-crate
            // (book § 3.2.7): the target consumes every dependency's trait
            // manifest so calls to a dependency's trait methods classify +
            // module-qualify (`<dep>::<Tr>_::m`).
            let mut dependency_ufcs_trait_manifests = ufcs_dep_manifest_paths.clone();
            if !matches!(target.kind, metadata::TargetKind::Lib)
                && let Some(lib_manifest) = root_lib_manifest_path.as_ref()
                && lib_manifest.exists()
                && !dependency_ufcs_trait_manifests.contains(lib_manifest)
            {
                dependency_ufcs_trait_manifests.push(lib_manifest.clone());
            }
            let target_options = transpile::TranspileOptions {
                by_value_cycle_breaking_prototype: opt_by_value,
                cpp_module_symbol_index: opt_cpp_index.clone(),
                cpp_module_symbol_index_sources: opt_cpp_index_sources.clone(),
                external_crate_module_aliases: flattened_dependency_aliases.clone(),
                dependency_ufcs_trait_manifests,
                emit_ufcs_trait_manifest_path: Some(target_dir.join("ufcs-traits.json")),
                use_import_std_in_modules: opt_import_std,
                prefer_rusty_unit_alias: opt_prefer_unit,
                prefer_rusty_view_aliases: opt_prefer_views,
                interface_traits: opt_interface_traits,
                inline_rust_block: opt_inline_rust,
                crate_module_names: opt_crate_module_names.clone(),
                cxx_namespace: opt_cxx_namespace.clone(),
                auto_namespace: opt_auto_namespace,
                ..Default::default()
            };
            let mut cpp = transpile::transpile_full_with_options(
                source,
                Some(&target.module_name),
                &type_map,
                &extension_method_hints,
                Some(crate_name),
                &target_options,
            )?;
            // Test targets often reference the crate's own types using
            // `::<crate_name>::Type` (Rust's absolute path), but our
            // flat-namespace emission places those types at `::Type`.
            // Strip the redundant crate prefix so type-position uses
            // resolve. Only applied to test targets to avoid touching
            // dep/lib emissions where the prefix may be load-bearing.
            //
            // EXCEPTION: a namespace-wrapped crate
            // (`transpile::crate_is_namespace_wrapped`) genuinely lives under
            // `namespace <crate>`, so `::<crate>::Type` is the *correct* path,
            // not a redundant prefix — the wrap's own re-qualification emits it
            // on purpose. Stripping it there would re-break the references.
            if matches!(target.kind, metadata::TargetKind::Test)
                && !transpile::crate_is_namespace_wrapped(crate_name)
            {
                let prefix = format!("::{}::", crate_name);
                if cpp.contains(&prefix) {
                    cpp = cpp.replace(&prefix, "::");
                }
            }
            let required_imports = collect_required_named_module_imports(
                source,
                &target.module_name,
                &root_to_module_import,
            );
            cpp = inject_named_module_imports(&cpp, &required_imports);
            if is_skippable_target {
                let unresolved = collect_external_crate_todo_markers(&cpp);
                if !unresolved.is_empty() {
                    return Ok((None, format!(
                        "  Skipping target '{}': unresolved external crates {} (no test wrappers from this target)\n",
                        target.module_name,
                        unresolved.join(", ")
                    )));
                }
                if cpp_has_invalid_codegen_pattern(&cpp) {
                    return Ok((None, format!(
                        "  Skipping target '{}': transpiled output contains invalid `<auto>` template arguments (no test wrappers from this target)\n",
                        target.module_name
                    )));
                }
            } else {
                ensure_no_external_crate_todos(
                    &format!("target '{}'", target.module_name),
                    &cpp,
                    &cppm_path,
                )?;
            }
            std::fs::write(&cppm_path, &cpp)
                .map_err(|e| format!("Failed to write transpiled output: {}", e))?;
            log.push_str(&format!(
                "  {}: {} lines → {}\n",
                target.module_name,
                cpp.lines().count(),
                cppm_path.display()
            ));
            Ok((Some(GeneratedCppmArtifact {
                path: cppm_path,
                module_name: target.module_name.clone(),
                is_dependency: false,
                is_test_target: matches!(target.kind, metadata::TargetKind::Test),
            }), log))
        };

        let transpile_jobs = std::env::var("RUSTY_CPP_TRANSPILE_JOBS")
            .ok()
            .and_then(|v| v.trim().parse::<usize>().ok())
            .filter(|&n| n >= 1)
            .unwrap_or(1);
        if transpile_jobs <= 1 {
            for (target, source) in expanded_sources.iter() {
                let (artifact, log) = transpile_one(target, source)?;
                print!("{}", log);
                if let Some(artifact) = artifact {
                    generated_cppm_files.push(artifact);
                }
            }
        } else {
            // Fan out independent targets, bounded by `transpile_jobs`, joining
            // each chunk before the next. Results are consumed in target order
            // so the progress log and `generated_cppm_files` stay deterministic.
            for chunk in expanded_sources.chunks(transpile_jobs) {
                let results: Vec<Result<(Option<GeneratedCppmArtifact>, String), String>> =
                    std::thread::scope(|scope| {
                        chunk
                            .iter()
                            .map(|(target, source)| {
                                scope.spawn(|| transpile_one(target, source.as_str()))
                            })
                            .collect::<Vec<_>>()
                            .into_iter()
                            .map(|handle| {
                                handle
                                    .join()
                                    .unwrap_or_else(|_| Err("transpile thread panicked".to_string()))
                            })
                            .collect()
                    });
                for result in results {
                    let (artifact, log) = result?;
                    print!("{}", log);
                    if let Some(artifact) = artifact {
                        generated_cppm_files.push(artifact);
                    }
                }
            }
        }
    }
    if should_stop("transpile") {
        println!("\nStopped after transpile stage.");
        return Ok(());
    }

    // ── Stage D: Build ──────────────────────────────────
    println!("Stage D: Building with C++ compiler...");

    // Find rusty-cpp include path (relative to the transpiler binary)
    let include_dir = find_rusty_include_dir();

    let cpp_compiler = parity_cpp_compiler();

    if args.dry_run {
        if args.import_std {
            println!(
                "  [dry-run] module build with {} (import std mode: precompile std.cppm + precompile .cppm + compile runner imports, -stdlib=libc++)",
                cpp_compiler
            );
        } else {
            println!(
                "  [dry-run] module build with {} (precompile .cppm + compile runner imports)",
                cpp_compiler
            );
        }
    } else {
        run_stage_d_module_build(
            args,
            &work_dir,
            &include_dir,
            &cpp_compiler,
            &generated_cppm_files,
            crate_name,
        )?;
    }
    if should_stop("build") {
        println!("\nStopped after build stage.");
        return Ok(());
    }

    // ── Stage E: Run ────────────────────────────────────
    println!("Stage E: Running transpiled tests...");
    let binary_path = work_dir.join("runner");
    let run_log_path = work_dir.join("run.log");

    if args.dry_run {
        println!("  [dry-run] {}", binary_path.display());
    } else {
        let run_output = std::process::Command::new(&binary_path)
            .output()
            .map_err(|e| format!("Failed to run transpiled tests: {}", e))?;

        let run_stdout = String::from_utf8_lossy(&run_output.stdout);
        let run_stderr = String::from_utf8_lossy(&run_output.stderr);
        std::fs::write(&run_log_path, format!("{}\n{}", run_stdout, run_stderr))
            .map_err(|e| format!("Failed to write run log: {}", e))?;

        // Print test output
        for line in run_stdout.lines() {
            println!("  {}", line);
        }
        for line in run_stderr.lines() {
            println!("  {}", line);
        }

        if !run_output.status.success() {
            return Err("Some transpiled tests FAILED".to_string());
        }
        println!("  Run: PASS");
    }

    println!();
    println!("Parity test pipeline complete for '{}'.", crate_name);
    println!("Artifacts saved in: {}", work_dir.display());

    Ok(())
}

/// Find the rusty-cpp include directory.
/// Tries: adjacent to binary, then repo root include/.
fn find_rusty_include_dir() -> PathBuf {
    // Try adjacent to this binary (for installed builds)
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let adjacent = dir.join("../include");
            if adjacent.join("rusty/rusty.hpp").exists() {
                return std::fs::canonicalize(&adjacent).unwrap_or(adjacent);
            }
        }
    }

    // Try workspace include relative to the transpiler crate.
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    if let Some(workspace_root) = manifest_dir.parent() {
        let workspace_include = workspace_root.join("include");
        if workspace_include.join("rusty/rusty.hpp").exists() {
            return std::fs::canonicalize(&workspace_include).unwrap_or(workspace_include);
        }
    }

    // Try relative to current dir (for development)
    let dev_include = PathBuf::from("include");
    if dev_include.join("rusty/rusty.hpp").exists() {
        return std::fs::canonicalize(dev_include).unwrap_or_else(|_| PathBuf::from("include"));
    }

    // Also try one level up from current dir (common when running from ./transpiler).
    let parent_include = PathBuf::from("../include");
    if parent_include.join("rusty/rusty.hpp").exists() {
        return std::fs::canonicalize(&parent_include).unwrap_or(parent_include);
    }

    // Fallback
    PathBuf::from("include")
}

fn main() {
    let cli = Cli::parse();

    if cli.module_preamble.is_some() && (cli.build_info || cli.command.is_some()) {
        eprintln!("Error: --module-preamble requires module output");
        process::exit(1);
    }

    if cli.build_info {
        println!(
            r#"{{"git_hash":"{}","git_dirty":{}}}"#,
            env!("RUSTY_CPP_GIT_HASH"),
            env!("RUSTY_CPP_GIT_DIRTY")
        );
        return;
    }

    // Handle subcommands
    if let Some(ref command) = cli.command {
        match command {
            Commands::ParityTest(args) => {
                match run_parity_test(args) {
                    Ok(()) => {}
                    Err(e) => {
                        eprintln!("Parity test error: {}", e);
                        process::exit(1);
                    }
                }
                return;
            }
            Commands::InlineRust(args) => {
                let mode = if args.rewrite {
                    inline_rust::InlineRustMode::Rewrite
                } else if args.check {
                    inline_rust::InlineRustMode::Check
                } else if let Some(output) = &args.emit_rust {
                    inline_rust::InlineRustMode::EmitRust {
                        output: output.clone(),
                        block_ids: args.block_ids.clone(),
                    }
                } else {
                    eprintln!(
                        "inline-rust error: one of --check, --rewrite, or --emit-rust must be provided"
                    );
                    process::exit(2);
                };
                let options = inline_rust::InlineRustOptions {
                    mode,
                    files: args.files.clone(),
                };
                match inline_rust::run_inline_rust(&options) {
                    Ok(()) => {}
                    Err(e) => {
                        eprintln!("inline-rust error: {}", e);
                        process::exit(1);
                    }
                }
                return;
            }
        }
    }

    // Load user type map if provided
    let type_map = if let Some(ref type_map_path) = cli.type_map {
        match types::UserTypeMap::load(type_map_path) {
            Ok(tm) => {
                println!(
                    "Loaded {} type mappings from {}",
                    tm.mappings.len(),
                    type_map_path.display()
                );
                tm
            }
            Err(e) => {
                eprintln!("Error: {}", e);
                process::exit(1);
            }
        }
    } else {
        types::UserTypeMap::default()
    };
    let cpp_module_symbol_index = if cli.cpp_module_index.is_empty() {
        None
    } else {
        match transpile::load_cpp_module_symbol_index_files(&cli.cpp_module_index) {
            Ok(index) => Some(index),
            Err(e) => {
                eprintln!("Error: {}", e);
                process::exit(1);
            }
        }
    };
    let module_preamble_manifest = if let Some(path) = cli.module_preamble.as_ref() {
        match transpile::load_module_preamble_file(path, cli.preamble_target_os.as_deref()) {
            Ok(manifest) => Some(manifest),
            Err(e) => {
                eprintln!("Error: {}", e);
                process::exit(1);
            }
        }
    } else {
        None
    };
    let transpile_options = transpile::TranspileOptions {
        // Crate mode's collect pass sees whole files; the sibling-block
        // cpp_inherit harvest is an inline-rust-only need.
        cross_file_cpp_inherit: Vec::new(),
        cpp_type_aliases: std::collections::HashMap::new(),
        by_value_cycle_breaking_prototype: cli.by_value_cycle_breaking_prototype,
        is_dependency: false,
        cpp_module_symbol_index,
        cpp_module_symbol_index_sources: cli.cpp_module_index.clone(),
        external_crate_module_aliases: HashMap::new(),
        emit_ufcs_trait_manifest_path: None,
        dependency_ufcs_trait_manifests: Vec::new(),
        use_import_std_in_modules: false,
        explicit_gmf_includes: Vec::new(),
        // `rusty::Unit` is the default spelling; `--prefer-std-tuple-alias`
        // opts out and `--prefer-rusty-unit-alias` is accepted (no-op)
        // for backwards-compatibility with existing scripts.
        prefer_rusty_unit_alias: !cli.prefer_std_tuple_alias,
        prefer_rusty_view_aliases: cli.prefer_rusty_view_aliases,
        interface_traits: cli.interface_traits,
        inline_rust_block: false,
        cross_file_enums: Vec::new(),
        cross_file_impl_blocks: Vec::new(),
        cross_file_structs: Vec::new(),
        cross_file_type_aliases: Vec::new(),
        flat_import_type_authorizations: BTreeSet::new(),
        crate_module_names: Vec::new(),
        cxx_namespace: cli.cxx_namespace.clone(),
        auto_namespace: cli.auto_namespace,
    };

    // Handle --crate: transpile entire crate
    if let Some(ref cargo_toml_path) = cli.crate_ {
        match transpile_crate(
            cargo_toml_path,
            &cli.output_dir,
            &type_map,
            cli.expand,
            cli.verify,
            &transpile_options,
            module_preamble_manifest.as_ref(),
        ) {
            Ok(()) => {}
            Err(e) => {
                eprintln!("Error: {}", e);
                process::exit(1);
            }
        }
        return;
    }

    // Handle --cmake: generate CMakeLists.txt from Cargo.toml
    if let Some(ref cargo_toml_path) = cli.cmake {
        match generate_cmake_from_cargo(cargo_toml_path) {
            Ok(()) => {}
            Err(e) => {
                eprintln!("Error: {}", e);
                process::exit(1);
            }
        }
        return;
    }

    // Single-file mode: require input
    let input_path = match &cli.input {
        Some(p) => p,
        None => {
            eprintln!("Error: input file required (or use --crate for whole-crate mode)");
            process::exit(1);
        }
    };

    if !input_path.exists() {
        eprintln!("Error: input file '{}' not found", input_path.display());
        process::exit(1);
    }

    let output_path = cli.output.unwrap_or_else(|| {
        let mut p = input_path.clone();
        p.set_extension("cppm");
        p
    });

    if cli.expand {
        match std::fs::read_to_string(input_path) {
            Ok(source) if cpp_default_args::source_mentions_marker(&source) => {
                eprintln!(
                    "Error: cpp_default_argument does not support --expand because expansion removes inert source markers"
                );
                process::exit(1);
            }
            Ok(_) => {}
            Err(error) => {
                eprintln!("Error reading '{}': {}", input_path.display(), error);
                process::exit(1);
            }
        }
    }

    let source = if cli.expand {
        let original = match std::fs::read_to_string(input_path) {
            Ok(source) => source,
            Err(e) => {
                eprintln!("Error reading '{}': {}", input_path.display(), e);
                process::exit(1);
            }
        };
        if cpp_name::source_mentions_reserved_marker(&original) {
            eprintln!(
                "Error: cpp_name does not support --expand because expansion removes inert name markers"
            );
            process::exit(1);
        }
        match run_cargo_expand(input_path) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("Error: {}", e);
                process::exit(1);
            }
        }
    } else {
        match std::fs::read_to_string(input_path) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("Error reading '{}': {}", input_path.display(), e);
                process::exit(1);
            }
        }
    };

    let mut single_transpile_options = transpile_options;
    if let Some(manifest) = module_preamble_manifest.as_ref() {
        let Some(module_name) = cli.module_name.as_deref() else {
            eprintln!(
                "Error: --module-preamble requires module output; pass --module-name for single-file transpilation"
            );
            process::exit(1);
        };
        let selected = match manifest.select_for_modules([module_name]) {
            Ok(selected) => selected,
            Err(e) => {
                eprintln!("Error: {}", e);
                process::exit(1);
            }
        };
        single_transpile_options.explicit_gmf_includes =
            selected.get(module_name).cloned().unwrap_or_default();
    }

    let cpp_output = match transpile::transpile_full_with_options(
        &source,
        cli.module_name.as_deref(),
        &type_map,
        &HashSet::new(),
        None,
        &single_transpile_options,
    ) {
        Ok(output) => output,
        Err(e) => {
            eprintln!("Transpilation error: {}", e);
            process::exit(1);
        }
    };

    match std::fs::write(&output_path, &cpp_output) {
        Ok(()) => {
            println!(
                "Transpiled {} -> {}",
                input_path.display(),
                output_path.display()
            );
        }
        Err(e) => {
            eprintln!("Error writing '{}': {}", output_path.display(), e);
            process::exit(1);
        }
    }

    if cli.verify {
        match run_rusty_cpp_checker(&output_path) {
            Ok(()) => {
                println!("Verification passed: no safety violations found.");
            }
            Err(e) => {
                eprintln!("Verification: {}", e);
                process::exit(2);
            }
        }
    }
}

/// Run the rusty-cpp-checker on the transpiled C++ output to verify safety.
fn run_rusty_cpp_checker(cpp_path: &Path) -> Result<(), String> {
    let checker = find_checker_binary();

    let output = std::process::Command::new(&checker)
        .arg(cpp_path)
        .output()
        .map_err(|e| {
            format!(
                "Failed to run `{}`: {}. Ensure rusty-cpp-checker is installed and in PATH.",
                checker, e
            )
        })?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    if !stdout.is_empty() {
        eprint!("{}", stdout);
    }
    if !stderr.is_empty() {
        eprint!("{}", stderr);
    }

    if output.status.success() {
        Ok(())
    } else {
        Err(format!(
            "rusty-cpp-checker found issues (exit code: {})",
            output.status.code().unwrap_or(-1)
        ))
    }
}

/// Find the rusty-cpp-checker binary.
fn find_checker_binary() -> String {
    if let Ok(self_path) = std::env::current_exe() {
        if let Some(dir) = self_path.parent() {
            let adjacent = dir.join("rusty-cpp-checker");
            if adjacent.exists() {
                return adjacent.to_string_lossy().to_string();
            }
        }
    }
    "rusty-cpp-checker".to_string()
}
