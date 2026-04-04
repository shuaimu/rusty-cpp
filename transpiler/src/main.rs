use clap::{Parser, Subcommand};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{self, Output};

mod cmake;
mod codegen;
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

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Run parity test: transpile a Rust crate's tests to C++ and verify same results
    ParityTest(ParityTestArgs),
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

    /// User-provided type mapping file
    #[arg(long)]
    type_map: Option<PathBuf>,
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
                    match transpile_crate(&dep_cargo_toml, &dep_out_dir, type_map, expand, verify) {
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
                match transpile::transpile_with_type_map(
                    &expanded_source,
                    Some(crate_name),
                    type_map,
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

        match transpile::transpile_with_type_map(&source, Some(&module_name), type_map) {
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

fn is_overloaded_template_line(trimmed: &str) -> bool {
    trimmed == "template<class... Ts>" || trimmed == "template <class... Ts>"
}

fn is_overloaded_struct_line(trimmed: &str) -> bool {
    trimmed.contains("struct overloaded : Ts... { using Ts::operator()...; };")
}

fn is_overloaded_deduction_line(trimmed: &str) -> bool {
    trimmed.contains("overloaded(Ts...) -> overloaded<Ts...>;")
}

fn extract_rusty_test_wrapper_name(trimmed: &str) -> Option<String> {
    let line = trimmed.strip_prefix("export ").unwrap_or(trimmed);
    let rest = line.strip_prefix("void rusty_test_")?;
    let end = rest.find('(')?;
    Some(format!("rusty_test_{}", &rest[..end]))
}

fn collect_rusty_test_entries_from_cppm(
    content: &str,
    seen_test_fns: &mut HashSet<String>,
    test_entries: &mut Vec<(String, String)>,
) {
    for line in content.lines() {
        if let Some(fn_name) = extract_rusty_test_wrapper_name(line.trim()) {
            if seen_test_fns.insert(fn_name.clone()) {
                test_entries.push((fn_name.clone(), test_label_from_fn_name(&fn_name)));
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
                ("rusty_test_alpha".to_string(), "alpha".to_string()),
                ("rusty_test_beta".to_string(), "beta".to_string()),
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
        assert_eq!(entries[0].0, "rusty_test_dup");
        assert_eq!(entries[0].1, "dup");
    }
}

fn run_cargo_test(
    current_dir: &Path,
    manifest_path: Option<&Path>,
    package: Option<&str>,
    cargo_flags: &[String],
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
    cmd.output()
        .map_err(|e| format!("Failed to run cargo test: {}", e))
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

fn run_baseline_with_workspace_fallback(
    manifest: &Path,
    project_dir: &Path,
    package: Option<&str>,
    crate_name: &str,
    cargo_flags: &[String],
    work_dir: &Path,
) -> Result<Output, String> {
    let initial = run_cargo_test(project_dir, None, package, cargo_flags)?;
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
        )?;
        if workspace_output.status.success() {
            return Ok(workspace_output);
        }
        let workspace_stderr = String::from_utf8_lossy(&workspace_output.stderr);
        if !is_workspace_package_miss(&workspace_stderr) {
            return Ok(workspace_output);
        }
    }

    let isolated_root = work_dir.join("baseline_source_manifest");
    if isolated_root.exists() {
        fs::remove_dir_all(&isolated_root).map_err(|e| {
            format!(
                "Failed to clean baseline isolation dir {}: {}",
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
    let isolated_manifest = isolated_root.join(&manifest_rel);

    println!(
        "  Baseline retry: cargo test --manifest-path {}",
        isolated_manifest.display()
    );
    run_cargo_test(
        &isolated_root,
        Some(&isolated_manifest),
        package,
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

/// Run the parity test pipeline: cargo test → cargo expand → transpile → g++ → run → compare.
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

    // Create work directory
    std::fs::create_dir_all(&args.work_dir)
        .map_err(|e| format!("Failed to create work dir: {}", e))?;
    if !args.dry_run {
        clear_stage_outputs(&args.work_dir)?;
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
                &args.work_dir,
            )?;
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);

            // Save baseline output
            let baseline_path = args.work_dir.join("baseline.txt");
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
    let (pkg_name, targets) = metadata::discover_targets(&manifest, args.package.as_deref())?;
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
    let target_dirs = if args.dry_run {
        HashMap::new()
    } else {
        reset_target_artifacts(&args.work_dir, &targets)?
    };
    println!();

    // ── Stage B: Expand ─────────────────────────────────
    println!("Stage B: Running cargo expand per target...");
    let mut expanded_sources: Vec<(metadata::CrateTarget, String)> = Vec::new();

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
                vec![target
                    .kind
                    .cargo_expand_flag()
                    .unwrap_or("--lib")
                    .to_string()],
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

        let mut cmd = std::process::Command::new("cargo");
        cmd.arg("expand").current_dir(&project_dir);
        for arg in &expand_args {
            cmd.arg(arg);
        }
        cmd.arg("--theme=none");
        for flag in &cargo_flags {
            cmd.arg(flag);
        }

        let output = cmd.output().map_err(|e| {
            format!(
                "Failed to run cargo expand for target '{}': {}",
                target.name, e
            )
        })?;

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
    if should_stop("expand") {
        println!("\nStopped after expand stage.");
        return Ok(());
    }

    // ── Stage C: Transpile ──────────────────────────────
    println!("Stage C: Transpiling to C++...");
    let type_map = if let Some(ref tm_path) = args.type_map {
        types::UserTypeMap::load(tm_path)?
    } else {
        types::UserTypeMap::default()
    };

    let mut generated_cppm_files: Vec<PathBuf> = Vec::new();
    if args.dry_run {
        for (target, _) in &expanded_sources {
            println!(
                "  [dry-run] transpile {} as module '{}'",
                target.name, target.module_name
            );
        }
    } else {
        for (target, source) in &expanded_sources {
            let cpp =
                transpile::transpile_with_type_map(source, Some(&target.module_name), &type_map)?;
            let target_dir = target_dirs.get(&target.module_name).ok_or_else(|| {
                format!(
                    "Missing target artifact directory for module '{}'",
                    target.module_name
                )
            })?;
            let cppm_path = cppm_artifact_path(target_dir, &target.module_name);
            std::fs::write(&cppm_path, &cpp)
                .map_err(|e| format!("Failed to write transpiled output: {}", e))?;
            println!(
                "  {}: {} lines → {}",
                target.module_name,
                cpp.lines().count(),
                cppm_path.display()
            );
            generated_cppm_files.push(cppm_path);
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

    if args.dry_run {
        println!(
            "  [dry-run] g++ -std=c++20 -I {} -o runner ...",
            include_dir.display()
        );
    } else {
        // Generate a runner .cpp that includes all transpiled code + test main
        let runner_path = args.work_dir.join("runner.cpp");
        let binary_path = args.work_dir.join("runner");

        // Compile only artifacts generated in this run to avoid stale file bleed
        // when reusing --work-dir with --keep-work-dir.
        let cppm_files = generated_cppm_files.clone();

        if cppm_files.is_empty() {
            return Err(
                "No .cppm files generated in this run — Stage C may have failed".to_string(),
            );
        }

        // Generate runner: strip module syntax, add includes, add main
        let mut runner_src = String::new();
        runner_src.push_str("// Auto-generated parity test runner\n");
        runner_src.push_str("#include <cstdint>\n#include <cstddef>\n");
        runner_src.push_str("#include <variant>\n#include <string>\n#include <optional>\n");
        runner_src.push_str("#include <iostream>\n#include <cassert>\n#include <vector>\n");
        runner_src.push_str("#include <functional>\n#include <span>\n");
        runner_src.push_str("#include <rusty/rusty.hpp>\n");
        runner_src.push_str("#include <rusty/io.hpp>\n#include <rusty/array.hpp>\n\n");
        runner_src.push_str("// Overloaded visitor helper\n");
        runner_src.push_str(
            "template<class... Ts> struct overloaded : Ts... { using Ts::operator()...; };\n",
        );
        runner_src.push_str("template<class... Ts>\n");
        runner_src.push_str("overloaded(Ts...) -> overloaded<Ts...>;\n\n");

        // Collect test names and transpiled code
        let mut test_entries: Vec<(String, String)> = Vec::new();
        let mut seen_test_fns: HashSet<String> = HashSet::new();
        let module_namespace_markers: Vec<String> = targets
            .iter()
            .map(|target| format!("{}::", target.module_name))
            .collect();

        for (cppm_index, cppm_path) in cppm_files.iter().enumerate() {
            let content = std::fs::read_to_string(cppm_path)
                .map_err(|e| format!("Failed to read {}: {}", cppm_path.display(), e))?;

            let mut pending_overloaded_template = false;
            let mut skip_shared_prelude = cppm_index > 0;
            collect_rusty_test_entries_from_cppm(&content, &mut seen_test_fns, &mut test_entries);

            // Strip module syntax and add code
            runner_src.push_str(&format!(
                "// ── from {} ──\n",
                cppm_path.file_name().unwrap().to_string_lossy()
            ));
            for line in content.lines() {
                let trimmed = line.trim();
                if skip_shared_prelude {
                    // For additional module units, skip the duplicated runtime prelude and
                    // resume at crate/test payloads (extern crate/use/export item region).
                    if trimmed.starts_with("// extern crate")
                        || trimmed.starts_with("// Rust-only:")
                        || (trimmed.starts_with("export ")
                            && !trimmed.starts_with("export module "))
                    {
                        skip_shared_prelude = false;
                    } else {
                        continue;
                    }
                }
                if pending_overloaded_template {
                    if is_overloaded_struct_line(trimmed) || is_overloaded_deduction_line(trimmed) {
                        pending_overloaded_template = false;
                        continue;
                    }
                    runner_src.push_str("template<class... Ts>\n");
                    pending_overloaded_template = false;
                }
                // Skip module/import/include lines (we provide our own)
                if trimmed.starts_with("export module ")
                    || trimmed.starts_with("import ")
                    || trimmed.starts_with("export import ")
                    || trimmed.starts_with("#include ")
                    || trimmed.starts_with("// Auto-generated")
                    || trimmed.starts_with("// Do not edit")
                    || trimmed == "module;"
                {
                    continue;
                }
                // Skip Rust-only using declarations
                if trimmed.starts_with("// Rust-only:") || trimmed.starts_with("// extern crate") {
                    continue;
                }
                // Skip using declarations for undefined namespaces
                if trimmed.starts_with("using ")
                    && (trimmed.contains("::Left")
                        || trimmed.contains("::Right")
                        || trimmed.contains("iterator::")
                        || trimmed.contains("into_either::")
                        || trimmed == "using namespace ;"
                        || module_namespace_markers
                            .iter()
                            .any(|module_prefix| trimmed.contains(module_prefix)))
                {
                    runner_src.push_str(&format!("// skipped: {}\n", trimmed));
                    continue;
                }
                // Skip redefinitions of overloaded helper from transpiled modules.
                if is_overloaded_template_line(trimmed) {
                    pending_overloaded_template = true;
                    continue;
                }
                if is_overloaded_struct_line(trimmed) || is_overloaded_deduction_line(trimmed) {
                    continue;
                }
                // Strip 'export ' prefix from declarations
                let line = if let Some(rest) = line.strip_prefix("export ") {
                    rest
                } else {
                    line
                };
                runner_src.push_str(line);
                runner_src.push('\n');
            }
            if pending_overloaded_template {
                runner_src.push_str("template<class... Ts>\n");
            }
            runner_src.push('\n');
        }

        if test_entries.is_empty() {
            return Err("No transpiled test wrappers discovered (expected exported rusty_test_* functions).".to_string());
        }
        test_entries.sort_by(|a, b| a.0.cmp(&b.0));

        // Generate main() that runs all tests
        runner_src.push_str("\n// ── Test runner ──\n");
        runner_src.push_str("int main() {\n");
        runner_src.push_str("    int pass = 0, fail = 0;\n");
        for (fn_name, label) in &test_entries {
            runner_src.push_str(&format!(
                "    try {{ {}(); std::cout << \"  {} PASSED\" << std::endl; pass++; }}\n",
                fn_name, label
            ));
            runner_src.push_str(&format!(
                "    catch (const std::exception& e) {{ std::cerr << \"  {} FAILED: \" << e.what() << std::endl; fail++; }}\n",
                label
            ));
            runner_src.push_str(&format!(
                "    catch (...) {{ std::cerr << \"  {} FAILED (unknown exception)\" << std::endl; fail++; }}\n",
                label
            ));
        }
        runner_src.push_str("    std::cout << std::endl;\n");
        runner_src.push_str("    std::cout << \"Results: \" << pass << \" passed, \" << fail << \" failed\" << std::endl;\n");
        runner_src.push_str("    return fail > 0 ? 1 : 0;\n");
        runner_src.push_str("}\n");

        std::fs::write(&runner_path, &runner_src)
            .map_err(|e| format!("Failed to write runner: {}", e))?;

        // Save runner log
        let build_log_path = args.work_dir.join("build.log");

        println!(
            "  Generated runner: {} ({} tests discovered)",
            runner_path.display(),
            test_entries.len()
        );

        // Compile with g++
        let compile_output = std::process::Command::new("g++")
            .arg("-std=c++20")
            .arg("-Wall")
            .arg("-Wno-unused-variable")
            .arg("-Wno-unused-but-set-variable")
            .arg(format!("-I{}", include_dir.display()))
            .arg("-o")
            .arg(&binary_path)
            .arg(&runner_path)
            .output()
            .map_err(|e| format!("Failed to run g++: {}", e))?;

        let compile_stderr = String::from_utf8_lossy(&compile_output.stderr);
        std::fs::write(&build_log_path, compile_stderr.as_ref())
            .map_err(|e| format!("Failed to write build log: {}", e))?;

        if !compile_output.status.success() {
            println!("  Build FAILED — see {}", build_log_path.display());
            // Print first 20 errors
            for line in compile_stderr.lines().take(20) {
                println!("    {}", line);
            }
            return Err("C++ compilation failed".to_string());
        }
        println!("  Build: PASS → {}", binary_path.display());
    }
    if should_stop("build") {
        println!("\nStopped after build stage.");
        return Ok(());
    }

    // ── Stage E: Run ────────────────────────────────────
    println!("Stage E: Running transpiled tests...");
    let binary_path = args.work_dir.join("runner");
    let run_log_path = args.work_dir.join("run.log");

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
    println!("Artifacts saved in: {}", args.work_dir.display());

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

    // Handle --crate: transpile entire crate
    if let Some(ref cargo_toml_path) = cli.crate_ {
        match transpile_crate(
            cargo_toml_path,
            &cli.output_dir,
            &type_map,
            cli.expand,
            cli.verify,
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

    let cpp_output =
        match transpile::transpile_with_type_map(&source, cli.module_name.as_deref(), &type_map) {
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
