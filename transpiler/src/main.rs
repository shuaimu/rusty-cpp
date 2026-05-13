use clap::{Parser, Subcommand};
use serde::Deserialize;
use std::collections::{BTreeSet, HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{self, Output};

mod cmake;
mod codegen;
mod inline_rust;
mod metadata;
mod transpile;
mod types;

#[derive(Parser)]
#[command(name = "rusty-cpp-transpiler")]
#[command(about = "Transpile Rust source code to C++ using rusty-cpp types")]
struct Cli {
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

    /// Enable diagnostic-only prototype planning for by-value SCC cycle breaking
    #[arg(long)]
    by_value_cycle_breaking_prototype: bool,

    /// Prefer `rusty::Unit` alias for Rust `()` in generated type positions.
    #[arg(long)]
    prefer_rusty_unit_alias: bool,

    /// Prefer `rusty::StrView` / `rusty::Span<...>` alias spellings in generated output.
    #[arg(long)]
    prefer_rusty_view_aliases: bool,

    /// Lower Rust traits to plain C++ Interface + Adapter classes
    /// (replaces `pro::proxy<...>` facade emission). See docs/rusty-cpp-transpiler.md § 3.2.9.
    #[arg(long)]
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

    /// Deprecated no-op: parity build is always module-based.
    /// Kept only for CLI compatibility with older scripts.
    #[arg(long, hide = true)]
    _module_build: bool,

    /// Prefer `rusty::Unit` alias for Rust `()` in generated type positions.
    #[arg(long)]
    prefer_rusty_unit_alias: bool,

    /// Prefer `rusty::StrView` / `rusty::Span<...>` alias spellings in generated output.
    #[arg(long)]
    prefer_rusty_view_aliases: bool,

    /// Lower Rust traits to plain C++ Interface + Adapter classes
    /// (replaces `pro::proxy<...>` facade emission). See docs/rusty-cpp-transpiler.md § 3.2.9.
    #[arg(long)]
    interface_traits: bool,
}

#[derive(Parser)]
struct InlineRustArgs {
    /// Validate marker structure and rust_sha256 hashes
    #[arg(long, conflicts_with = "rewrite")]
    check: bool,

    /// Rewrite GEN regions with deterministic markers and generated C++ fallback
    #[arg(long, conflicts_with = "check")]
    rewrite: bool,

    /// C++ files containing inline Rust blocks
    #[arg(long = "files", required = true, num_args = 1..)]
    files: Vec<PathBuf>,
}

/// Transpile an entire Rust crate in one command.
/// Walks all .rs files, transpiles each with the correct module name,
/// and generates CMakeLists.txt.
fn transpile_crate(
    cargo_toml_path: &Path,
    output_dir: &Path,
    type_map: &types::UserTypeMap,
    expand: bool,
    verify: bool,
    transpile_options: &transpile::TranspileOptions,
) -> Result<(), String> {
    // Step 1: Parse Cargo.toml and discover source files
    let cargo = cmake::parse_cargo_toml(cargo_toml_path)?;
    let project_dir = cargo_toml_path.parent().unwrap_or(Path::new("."));
    let crate_name = &cargo.package.name;
    let sources = cmake::collect_source_files(project_dir);

    if sources.is_empty() {
        return Err("No .rs source files found in src/".to_string());
    }

    // Create output directory
    std::fs::create_dir_all(output_dir)
        .map_err(|e| format!("Failed to create output dir: {}", e))?;

    // Detect and handle dependencies
    let deps = cmake::extract_dependencies(&cargo);
    let mut local_dep_dirs: Vec<String> = Vec::new();

    if !deps.is_empty() {
        println!("\nDependencies:");
        for dep in &deps {
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
                    match transpile_crate(
                        &dep_cargo_toml,
                        &dep_out_dir,
                        type_map,
                        expand,
                        verify,
                        transpile_options,
                    ) {
                        Ok(()) => {
                            local_dep_dirs.push(dep.name.clone());
                        }
                        Err(e) => {
                            eprintln!(
                                "  Warning: failed to transpile dependency '{}': {}",
                                dep.name, e
                            );
                        }
                    }
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
                match transpile::transpile_with_type_map_and_extension_hints_and_options(
                    &expanded_source,
                    Some(crate_name),
                    type_map,
                    &extension_method_hints,
                    transpile_options,
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
    let mut extension_method_hints = HashSet::new();
    for rs_path in &sources {
        let full_rs_path = project_dir.join(rs_path);
        if let Ok(source) = std::fs::read_to_string(&full_rs_path) {
            extension_method_hints.extend(transpile::collect_extension_method_hints(&source));
        }
    }

    for rs_path in &sources {
        let (cppm_path, module_name) = cmake::map_rs_to_cppm(rs_path, crate_name);
        let full_rs_path = project_dir.join(rs_path);
        let full_cppm_path = output_dir.join(&cppm_path);

        let source = match std::fs::read_to_string(&full_rs_path) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("  Error reading {}: {}", full_rs_path.display(), e);
                error_count += 1;
                continue;
            }
        };

        match transpile::transpile_with_type_map_and_extension_hints_and_options(
            &source,
            Some(&module_name),
            type_map,
            &extension_method_hints,
            transpile_options,
        ) {
            Ok(cpp_output) => {
                if let Err(e) = std::fs::write(&full_cppm_path, &cpp_output) {
                    eprintln!("  Error writing {}: {}", full_cppm_path.display(), e);
                    error_count += 1;
                    continue;
                }
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

    println!("\nGenerated {}", cmake_path.display());
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

    let output = std::process::Command::new("cargo")
        .arg("expand")
        .arg("--theme=none")
        .current_dir(&dir)
        .output()
        .map_err(|e| {
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
            "            if (test_name == \"{}\") {{ {}(); return 0; }}\n",
            entry.fn_name, entry.fn_name
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
                    "    rusty::mem::clear_all_forgotten_addresses();\n    try {{ {}(); std::cout << \"  {} PASSED\" << std::endl; pass++; }}\n",
                    entry.fn_name, entry.label
                ));
            } else {
                runner_src.push_str(&format!(
                    "    try {{ {}(); std::cout << \"  {} PASSED\" << std::endl; pass++; }}\n",
                    entry.fn_name, entry.label
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

fn run_stage_d_module_build(
    args: &ParityTestArgs,
    work_dir: &Path,
    include_dir: &Path,
    cpp_compiler: &str,
    generated_cppm_files: &[GeneratedCppmArtifact],
) -> Result<(), String> {
    let runner_path = work_dir.join("runner.cpp");
    let binary_path = work_dir.join("runner");
    let build_log_path = work_dir.join("build.log");

    if generated_cppm_files.is_empty() {
        return Err("No .cppm files generated in this run — Stage C may have failed".to_string());
    }

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

    for artifact in generated_cppm_files {
        let source = fs::read_to_string(&artifact.path)
            .map_err(|e| format!("Failed to read {}: {}", artifact.path.display(), e))?;
        collect_rusty_test_entries_from_cppm(&source, &mut seen_test_fns, &mut test_entries);
        units.push(ModuleBuildUnit {
            module_name: artifact.module_name.clone(),
            source_path: artifact.path.clone(),
            imports: collect_named_module_imports(&source),
            pcm_path: pcm_dir.join(module_artifact_name(&artifact.module_name, "pcm")),
            object_path: obj_dir.join(module_artifact_name(&artifact.module_name, "o")),
        });
    }

    let compile_start = std::time::Instant::now();
    let mut build_log = String::new();
    let mut object_files: Vec<PathBuf> = Vec::new();
    let order = module_build_order(&units);
    let portable_intrinsics_define = "-DRUSTY_PORTABLE_INTRINSICS=1";
    let cxx_standard = if args.import_std { "c++23" } else { "c++20" };
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

    for idx in order {
        let unit = &units[idx];
        let precompile_cmd = format!(
            "{} -std={}{} {} -x c++-module --precompile -I{} -fprebuilt-module-path={} -o {} {}",
            cpp_compiler,
            cxx_standard,
            stdlib_flag_suffix,
            portable_intrinsics_define,
            include_dir.display(),
            pcm_dir.display(),
            unit.pcm_path.display(),
            unit.source_path.display()
        );
        build_log.push_str(&format!("$ {}\n", precompile_cmd));
        let mut precompile_command = std::process::Command::new(cpp_compiler);
        precompile_command
            .arg(format!("-std={}", cxx_standard))
            .arg(portable_intrinsics_define);
        if args.import_std {
            precompile_command.arg("-stdlib=libc++");
        }
        let precompile_output = precompile_command
            .arg("-x")
            .arg("c++-module")
            .arg("--precompile")
            .arg(format!("-I{}", include_dir.display()))
            .arg(format!("-fprebuilt-module-path={}", pcm_dir.display()))
            .arg("-o")
            .arg(&unit.pcm_path)
            .arg(&unit.source_path)
            .output()
            .map_err(|e| format!("Failed to run {}: {}", cpp_compiler, e))?;
        build_log.push_str(&String::from_utf8_lossy(&precompile_output.stderr));
        build_log.push_str(&String::from_utf8_lossy(&precompile_output.stdout));
        build_log.push('\n');
        if !precompile_output.status.success() {
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

        let object_cmd = format!(
            "{} -std={}{} {} -Wall -Wno-unused-variable -Wno-unused-but-set-variable -I{} -fprebuilt-module-path={} -c {} -o {}",
            cpp_compiler,
            cxx_standard,
            stdlib_flag_suffix,
            portable_intrinsics_define,
            include_dir.display(),
            pcm_dir.display(),
            unit.source_path.display(),
            unit.object_path.display()
        );
        build_log.push_str(&format!("$ {}\n", object_cmd));
        let mut object_command = std::process::Command::new(cpp_compiler);
        object_command
            .arg(format!("-std={}", cxx_standard))
            .arg(portable_intrinsics_define);
        if args.import_std {
            object_command.arg("-stdlib=libc++");
        }
        let object_output = object_command
            .arg("-Wall")
            .arg("-Wno-unused-variable")
            .arg("-Wno-unused-but-set-variable")
            .arg(format!("-I{}", include_dir.display()))
            .arg(format!("-fprebuilt-module-path={}", pcm_dir.display()))
            .arg("-c")
            .arg(&unit.source_path)
            .arg("-o")
            .arg(&unit.object_path)
            .output()
            .map_err(|e| format!("Failed to run {}: {}", cpp_compiler, e))?;
        build_log.push_str(&String::from_utf8_lossy(&object_output.stderr));
        build_log.push_str(&String::from_utf8_lossy(&object_output.stdout));
        build_log.push('\n');
        if !object_output.status.success() {
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

    let mut runner_src = String::new();
    runner_src.push_str("// Auto-generated parity test runner (module mode)\n");
    if args.import_std {
        runner_src.push_str("import std;\n");
    }
    let mut imported_targets: BTreeSet<String> = BTreeSet::new();
    for artifact in generated_cppm_files {
        if !artifact.is_dependency {
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
    append_parity_runner_main(
        &mut runner_src,
        &mut test_entries,
        args.no_baseline,
        args.allow_empty_tests,
        work_dir,
        !args.import_std,
    )?;

    fs::write(&runner_path, &runner_src).map_err(|e| format!("Failed to write runner: {}", e))?;
    println!(
        "  Generated runner: {} ({} tests discovered)",
        runner_path.display(),
        test_entries.len()
    );

    let runner_object = obj_dir.join("runner.o");
    let runner_compile_cmd = format!(
        "{} -std={}{} {} -Wall -Wno-unused-variable -Wno-unused-but-set-variable -I{} -fprebuilt-module-path={} -c {} -o {}",
        cpp_compiler,
        cxx_standard,
        stdlib_flag_suffix,
        portable_intrinsics_define,
        include_dir.display(),
        pcm_dir.display(),
        runner_path.display(),
        runner_object.display()
    );
    build_log.push_str(&format!("$ {}\n", runner_compile_cmd));
    let mut runner_compile_command = std::process::Command::new(cpp_compiler);
    runner_compile_command
        .arg(format!("-std={}", cxx_standard))
        .arg(portable_intrinsics_define);
    if args.import_std {
        runner_compile_command.arg("-stdlib=libc++");
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

        // Cached combined `cargo expand --tests` output. Lazily filled
        // the first time a per-target `--test X` expansion fails (e.g.
        // semver's integration tests panic in cargo's feature resolver),
        // then reused for any other failed test targets in the same
        // package — the combined form expands all integration tests as
        // a single TU, so any test target's wrappers are present.
        let mut combined_tests_expansion: Option<String> = None;

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

            let mut source = if output.status.success() {
                String::from_utf8(output.stdout)
                    .map_err(|e| format!("Invalid UTF-8 from cargo expand: {}", e))?
            } else {
                // Per-target `--test X` expansion sometimes panics inside
                // cargo's feature resolver (semver's integration tests
                // trip src/tools/cargo/.../features.rs:325). Fall back to
                // the combined `--tests` form (no target name) which
                // expands all integration tests as one TU. Cache the
                // combined output so subsequent failed test targets reuse
                // it instead of re-running cargo expand.
                let is_test_target =
                    matches!(target.kind, metadata::TargetKind::Test);
                if !is_test_target {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    eprintln!(
                        "  Warning: cargo expand failed for target '{}': {}",
                        target.name,
                        stderr.lines().next().unwrap_or("")
                    );
                    continue;
                }
                if combined_tests_expansion.is_none() {
                    let combined = run_cargo_expand_with_workspace_fallback(
                        &manifest,
                        &project_dir,
                        args.package.as_deref(),
                        crate_name,
                        &["--tests".to_string()],
                        &cargo_flags,
                        &work_dir,
                        &mut expand_isolated_manifest,
                    )?;
                    if combined.status.success() {
                        match String::from_utf8(combined.stdout) {
                            Ok(src) => {
                                println!(
                                    "  Combined --tests expansion fallback: {} lines (cached for all failed --test X)",
                                    src.lines().count()
                                );
                                combined_tests_expansion = Some(src);
                            }
                            Err(e) => {
                                eprintln!(
                                    "  Warning: combined --tests expansion produced invalid UTF-8: {}",
                                    e
                                );
                            }
                        }
                    } else {
                        let combined_err = String::from_utf8_lossy(&combined.stderr);
                        eprintln!(
                            "  Warning: combined --tests fallback also failed: {}",
                            combined_err.lines().next().unwrap_or("")
                        );
                    }
                }
                let Some(combined_src) = combined_tests_expansion.as_ref() else {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    eprintln!(
                        "  Warning: cargo expand failed for target '{}': {}",
                        target.name,
                        stderr.lines().next().unwrap_or("")
                    );
                    continue;
                };
                eprintln!(
                    "  Using combined --tests fallback for target '{}'",
                    target.name
                );
                combined_src.clone()
            };

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
    for dep in &dependency_targets {
        for root in &dep.extern_crate_roots {
            flattened_dependency_aliases.insert(root.clone(), String::new());
        }
    }
    for root in &non_library_dependency_roots {
        flattened_dependency_aliases
            .entry(root.clone())
            .or_insert_with(String::new);
    }
    // Include the root crate's own extern roots so dependency transpilation can
    // resolve back-edges like `serde -> serde_core` when parity is run for
    // `serde_core`.
    let normalized_root_crate = crate_name.replace('-', "_");
    if is_external_crate_root_candidate(&normalized_root_crate) {
        flattened_dependency_aliases
            .entry(normalized_root_crate)
            .or_insert_with(String::new);
    }
    for target in &targets {
        if !matches!(target.kind, metadata::TargetKind::Lib) {
            continue;
        }
        let root = target.module_name.trim();
        if is_external_crate_root_candidate(root) {
            flattened_dependency_aliases
                .entry(root.to_string())
                .or_insert_with(String::new);
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
        by_value_cycle_breaking_prototype: args.by_value_cycle_breaking_prototype,
        cpp_module_symbol_index,
        cpp_module_symbol_index_sources: args.cpp_module_index.clone(),
        external_crate_module_aliases: HashMap::new(),
        use_import_std_in_modules: args.import_std,
        prefer_rusty_unit_alias: args.prefer_rusty_unit_alias,
        prefer_rusty_view_aliases: args.prefer_rusty_view_aliases,
        interface_traits: args.interface_traits,
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
        for (dep, source) in &expanded_dependency_sources {
            let dep_dir = dependency_dirs.get(&dep.module_name).ok_or_else(|| {
                format!(
                    "Missing dependency artifact directory for module '{}'",
                    dep.module_name
                )
            })?;
            let cppm_path = cppm_artifact_path(dep_dir, &dep.module_name);
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
                });
                continue;
            }
            let mut dep_options = transpile_options.clone();
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
            });
        }

        for (target, source) in expanded_sources.iter() {
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
            let is_skippable_target =
                matches!(target.kind, metadata::TargetKind::Test);
            if args.incremental_transpile && cppm_path.exists() {
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
                        eprintln!(
                            "  Skipping target '{}': unresolved external crates {} (no test wrappers from this target)",
                            target.module_name,
                            unresolved.join(", ")
                        );
                        continue;
                    }
                } else {
                    ensure_no_external_crate_todos(
                        &format!("target '{}'", target.module_name),
                        &reused,
                        &cppm_path,
                    )?;
                }
                println!(
                    "  {}: reused {} lines ← {}",
                    target.module_name,
                    reused.lines().count(),
                    cppm_path.display()
                );
                generated_cppm_files.push(GeneratedCppmArtifact {
                    path: cppm_path,
                    module_name: target.module_name.clone(),
                    is_dependency: false,
                });
                continue;
            }
            let mut target_options = transpile_options.clone();
            target_options.external_crate_module_aliases = flattened_dependency_aliases.clone();
            let mut cpp = transpile::transpile_full_with_options(
                source,
                Some(&target.module_name),
                &type_map,
                &extension_method_hints,
                Some(crate_name),
                &target_options,
            )?;
            let required_imports = collect_required_named_module_imports(
                source,
                &target.module_name,
                &root_to_module_import,
            );
            cpp = inject_named_module_imports(&cpp, &required_imports);
            if is_skippable_target {
                let unresolved = collect_external_crate_todo_markers(&cpp);
                if !unresolved.is_empty() {
                    eprintln!(
                        "  Skipping target '{}': unresolved external crates {} (no test wrappers from this target)",
                        target.module_name,
                        unresolved.join(", ")
                    );
                    continue;
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
            println!(
                "  {}: {} lines → {}",
                target.module_name,
                cpp.lines().count(),
                cppm_path.display()
            );
            generated_cppm_files.push(GeneratedCppmArtifact {
                path: cppm_path,
                module_name: target.module_name.clone(),
                is_dependency: false,
            });
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
                } else {
                    eprintln!("inline-rust error: either --check or --rewrite must be provided");
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
    let transpile_options = transpile::TranspileOptions {
        by_value_cycle_breaking_prototype: cli.by_value_cycle_breaking_prototype,
        cpp_module_symbol_index,
        cpp_module_symbol_index_sources: cli.cpp_module_index.clone(),
        external_crate_module_aliases: HashMap::new(),
        use_import_std_in_modules: false,
        prefer_rusty_unit_alias: cli.prefer_rusty_unit_alias,
        prefer_rusty_view_aliases: cli.prefer_rusty_view_aliases,
        interface_traits: cli.interface_traits,
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

    let source = if cli.expand {
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

    let cpp_output = match transpile::transpile_full_with_options(
        &source,
        cli.module_name.as_deref(),
        &type_map,
        &HashSet::new(),
        None,
        &transpile_options,
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
