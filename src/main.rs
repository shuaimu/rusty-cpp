use clap::Parser;
use colored::*;
use std::path::PathBuf;
use std::fs;
use std::env;
use serde_json;

#[macro_use]
mod debug_macros;

mod parser;
mod ir;
mod analysis;
mod solver;
mod diagnostics;

#[derive(clap::Parser, Debug)]
#[command(name = "rusty-cpp-checker")]
#[command(about = "A static analyzer that enforces Rust-like borrow checking rules for C++")]
#[command(version)]
#[command(long_about = "Rusty C++ Checker - A static analyzer that enforces Rust-like borrow checking rules for C++\n\n\
Environment variables:\n  \
CPLUS_INCLUDE_PATH  : Colon-separated list of C++ include directories\n  \
C_INCLUDE_PATH      : Colon-separated list of C include directories\n  \
CPATH               : Colon-separated list of C/C++ include directories\n  \
CPP_INCLUDE_PATH    : Custom include paths for this tool")]
struct Args {
    /// C++ source file to analyze
    #[arg(value_name = "FILE")]
    input: PathBuf,

    /// Include paths for header files (can be specified multiple times)
    #[arg(short = 'I', value_name = "DIR")]
    include_paths: Vec<PathBuf>,

    /// Preprocessor definitions (can be specified multiple times)
    /// Example: -D CONFIG_H=\"config.h\" -D DEBUG=1
    #[arg(short = 'D', value_name = "DEFINE")]
    defines: Vec<String>,

    /// Path to compile_commands.json for extracting include paths
    #[arg(long, value_name = "FILE")]
    compile_commands: Option<PathBuf>,

    /// Verbosity level
    #[arg(short, long, action = clap::ArgAction::Count)]
    verbose: u8,

    /// Output format (text, json)
    #[arg(long, default_value = "text")]
    format: String,
}

fn main() {
    let args = Args::parse();
    
    println!("{}", "Rusty C++ Checker".bold().blue());
    println!("Analyzing: {}", args.input.display());
    
    match analyze_file(&args.input, &args.include_paths, &args.defines, args.compile_commands.as_ref()) {
        Ok(results) => {
            if results.is_empty() {
                println!("{}", "✓ rusty-cpp: no violations found!".green());
            } else {
                println!("{}", format!("✗ Found {} violation(s) in {}:", results.len(), args.input.display()).red());
                for error in results {
                    println!("{}", error);
                }
                std::process::exit(1);
            }
        }
        Err(e) => {
            eprintln!("{}: {}", "Error".red().bold(), e);
            std::process::exit(1);
        }
    }
}

fn analyze_file(path: &PathBuf, include_paths: &[PathBuf], defines: &[String], compile_commands: Option<&PathBuf>) -> Result<Vec<String>, String> {
    // Start with CLI-provided include paths
    let mut all_include_paths = include_paths.to_vec();
    
    // Add include paths from environment variables
    all_include_paths.extend(extract_include_paths_from_env());
    
    // Extract include paths from compile_commands.json if provided
    if let Some(cc_path) = compile_commands {
        let extracted_paths = extract_include_paths_from_compile_commands(cc_path, path)?;
        all_include_paths.extend(extracted_paths);
    }
    
    // Parse included headers for lifetime annotations
    let mut header_cache = parser::HeaderCache::new();
    header_cache.set_include_paths(all_include_paths.clone());
    header_cache.parse_includes_from_source(path)?;

    // Also parse external annotations from the source file itself (not just headers)
    // This allows annotations like @external: { function: [unsafe, ...] } in .cc/.cpp files
    if let Ok(source_content) = std::fs::read_to_string(path) {
        if let Err(e) = header_cache.external_annotations.parse_content(&source_content) {
            debug_println!("DEBUG: Failed to parse external annotations from source file: {}", e);
        } else {
            debug_println!("DEBUG: Parsed external annotations from source file");
        }
    }

    // Parse the C++ file with include paths and defines
    let ast = parser::parse_cpp_file_with_includes_and_defines(path, &all_include_paths, defines)?;
    
    // Parse safety annotations using the unified rule
    let mut safety_context = parser::safety_annotations::parse_safety_annotations(path)?;
    
    // Merge safety annotations from headers into the context
    safety_context.merge_header_annotations(&header_cache);
    
    // Build a set of known safe functions from the safety context
    let mut known_safe_functions = std::collections::HashSet::new();
    for (func_name, mode) in &safety_context.function_overrides {
        if *mode == parser::safety_annotations::SafetyMode::Safe {
            known_safe_functions.insert(func_name.clone());
        }
    }
    
    // Helper function to check if a file or function is from a system header
    fn is_system_header_or_std(file_path: &str, _function_name: &str) -> bool {
        // Common system header paths (absolute)
        let system_paths = [
            "/usr/include",
            "/usr/local/include",
            "/opt/homebrew/include",
            "/Library/Developer",
            "C:\\Program Files",
            "/Applications/Xcode.app",
        ];

        for path in &system_paths {
            if file_path.starts_with(path) {
                return true;
            }
        }

        // STL and system library patterns (works for relative paths too)
        if file_path.contains("/include/c++/") ||
           file_path.contains("/bits/") ||
           file_path.contains("/ext/") ||
           file_path.contains("stl_") ||
           file_path.contains("/lib/gcc/") {
            return true;
        }

        // Also skip the project's include/ directory (third-party headers like rusty::Box)
        if file_path.contains("/include/rusty/") || file_path.contains("/include/unified_") {
            return true;
        }

        false
    }

    // Check for unsafe pointer operations and unsafe propagation in safe functions
    let mut violations = Vec::new();
    debug_println!("DEBUG: Found {} functions in AST", ast.functions.len());
    for function in &ast.functions {
        // Skip system header functions - they shouldn't be analyzed internally
        if is_system_header_or_std(&function.location.file, &function.name) {
            debug_println!("DEBUG: Skipping system header function '{}' from {}", function.name, function.location.file);
            continue;
        }

        debug_println!("DEBUG: Processing function '{}' from '{}' with {} statements", function.name, function.location.file, function.body.len());
        if safety_context.should_check_function(&function.name) {
            debug_println!("DEBUG: Function '{}' is marked safe, performing checks", function.name);
            // Check for pointer operations
            let pointer_errors = analysis::pointer_safety::check_parsed_function_for_pointers(function);
            violations.extend(pointer_errors);

            // Check for calls to unsafe functions with external annotations from headers
            let propagation_errors = analysis::unsafe_propagation::check_unsafe_propagation_with_external(
                function,
                &safety_context,
                &known_safe_functions,
                Some(&header_cache.external_annotations)
            );
            violations.extend(propagation_errors);
        }
    }

    // Check for mutable fields in safe classes (before building IR)
    let mutable_violations = analysis::mutable_checker::check_mutable_fields(&ast, &safety_context)?;
    violations.extend(mutable_violations);

    // Build intermediate representation with safety context
    let ir = ir::build_ir_with_safety_context(ast, safety_context.clone())?;

    // Perform borrow checking analysis with header knowledge and safety context
    let borrow_violations = analysis::check_borrows_with_safety_context(ir, header_cache, safety_context)?;
    violations.extend(borrow_violations);

    Ok(violations)
}

