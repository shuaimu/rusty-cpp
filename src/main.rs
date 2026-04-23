use clang_sys::support::Clang;
use clap::Parser;
use colored::*;
use serde_json;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

#[macro_use]
mod debug_macros;

mod analysis;
mod diagnostics;
mod ir;
mod parser;
mod solver;

#[derive(clap::Parser, Debug)]
#[command(name = "rusty-cpp-checker")]
#[command(about = "A static analyzer that enforces Rust-like borrow checking rules for C++")]
#[command(version)]
#[command(
    long_about = "Rusty C++ Checker - A static analyzer that enforces Rust-like borrow checking rules for C++\n\n\
Environment variables:\n  \
CPLUS_INCLUDE_PATH  : Colon-separated list of C++ include directories\n  \
C_INCLUDE_PATH      : Colon-separated list of C include directories\n  \
CPATH               : Colon-separated list of C/C++ include directories\n  \
CPP_INCLUDE_PATH    : Custom include paths for this tool"
)]
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

#[derive(Debug, Default)]
struct CompileCommandConfig {
    include_paths: Vec<PathBuf>,
    clang_args: Vec<String>,
}

fn main() {
    let args = Args::parse();

    println!("{}", "Rusty C++ Checker".bold().blue());
    println!("Analyzing: {}", args.input.display());

    match analyze_file(
        &args.input,
        &args.include_paths,
        &args.defines,
        args.compile_commands.as_ref(),
    ) {
        Ok(results) => {
            if results.is_empty() {
                println!("{}", "✓ rusty-cpp: no violations found!".green());
            } else {
                println!(
                    "{}",
                    format!(
                        "✗ Found {} violation(s) in {}:",
                        results.len(),
                        args.input.display()
                    )
                    .red()
                );
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

fn analyze_file(
    path: &PathBuf,
    include_paths: &[PathBuf],
    defines: &[String],
    compile_commands: Option<&PathBuf>,
) -> Result<Vec<String>, String> {
    // Start with CLI-provided include paths
    let mut all_include_paths = include_paths.to_vec();
    let mut extra_clang_args: Vec<String> = Vec::new();
    let mut should_auto_detect_clang_includes = true;

    // Add include paths from environment variables
    all_include_paths.extend(extract_include_paths_from_env());

    // Extract additional include paths and compile flags from compile_commands.json if provided
    if let Some(cc_path) = compile_commands {
        let extracted = extract_compile_config_from_compile_commands(cc_path, path)?;
        // For module/libc++ builds we should trust compile_commands include paths.
        // Injecting auto-detected STL paths can mix libstdc++ and libc++ and produce
        // ambiguous declarations that break parsing.
        should_auto_detect_clang_includes = !extracted.clang_args.iter().any(|arg| {
            arg == "-stdlib"
                || arg.starts_with("-stdlib=")
                || arg == "-fmodules"
                || arg == "-fmodules-ts"
                || arg.starts_with("-fmodule-file=")
                || arg.starts_with("-fmodule-map-file=")
                || arg.starts_with("-fprebuilt-module-path=")
        });
        all_include_paths.extend(extracted.include_paths);
        extra_clang_args.extend(extracted.clang_args);
    }

    // Auto-detect C++ standard library paths from clang installation when needed.
    if should_auto_detect_clang_includes {
        all_include_paths.extend(extract_include_paths_from_clang());
    }

    // Parse included headers for lifetime annotations
    let mut header_cache = parser::HeaderCache::new();
    header_cache.set_include_paths(all_include_paths.clone());
    header_cache.parse_includes_from_source(path)?;

    // IMPORTANT: Also parse the source file itself for lifetime annotations
    // Without this, lifetime annotations in .cc/.cpp files are not recognized
    header_cache.parse_header(path)?;

    // Also parse external annotations from the source file itself (not just headers)
    // This allows annotations like @external: { function: [unsafe, ...] } in .cc/.cpp files
    if let Ok(source_content) = std::fs::read_to_string(path) {
        if let Err(e) = header_cache
            .external_annotations
            .parse_content(&source_content)
        {
            debug_println!(
                "DEBUG: Failed to parse external annotations from source file: {}",
                e
            );
        } else {
            debug_println!("DEBUG: Parsed external annotations from source file");
        }
    }

    // Parse the C++ file with include paths and defines
    let ast = parser::parse_cpp_file_with_includes_defines_and_args(
        path,
        &all_include_paths,
        defines,
        &extra_clang_args,
    )?;

    // Parse safety annotations using the unified rule
    let mut safety_context = parser::safety_annotations::parse_safety_annotations(path)?;

    // Merge safety annotations from headers into the context
    safety_context.merge_header_annotations(&header_cache);

    // Build a set of known safe functions from the safety context
    let mut known_safe_functions = std::collections::HashSet::new();
    for (func_sig, mode) in &safety_context.function_overrides {
        if *mode == parser::safety_annotations::SafetyMode::Safe {
            known_safe_functions.insert(func_sig.name.clone());
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
        if file_path.contains("/include/c++/")
            || file_path.contains("/bits/")
            || file_path.contains("/ext/")
            || file_path.contains("stl_")
            || file_path.contains("/lib/gcc/")
        {
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
            debug_println!(
                "DEBUG: Skipping system header function '{}' from {}",
                function.name,
                function.location.file
            );
            continue;
        }

        debug_println!(
            "DEBUG: Processing function '{}' from '{}' with {} statements",
            function.name,
            function.location.file,
            function.body.len()
        );

        // TEMPORARY WORKAROUND: Treat all operator overloads as unsafe
        // This bypasses annotation matching issues with template operators
        let is_operator = function.name.contains("operator");

        // Get the function's safety mode to pass to the pointer checker
        let mut function_safety = safety_context.get_function_safety(&function.name);

        // Override safety mode for operators - treat them as unsafe
        if is_operator {
            function_safety = parser::safety_annotations::SafetyMode::Unsafe;
            debug_println!(
                "DEBUG: Function '{}' is an operator overload, automatically treating as unsafe",
                function.name
            );
        }

        if safety_context.should_check_function(&function.name) && !is_operator {
            debug_println!(
                "DEBUG: Function '{}' is marked safe, performing checks",
                function.name
            );
            // Check for pointer operations (pass the function's safety mode)
            let pointer_errors = analysis::pointer_safety::check_parsed_function_for_pointers(
                function,
                function_safety,
            );
            violations.extend(pointer_errors);

            // Check for null safety (dereferencing potentially null pointers)
            let null_errors = analysis::null_safety::check_null_safety(function, function_safety);
            violations.extend(null_errors);

            // Check for initialization safety (use of uninitialized variables)
            let init_errors = analysis::initialization_tracking::check_initialization_safety(
                function,
                function_safety,
            );
            violations.extend(init_errors);

            // Check for pointer provenance (pointer subtraction/comparison between different allocations)
            let provenance_errors =
                analysis::pointer_provenance::check_pointer_provenance(function, function_safety);
            violations.extend(provenance_errors);

            // Check for alignment safety (misaligned pointer access)
            let alignment_errors =
                analysis::alignment_safety::check_alignment_safety(function, function_safety);
            violations.extend(alignment_errors);

            // Check for array bounds safety (out-of-bounds access)
            let bounds_errors =
                analysis::array_bounds::check_array_bounds(function, function_safety);
            violations.extend(bounds_errors);

            // Check for std::move on references (forbidden in @safe code)
            let std_move_errors =
                analysis::pointer_safety::check_std_move_on_references(function, function_safety);
            violations.extend(std_move_errors);

            // Check for lambda capture safety (reference captures forbidden in @safe)
            let lambda_errors = analysis::lambda_capture_safety::check_lambda_capture_safety(
                function,
                function_safety,
            );
            violations.extend(lambda_errors);

            // Check for calls to unsafe functions with external annotations from headers
            let propagation_errors =
                analysis::unsafe_propagation::check_unsafe_propagation_with_external(
                    function,
                    &safety_context,
                    &known_safe_functions,
                    Some(&header_cache.external_annotations),
                );
            violations.extend(propagation_errors);
        }
    }

    // Check for mutable fields in safe classes (before building IR)
    // Pass external annotations to skip STL internal types marked as unsafe_type
    let mutable_violations = analysis::mutable_checker::check_mutable_fields(
        &ast,
        &safety_context,
        Some(&header_cache.external_annotations),
    )?;
    violations.extend(mutable_violations);

    // Check inheritance safety (@interface validation, safe inheritance rules)
    let inheritance_violations =
        analysis::inheritance_safety::check_inheritance_safety(&ast.classes);
    violations.extend(inheritance_violations);

    // Check struct pointer member safety (pointer members must be non-null)
    let struct_pointer_violations =
        analysis::struct_pointer_safety::check_struct_pointer_safety(&ast.classes);
    violations.extend(struct_pointer_violations);

    // Check const propagation through pointer members (in @safe code, const propagates)
    let const_propagation_violations =
        analysis::const_propagation::check_const_propagation(&ast.functions, &ast.classes);
    violations.extend(const_propagation_violations);

    // Build intermediate representation with safety context
    let mut ir = ir::build_ir_with_safety_context(ast, safety_context.clone())?;

    // Phase 1: Populate lifetime information from annotations in HeaderCache
    for ir_func in &mut ir.functions {
        // Try to get the function signature from the header cache
        if let Some(signature) = header_cache.get_signature(&ir_func.name) {
            debug_println!(
                "DEBUG MAIN: Found lifetime annotations for function '{}'",
                ir_func.name
            );
            ir::populate_lifetime_info(ir_func, signature);
        }
    }

    // Perform borrow checking analysis with header knowledge and safety context
    let borrow_violations =
        analysis::check_borrows_with_safety_context(ir, header_cache, safety_context)?;
    violations.extend(borrow_violations);

    Ok(violations)
}

fn extract_compile_config_from_compile_commands(
    cc_path: &PathBuf,
    source_file: &PathBuf,
) -> Result<CompileCommandConfig, String> {
    let content = fs::read_to_string(cc_path)
        .map_err(|e| format!("Failed to read compile_commands.json: {}", e))?;

    let commands: Vec<serde_json::Value> = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse compile_commands.json: {}", e))?;

    let source_str = source_file.to_string_lossy();

    // Find the entry for our source file
    for entry in commands {
        if let Some(file) = entry.get("file").and_then(|f| f.as_str()) {
            if file.ends_with(&*source_str) || source_str.ends_with(file) {
                let directory = entry
                    .get("directory")
                    .and_then(|d| d.as_str())
                    .map(PathBuf::from)
                    .unwrap_or_else(|| PathBuf::from("."));

                if let Some(arguments) = entry.get("arguments").and_then(|a| a.as_array()) {
                    let tokens: Vec<String> = arguments
                        .iter()
                        .filter_map(|arg| arg.as_str().map(|s| s.to_string()))
                        .collect();
                    if !tokens.is_empty() {
                        return extract_compile_config_from_tokens(&tokens, &directory);
                    }
                }

                if let Some(command) = entry.get("command").and_then(|c| c.as_str()) {
                    return extract_compile_config_from_command(command, &directory);
                }
            }
        }
    }

    Ok(CompileCommandConfig::default()) // No matching entry found
}

fn absolutize_if_needed(path: &str, base_dir: &Path) -> PathBuf {
    let p = PathBuf::from(path);
    if p.is_absolute() {
        p
    } else {
        base_dir.join(p)
    }
}

fn strip_outer_quotes(s: &str) -> &str {
    if s.len() >= 2 {
        let bytes = s.as_bytes();
        if (bytes[0] == b'"' && bytes[s.len() - 1] == b'"')
            || (bytes[0] == b'\'' && bytes[s.len() - 1] == b'\'')
        {
            return &s[1..s.len() - 1];
        }
    }
    s
}

fn merge_compile_configs(base: &mut CompileCommandConfig, extra: CompileCommandConfig) {
    for include_path in extra.include_paths {
        if !base.include_paths.contains(&include_path) {
            base.include_paths.push(include_path);
        }
    }
    for arg in extra.clang_args {
        if !base.clang_args.contains(&arg) {
            base.clang_args.push(arg);
        }
    }
}

fn normalize_module_file_arg(value: &str, directory: &Path) -> Option<String> {
    let value = strip_outer_quotes(value);
    if let Some((module_name, module_path)) = value.split_once('=') {
        if module_path.is_empty() {
            return None;
        }
        let module_path = absolutize_if_needed(strip_outer_quotes(module_path), directory);
        if !module_path.exists() {
            return None;
        }
        return Some(format!("{}={}", module_name, module_path.display()));
    }

    let module_path = absolutize_if_needed(value, directory);
    if !module_path.exists() {
        return None;
    }
    Some(module_path.display().to_string())
}

fn parse_command_tokens(command: &str) -> Vec<String> {
    command.split_whitespace().map(|s| s.to_string()).collect()
}

fn extract_compile_config_from_tokens(
    tokens: &[String],
    directory: &Path,
) -> Result<CompileCommandConfig, String> {
    let mut config = CompileCommandConfig::default();

    let mut i = 0;
    while i < tokens.len() {
        let token = tokens[i].as_str();

        if token == "-I" && i + 1 < tokens.len() {
            // -I /path/to/include
            let include_dir =
                absolutize_if_needed(strip_outer_quotes(tokens[i + 1].as_str()), directory);
            if !config.include_paths.contains(&include_dir) {
                config.include_paths.push(include_dir);
            }
            i += 2;
        } else if let Some(path) = token.strip_prefix("-I") {
            // -I/path/to/include
            let path = strip_outer_quotes(path);
            if !path.is_empty() {
                let include_dir = absolutize_if_needed(path, directory);
                if !config.include_paths.contains(&include_dir) {
                    config.include_paths.push(include_dir);
                }
            }
            i += 1;
        } else if token == "-std" && i + 1 < tokens.len() {
            // -std gnu++23
            config.clang_args.push("-std".to_string());
            config.clang_args.push(tokens[i + 1].clone());
            i += 2;
        } else if token.starts_with("-std=") {
            // -std=gnu++23
            config.clang_args.push(token.to_string());
            i += 1;
        } else if token == "-stdlib" && i + 1 < tokens.len() {
            // -stdlib libc++
            config.clang_args.push("-stdlib".to_string());
            config.clang_args.push(tokens[i + 1].clone());
            i += 2;
        } else if token.starts_with("-stdlib=") {
            // -stdlib=libc++
            config.clang_args.push(token.to_string());
            i += 1;
        } else if token == "-x" && i + 1 < tokens.len() {
            // -x c++-module
            config.clang_args.push("-x".to_string());
            config.clang_args.push(tokens[i + 1].clone());
            i += 2;
        } else if token == "-fprebuilt-module-path" && i + 1 < tokens.len() {
            // -fprebuilt-module-path /path/to/pcms
            let raw = strip_outer_quotes(tokens[i + 1].as_str());
            if !raw.is_empty() {
                let module_dir = absolutize_if_needed(raw, directory);
                config
                    .clang_args
                    .push(format!("-fprebuilt-module-path={}", module_dir.display()));
            }
            i += 2;
        } else if let Some(module_dir) = token.strip_prefix("-fprebuilt-module-path=") {
            // -fprebuilt-module-path=/path/to/pcms
            let module_dir = strip_outer_quotes(module_dir);
            if !module_dir.is_empty() {
                let module_dir = absolutize_if_needed(module_dir, directory);
                config
                    .clang_args
                    .push(format!("-fprebuilt-module-path={}", module_dir.display()));
            }
            i += 1;
        } else if token == "-fmodule-file" && i + 1 < tokens.len() {
            // -fmodule-file std=/path/to/std.pcm or -fmodule-file /path/to/module.pcm
            if let Some(normalized) = normalize_module_file_arg(tokens[i + 1].as_str(), directory) {
                config
                    .clang_args
                    .push(format!("-fmodule-file={}", normalized));
            }
            i += 2;
        } else if let Some(value) = token.strip_prefix("-fmodule-file=") {
            // -fmodule-file=std=/path/to/std.pcm
            if let Some(normalized) = normalize_module_file_arg(value, directory) {
                config
                    .clang_args
                    .push(format!("-fmodule-file={}", normalized));
            }
            i += 1;
        } else if token == "-fmodule-map-file" && i + 1 < tokens.len() {
            // -fmodule-map-file /path/to/module.modulemap
            let map_path =
                absolutize_if_needed(strip_outer_quotes(tokens[i + 1].as_str()), directory);
            config
                .clang_args
                .push(format!("-fmodule-map-file={}", map_path.display()));
            i += 2;
        } else if let Some(map_path) = token.strip_prefix("-fmodule-map-file=") {
            // -fmodule-map-file=/path/to/module.modulemap
            let map_path = absolutize_if_needed(strip_outer_quotes(map_path), directory);
            config
                .clang_args
                .push(format!("-fmodule-map-file={}", map_path.display()));
            i += 1;
        } else if token == "-fmodules" || token == "-fmodules-ts" {
            config.clang_args.push(token.to_string());
            i += 1;
        } else if let Some(response_file) = token.strip_prefix('@') {
            // CMake/Ninja module support often stores module mappings in response files.
            // Expand them so libclang sees -fmodule-file/-x flags while parsing.
            let response_file = strip_outer_quotes(response_file);
            if !response_file.is_empty() {
                let response_file = absolutize_if_needed(response_file, directory);
                match fs::read_to_string(&response_file) {
                    Ok(content) => {
                        let response_tokens: Vec<String> =
                            content.split_whitespace().map(|s| s.to_string()).collect();
                        let response_config =
                            extract_compile_config_from_tokens(&response_tokens, directory)?;
                        merge_compile_configs(&mut config, response_config);
                    }
                    Err(e) => {
                        eprintln!(
                            "Warning: skipping missing compiler response file '{}': {}",
                            response_file.display(),
                            e
                        );
                    }
                }
            }
            i += 1;
        } else {
            i += 1;
        }
    }

    Ok(config)
}

fn extract_compile_config_from_command(
    command: &str,
    directory: &Path,
) -> Result<CompileCommandConfig, String> {
    let tokens = parse_command_tokens(command);
    extract_compile_config_from_tokens(&tokens, directory)
}

fn extract_include_paths_from_env() -> Vec<PathBuf> {
    let mut paths = Vec::new();

    // Standard C++ include path environment variables
    // Priority order: CPLUS_INCLUDE_PATH > C_INCLUDE_PATH > CPATH
    let env_vars = [
        "CPLUS_INCLUDE_PATH", // C++ specific
        "C_INCLUDE_PATH",     // C specific (but we might use it)
        "CPATH",              // Both C and C++
        "CPP_INCLUDE_PATH",   // Custom variable for our tool
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
        eprintln!(
            "Found {} include path(s) from environment variables",
            paths.len()
        );
    }

    paths
}

/// Extract C++ standard library include paths using clang_sys
/// This queries the system's clang installation to find STL headers and builtin headers
fn extract_include_paths_from_clang() -> Vec<PathBuf> {
    let mut paths = Vec::new();

    // Try to find clang and get its C++ search paths
    match Clang::find(None, &[]) {
        Some(clang) => {
            // First, add the Clang resource directory for built-in headers (stdarg.h, etc.)
            // This is essential for LibClang to parse code that includes standard headers
            //
            // IMPORTANT: We need to use the resource directory that matches the actual libclang
            // version being linked, not the clang binary on PATH. The clang binary and libclang
            // can be different versions (e.g., clang-14 on PATH but libclang-16 linked).
            //
            // Try to detect the actual libclang version and use its resource directory.
            if let Some(resource_include) = find_libclang_resource_include() {
                if !paths.contains(&resource_include) {
                    paths.push(resource_include);
                }
            } else if let Ok(output) = std::process::Command::new(&clang.path)
                .arg("-print-resource-dir")
                .output()
            {
                // Fallback: use clang binary's resource dir
                if output.status.success() {
                    let resource_dir = String::from_utf8_lossy(&output.stdout).trim().to_string();
                    let builtin_include = PathBuf::from(&resource_dir).join("include");
                    if builtin_include.exists() && !paths.contains(&builtin_include) {
                        paths.push(builtin_include);
                    }
                }
            }

            if let Some(cpp_paths) = clang.cpp_search_paths {
                // Check if we have GCC paths - if so, filter out LLVM/clang paths to avoid conflicts
                let has_gcc_paths = cpp_paths.iter().any(|p| {
                    let s = p.to_string_lossy();
                    s.contains("/gcc/") || s.contains("/g++/") || s.contains("/c++/")
                });

                for path in cpp_paths {
                    let path_str = path.to_string_lossy();

                    // Skip LLVM/clang internal paths if we have GCC paths
                    // These can conflict with GCC's stdint definitions
                    if has_gcc_paths && (path_str.contains("/llvm") || path_str.contains("/clang"))
                    {
                        continue;
                    }

                    if path.exists() && !paths.contains(&path) {
                        paths.push(path);
                    }
                }
            }

            // Add system C include paths that are needed for libc headers (stdint.h, etc.)
            // clang_sys only provides C++ paths, but we also need the C library paths
            // for headers like bits/stdint-intn.h which define int8_t, int16_t, etc.
            add_system_c_include_paths(&mut paths);

            if !paths.is_empty() {
                eprintln!("Auto-detected {} C++ include path(s)", paths.len());
            }
        }
        None => {
            // Clang not found - this is okay, user can specify paths manually
            debug_println!(
                "DEBUG: Could not find clang installation for auto-detecting include paths"
            );
            // Still try to add system C paths even without clang
            add_system_c_include_paths(&mut paths);
        }
    }

    paths
}

/// Find the resource include directory for the actual libclang being linked
/// This handles the case where clang binary version differs from libclang version
fn find_libclang_resource_include() -> Option<PathBuf> {
    // Try to find libclang version by checking common LLVM installation paths
    // Search for the newest version first (higher versions are typically more compatible)
    for version in (14..=20).rev() {
        // Try versioned path (e.g., /usr/lib/llvm-16/lib/clang/16/include)
        let versioned_path = PathBuf::from(format!(
            "/usr/lib/llvm-{}/lib/clang/{}/include",
            version, version
        ));
        if versioned_path.exists() {
            // Verify this version's libclang is actually what we're linked against
            let libclang_path =
                PathBuf::from(format!("/lib/x86_64-linux-gnu/libclang-{}.so", version));
            let libclang_path_alt =
                PathBuf::from(format!("/usr/lib/x86_64-linux-gnu/libclang-{}.so", version));
            if libclang_path.exists() || libclang_path_alt.exists() {
                return Some(versioned_path);
            }
        }

        // Also try the format with full version (e.g., /usr/lib/llvm-14/lib/clang/14.0.6/include)
        for minor in (0..=9).rev() {
            for patch in (0..=9).rev() {
                let full_version_path = PathBuf::from(format!(
                    "/usr/lib/llvm-{}/lib/clang/{}.{}.{}/include",
                    version, version, minor, patch
                ));
                if full_version_path.exists() {
                    let libclang_path =
                        PathBuf::from(format!("/lib/x86_64-linux-gnu/libclang-{}.so", version));
                    let libclang_path_alt =
                        PathBuf::from(format!("/usr/lib/x86_64-linux-gnu/libclang-{}.so", version));
                    if libclang_path.exists() || libclang_path_alt.exists() {
                        return Some(full_version_path);
                    }
                }
            }
        }
    }

    // Fallback: Try to find any clang resource directory
    for version in (14..=20).rev() {
        let versioned_path = PathBuf::from(format!(
            "/usr/lib/llvm-{}/lib/clang/{}/include",
            version, version
        ));
        if versioned_path.exists() {
            return Some(versioned_path);
        }
    }

    None
}

/// Add system C include paths needed for libc headers (stdint.h, etc.)
/// These paths are typically used as -internal-externc-isystem by clang
fn add_system_c_include_paths(paths: &mut Vec<PathBuf>) {
    // Platform-specific system include paths
    #[cfg(target_os = "linux")]
    {
        // Linux: need architecture-specific include path for bits/ headers
        let arch_include = PathBuf::from("/usr/include/x86_64-linux-gnu");
        if arch_include.exists() && !paths.contains(&arch_include) {
            paths.push(arch_include);
        }

        // Also try aarch64 for ARM64
        let arm_include = PathBuf::from("/usr/include/aarch64-linux-gnu");
        if arm_include.exists() && !paths.contains(&arm_include) {
            paths.push(arm_include);
        }

        // Generic /usr/include (lower priority, add last)
        let usr_include = PathBuf::from("/usr/include");
        if usr_include.exists() && !paths.contains(&usr_include) {
            paths.push(usr_include);
        }
    }

    #[cfg(target_os = "macos")]
    {
        // macOS: Xcode SDK paths
        if let Ok(output) = std::process::Command::new("xcrun")
            .args(["--show-sdk-path"])
            .output()
        {
            if output.status.success() {
                let sdk_path = String::from_utf8_lossy(&output.stdout).trim().to_string();
                let sdk_include = PathBuf::from(&sdk_path).join("usr/include");
                if sdk_include.exists() && !paths.contains(&sdk_include) {
                    paths.push(sdk_include);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_module_flags_from_response_file() {
        let temp_dir = tempfile::tempdir().expect("create temp dir");
        let build_dir = temp_dir.path().to_path_buf();
        let response_file = build_dir.join("modules.modmap");
        let pcm_dir = build_dir.join("CMakeFiles");

        fs::create_dir_all(&pcm_dir).expect("create pcm dir");
        fs::write(pcm_dir.join("std.pcm"), "").expect("create std pcm");
        fs::write(pcm_dir.join("rrr.pcm"), "").expect("create rrr pcm");

        fs::write(
            &response_file,
            "-x c++-module\n\
             -fmodule-file=std=CMakeFiles/std.pcm\n\
             -fmodule-file=rrr=CMakeFiles/rrr.pcm\n",
        )
        .expect("write response file");

        let command = "clang++ -Iinclude -std=gnu++23 @modules.modmap -c src/file.cpp";
        let config =
            extract_compile_config_from_command(command, &build_dir).expect("extract config");

        assert!(config.include_paths.contains(&build_dir.join("include")));
        assert!(config.clang_args.contains(&"-x".to_string()));
        assert!(config.clang_args.contains(&"c++-module".to_string()));
        assert!(config.clang_args.contains(&format!(
            "-fmodule-file=std={}",
            build_dir.join("CMakeFiles/std.pcm").display()
        )));
        assert!(config.clang_args.contains(&format!(
            "-fmodule-file=rrr={}",
            build_dir.join("CMakeFiles/rrr.pcm").display()
        )));
    }

    #[test]
    fn extracts_from_compile_commands_arguments_field() {
        let temp_dir = tempfile::tempdir().expect("create temp dir");
        let build_dir = temp_dir.path().to_path_buf();
        let response_file = build_dir.join("modules.modmap");
        let source_file = build_dir.join("src/file.cpp");
        let compile_commands = build_dir.join("compile_commands.json");
        let pcm_dir = build_dir.join("CMakeFiles");

        fs::create_dir_all(source_file.parent().expect("source parent")).expect("create src dir");
        fs::create_dir_all(&pcm_dir).expect("create pcm dir");
        fs::write(&source_file, "int main() { return 0; }\n").expect("write source");
        fs::write(pcm_dir.join("std.pcm"), "").expect("create std pcm");
        fs::write(pcm_dir.join("rrr.pcm"), "").expect("create rrr pcm");
        fs::write(
            &response_file,
            "-fmodule-file=std=CMakeFiles/std.pcm\n\
             -fmodule-file=rrr=CMakeFiles/rrr.pcm\n",
        )
        .expect("write response file");

        let cc = serde_json::json!([
            {
                "directory": build_dir.display().to_string(),
                "file": source_file.display().to_string(),
                "arguments": [
                    "clang++",
                    "-Iinclude",
                    "@modules.modmap",
                    "-std=gnu++23",
                    "-c",
                    source_file.display().to_string()
                ]
            }
        ]);
        fs::write(
            &compile_commands,
            serde_json::to_string_pretty(&cc).expect("serialize compile_commands"),
        )
        .expect("write compile_commands");

        let config = extract_compile_config_from_compile_commands(&compile_commands, &source_file)
            .expect("extract config from compile_commands");

        assert!(config.include_paths.contains(&build_dir.join("include")));
        assert!(config.clang_args.contains(&format!(
            "-fmodule-file=std={}",
            build_dir.join("CMakeFiles/std.pcm").display()
        )));
        assert!(config.clang_args.contains(&format!(
            "-fmodule-file=rrr={}",
            build_dir.join("CMakeFiles/rrr.pcm").display()
        )));
    }
}
