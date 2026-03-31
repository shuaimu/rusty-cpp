use clap::Parser;
use std::path::{Path, PathBuf};
use std::process;

mod cmake;
mod codegen;
mod transpile;
mod types;

#[derive(Parser)]
#[command(name = "rusty-cpp-transpiler")]
#[command(about = "Transpile Rust source code to C++ using rusty-cpp types")]
struct Cli {
    /// Input Rust source file (.rs) — not needed with --crate
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
}

/// Transpile an entire Rust crate in one command.
/// Walks all .rs files, transpiles each with the correct module name,
/// and generates CMakeLists.txt.
fn transpile_crate(
    cargo_toml_path: &Path,
    output_dir: &Path,
    verify: bool,
) -> Result<(), String> {
    // Step 1: Parse Cargo.toml and discover source files
    let cargo = cmake::parse_cargo_toml(cargo_toml_path)?;
    let project_dir = cargo_toml_path
        .parent()
        .unwrap_or(Path::new("."));
    let crate_name = &cargo.package.name;
    let sources = cmake::collect_source_files(project_dir);

    if sources.is_empty() {
        return Err("No .rs source files found in src/".to_string());
    }

    // Create output directory
    std::fs::create_dir_all(output_dir)
        .map_err(|e| format!("Failed to create output dir: {}", e))?;

    // Report external dependencies
    let deps = cmake::extract_dependencies(&cargo);
    if !deps.is_empty() {
        println!("\nExternal dependencies:");
        for dep in &deps {
            if dep.is_local {
                println!("  {} (local: {})", dep.name, dep.path.as_deref().unwrap_or("?"));
            } else {
                println!("  {} = \"{}\" (external — types may need manual mapping)",
                    dep.name, dep.version.as_deref().unwrap_or("*"));
            }
        }
        println!();
    }

    println!("Transpiling crate '{}' ({} source files)", crate_name, sources.len());

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

        match transpile::transpile(&source, Some(&module_name)) {
            Ok(cpp_output) => {
                if let Err(e) = std::fs::write(&full_cppm_path, &cpp_output) {
                    eprintln!("  Error writing {}: {}", full_cppm_path.display(), e);
                    error_count += 1;
                    continue;
                }
                println!("  {} → {} (module: {})", rs_path.display(), cppm_path.display(), module_name);
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

    // Step 3: Generate CMakeLists.txt
    let cmake_content = cmake::generate_cmake(&cargo, &sources);
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
    let mut dir = input_path
        .parent()
        .unwrap_or(Path::new("."))
        .to_path_buf();

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
    let project_dir = cargo_toml_path
        .parent()
        .unwrap_or(Path::new("."));
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
        println!("  {} → {} (module: {})", source.display(), cppm.display(), module);
    }

    Ok(())
}

fn main() {
    let cli = Cli::parse();

    // Handle --crate: transpile entire crate
    if let Some(ref cargo_toml_path) = cli.crate_ {
        match transpile_crate(cargo_toml_path, &cli.output_dir, cli.verify) {
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

    let cpp_output = match transpile::transpile(&source, cli.module_name.as_deref()) {
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