fn extract_include_paths_from_compile_commands(cc_path: &PathBuf, source_file: &PathBuf) -> Result<Vec<PathBuf>, String> {
    let content = fs::read_to_string(cc_path)
        .map_err(|e| format!("Failed to read compile_commands.json: {}", e))?;
    
    let commands: Vec<serde_json::Value> = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse compile_commands.json: {}", e))?;
    
    let source_str = source_file.to_string_lossy();
    
    // Find the entry for our source file
    for entry in commands {
        if let Some(file) = entry.get("file").and_then(|f| f.as_str()) {
            if file.ends_with(&*source_str) || source_str.ends_with(file) {
                if let Some(command) = entry.get("command").and_then(|c| c.as_str()) {
                    return extract_include_paths_from_command(command);
                }
            }
        }
    }
    
    Ok(Vec::new()) // No matching entry found
}

fn extract_include_paths_from_command(command: &str) -> Result<Vec<PathBuf>, String> {
    let mut paths = Vec::new();
    let parts: Vec<&str> = command.split_whitespace().collect();
    
    let mut i = 0;
    while i < parts.len() {
        if parts[i] == "-I" && i + 1 < parts.len() {
            // -I /path/to/include
            paths.push(PathBuf::from(parts[i + 1]));
            i += 2;
        } else if parts[i].starts_with("-I") {
            // -I/path/to/include
            let path = &parts[i][2..];
            paths.push(PathBuf::from(path));
            i += 1;
        } else {
            i += 1;
        }
    }
    
    Ok(paths)
}

fn extract_include_paths_from_env() -> Vec<PathBuf> {
    let mut paths = Vec::new();
    
    // Standard C++ include path environment variables
    // Priority order: CPLUS_INCLUDE_PATH > C_INCLUDE_PATH > CPATH
    let env_vars = [
        "CPLUS_INCLUDE_PATH",  // C++ specific
        "C_INCLUDE_PATH",       // C specific (but we might use it)
        "CPATH",                // Both C and C++
        "CPP_INCLUDE_PATH",     // Custom variable for our tool
    ];
    
    for var_name in &env_vars {
        if let Ok(env_value) = env::var(var_name) {
            // Split by platform-specific path separator
            let separator = if cfg!(windows) { ';' } else { ':' };
            
            for path_str in env_value.split(separator) {
                let path_str = path_str.trim();
                if !path_str.is_empty() {
                    let path = PathBuf::from(path_str);
                    // Only add if it exists and we haven't already added it
                    if path.exists() && !paths.iter().any(|p| p == &path) {
                        paths.push(path);
                    }
                }
            }
        }
    }
    
    // Print info about environment paths if verbose mode is enabled
    if !paths.is_empty() {
        eprintln!("Found {} include path(s) from environment variables", paths.len());
    }
    
    paths
}
