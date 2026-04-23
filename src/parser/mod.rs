use clang::{Clang, Entity, EntityKind, Index};
use std::path::Path;

pub mod annotations;
pub mod ast_visitor;
pub mod external_annotations;
pub mod header_cache;
pub mod safety_annotations;

pub use ast_visitor::{
    CastKind, CppAst, Expression, Function, MethodQualifier, MoveKind, Statement,
};
#[allow(unused_imports)]
pub use ast_visitor::{SourceLocation, Variable};
pub use header_cache::HeaderCache;

use std::fs;
use std::io::{BufRead, BufReader};

fn is_module_driver_flag(arg: &str) -> bool {
    arg == "-fmodules"
        || arg == "-fmodules-ts"
        || arg == "-fmodule-file"
        || arg.starts_with("-fmodule-file=")
        || arg == "-fmodule-map-file"
        || arg.starts_with("-fmodule-map-file=")
        || arg == "-fprebuilt-module-path"
        || arg.starts_with("-fprebuilt-module-path=")
        || arg == "-fmodule-output"
        || arg.starts_with("-fmodule-output=")
}

fn module_file_is_std(value: &str) -> bool {
    let value = value.trim_matches('"').trim_matches('\'');
    if let Some((module_name, _module_path)) = value.split_once('=') {
        module_name == "std" || module_name.starts_with("std.") || module_name.starts_with("std:")
    } else {
        value.contains("std.pcm") || value.contains("std.compat.pcm")
    }
}

fn filter_module_args_for_recovery(args: &[String], keep_std_module_files: bool) -> Vec<String> {
    let mut filtered = Vec::with_capacity(args.len());
    let mut i = 0;

    while i < args.len() {
        let arg = args[i].as_str();

        // Drop explicit module language mode in fallback passes.
        if arg == "-x" && i + 1 < args.len() && args[i + 1] == "c++-module" {
            i += 2;
            continue;
        }
        if arg == "c++-module" {
            i += 1;
            continue;
        }

        if arg == "-fmodule-file" && i + 1 < args.len() {
            let value = args[i + 1].as_str();
            if keep_std_module_files && module_file_is_std(value) {
                filtered.push(arg.to_string());
                filtered.push(args[i + 1].clone());
            }
            i += 2;
            continue;
        }
        if let Some(value) = arg.strip_prefix("-fmodule-file=") {
            if keep_std_module_files && module_file_is_std(value) {
                filtered.push(args[i].clone());
            }
            i += 1;
            continue;
        }

        // Drop module graph wiring in fallback passes.
        if arg == "-fmodule-map-file" || arg == "-fprebuilt-module-path" || arg == "-fmodule-output"
        {
            i += if i + 1 < args.len() { 2 } else { 1 };
            continue;
        }
        if arg == "-fmodules"
            || arg == "-fmodules-ts"
            || arg.starts_with("-fmodule-map-file=")
            || arg.starts_with("-fprebuilt-module-path=")
            || arg.starts_with("-fmodule-output=")
        {
            i += 1;
            continue;
        }

        filtered.push(args[i].clone());
        i += 1;
    }

    filtered
}

#[allow(dead_code)]
pub fn parse_cpp_file(path: &Path) -> Result<CppAst, String> {
    parse_cpp_file_with_includes(path, &[])
}

#[allow(dead_code)]
pub fn parse_cpp_file_with_includes(
    path: &Path,
    include_paths: &[std::path::PathBuf],
) -> Result<CppAst, String> {
    parse_cpp_file_with_includes_and_defines(path, include_paths, &[])
}

pub fn parse_cpp_file_with_includes_and_defines(
    path: &Path,
    include_paths: &[std::path::PathBuf],
    defines: &[String],
) -> Result<CppAst, String> {
    parse_cpp_file_with_includes_defines_and_args(path, include_paths, defines, &[])
}

pub fn parse_cpp_file_with_includes_defines_and_args(
    path: &Path,
    include_paths: &[std::path::PathBuf],
    defines: &[String],
    extra_clang_args: &[String],
) -> Result<CppAst, String> {
    // Initialize Clang
    let clang = Clang::new().map_err(|e| format!("Failed to initialize Clang: {:?}", e))?;

    let index = Index::new(&clang, false, false);

    // Build arguments with include paths and defines
    let mut args = vec![
        "-std=c++20".to_string(),
        "-xc++".to_string(),
        // Add flags to make parsing more lenient
        "-fno-delayed-template-parsing".to_string(),
        "-fparse-all-comments".to_string(),
        // Suppress certain errors that don't affect borrow checking
        "-Wno-everything".to_string(),
        // Don't fail on missing includes
        "-Wno-error".to_string(),
    ];

    // Add extra compile flags (for example module flags extracted from compile_commands.json).
    for extra_arg in extra_clang_args {
        args.push(extra_arg.clone());
    }

    // Add include paths
    for include_path in include_paths {
        args.push(format!("-I{}", include_path.display()));
    }

    // Add preprocessor definitions
    for define in defines {
        args.push(format!("-D{}", define));
    }

    let parse_with_args = |parse_args: &[String]| {
        index
            .parser(path)
            .arguments(&parse_args.iter().map(|s| s.as_str()).collect::<Vec<_>>())
            .detailed_preprocessing_record(true)
            .skip_function_bodies(false) // We need function bodies for analysis
            .incomplete(true) // Allow incomplete translation units
            .parse()
    };

    // Parse the translation unit. If prebuilt module deserialization fails, retry with
    // progressively less module state so analysis can still proceed.
    let tu = match parse_with_args(&args) {
        Ok(tu) => tu,
        Err(first_error) => {
            let first_error_text = format!("{:?}", first_error);
            let has_module_args = args.iter().any(|arg| is_module_driver_flag(arg))
                || args
                    .windows(2)
                    .any(|pair| pair[0] == "-x" && pair[1] == "c++-module");

            if !has_module_args || !first_error_text.contains("AstDeserialization") {
                return Err(format!("Failed to parse file: {:?}", first_error));
            }

            let std_only_args = filter_module_args_for_recovery(&args, true);
            if std_only_args != args {
                eprintln!(
                    "Warning: module deserialization failed, retrying with std-only module inputs"
                );
                match parse_with_args(&std_only_args) {
                    Ok(tu) => tu,
                    Err(second_error) => {
                        let second_error_text = format!("{:?}", second_error);
                        if !second_error_text.contains("AstDeserialization") {
                            return Err(format!("Failed to parse file: {:?}", second_error));
                        }

                        let no_module_args = filter_module_args_for_recovery(&args, false);
                        if no_module_args != std_only_args {
                            eprintln!(
                                "Warning: module deserialization still failed, retrying without prebuilt module inputs"
                            );
                        }
                        parse_with_args(&no_module_args)
                            .map_err(|e| format!("Failed to parse file: {:?}", e))?
                    }
                }
            } else {
                return Err(format!("Failed to parse file: {:?}", first_error));
            }
        }
    };

    fn is_module_resolution_fatal(diagnostic_text: &str) -> bool {
        // C++ module graphs are often incomplete when the checker runs before all PCM files
        // are produced. Keep analysis running with partial AST in that case.
        diagnostic_text.contains("module '") && diagnostic_text.contains("not found")
            || (diagnostic_text.contains("module file") && diagnostic_text.contains("not found"))
            || diagnostic_text.contains("could not build module")
    }

    // Check for diagnostics but only fail on fatal errors
    let diagnostics = tu.get_diagnostics();
    let mut has_fatal = false;
    if !diagnostics.is_empty() {
        for diag in &diagnostics {
            let text = diag.get_text();
            // Only fail on fatal errors, ignore regular errors
            if diag.get_severity() >= clang::diagnostic::Severity::Fatal {
                if is_module_resolution_fatal(&text) {
                    eprintln!("Warning (suppressed module fatal): {}", text);
                } else {
                    has_fatal = true;
                    eprintln!("Fatal error: {}", text);
                }
            } else if diag.get_severity() >= clang::diagnostic::Severity::Error {
                // Log errors but don't fail
                eprintln!("Warning (suppressed error): {}", text);
            }
        }
    }

    if has_fatal {
        return Err("Fatal parsing errors encountered".to_string());
    }

    // Visit the AST
    let mut ast = CppAst::new();
    let root = tu.get_entity();
    let main_file_path = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    visit_entity(&root, &mut ast, &main_file_path);

    Ok(ast)
}

fn visit_entity(entity: &Entity, ast: &mut CppAst, main_file: &Path) {
    use crate::debug_println;
    // Only debug function and class-related entities
    if matches!(
        entity.get_kind(),
        EntityKind::FunctionDecl
            | EntityKind::Method
            | EntityKind::FunctionTemplate
            | EntityKind::ClassTemplate
    ) {
        // Check if this is a template specialization
        let template_kind = entity.get_template_kind();
        let template_args = entity.get_template_arguments();
        debug_println!(
            "DEBUG PARSE: Visiting entity: kind={:?}, name={:?}, is_definition={}, template_kind={:?}, has_template_args={}",
            entity.get_kind(),
            entity.get_name(),
            entity.is_definition(),
            template_kind,
            template_args.is_some()
        );
    }

    // Extract entities from all files (main file and headers)
    // The analysis phase will distinguish between system headers and user code
    // System headers: track for safety status, but skip borrow checking
    // User code: full borrow checking and safety analysis
    let _main_file = main_file; // Keep parameter for future use

    match entity.get_kind() {
        EntityKind::FunctionDecl | EntityKind::Method => {
            debug_println!(
                "DEBUG PARSE: Found FunctionDecl: name={:?}, is_definition={}, kind={:?}",
                entity.get_name(),
                entity.is_definition(),
                entity.get_kind()
            );
            // Extract all function definitions (from main file and headers)
            if entity.is_definition() {
                let func = ast_visitor::extract_function(entity);
                ast.functions.push(func);
            }
        }
        EntityKind::FunctionTemplate => {
            // Template free functions: extract the template declaration to analyze with generic types
            // We don't need instantiations - our borrow/move checking works on generic types!
            debug_println!(
                "DEBUG PARSE: Found FunctionTemplate: {:?}, is_definition={}",
                entity.get_name(),
                entity.is_definition()
            );

            // The FunctionTemplateDecl IS the function entity in LibClang
            // Its children are: TemplateTypeParameter, ParmDecl, CompoundStmt
            // We extract the function directly from this entity
            if entity.is_definition() {
                debug_println!(
                    "DEBUG PARSE: Extracting template function from FunctionTemplate entity"
                );
                let func = ast_visitor::extract_function(entity);
                debug_println!("DEBUG PARSE: Extracted template function: {}", func.name);
                ast.functions.push(func);
            }
        }
        EntityKind::ClassTemplate => {
            // Phase 3: Template classes
            debug_println!(
                "DEBUG PARSE: Found ClassTemplate: {:?}, is_definition={}",
                entity.get_name(),
                entity.is_definition()
            );

            // ClassTemplateDecl works similarly to FunctionTemplateDecl
            // Its children are: TemplateTypeParameter, CXXRecordDecl
            // Extract the class directly from this entity
            if entity.is_definition() {
                debug_println!("DEBUG PARSE: Extracting template class from ClassTemplate entity");
                let class = ast_visitor::extract_class(entity);
                debug_println!("DEBUG PARSE: Extracted template class: {}", class.name);
                ast.classes.push(class);
            }
        }
        EntityKind::ClassDecl | EntityKind::StructDecl => {
            // Regular (non-template) classes and structs
            debug_println!(
                "DEBUG PARSE: Found ClassDecl/StructDecl: {:?}, is_definition={}",
                entity.get_name(),
                entity.is_definition()
            );

            if entity.is_definition() {
                debug_println!("DEBUG PARSE: Extracting regular class from ClassDecl entity");
                let class = ast_visitor::extract_class(entity);
                debug_println!("DEBUG PARSE: Extracted class: {}", class.name);
                ast.classes.push(class);
            }
        }
        EntityKind::CallExpr => {
            // Note: We don't need to handle template instantiations here.
            // Template functions are analyzed via their declarations (with generic types).
            // CallExpr references to instantiations don't have bodies in LibClang anyway.
        }
        EntityKind::VarDecl => {
            // Extract all global variables (from main file and headers)
            let var = ast_visitor::extract_variable(entity);
            ast.global_variables.push(var);
        }
        _ => {}
    }

    // Recursively visit children
    for child in entity.get_children() {
        visit_entity(&child, ast, main_file);
    }
}

/// Check if the file has @safe annotation at the beginning
#[allow(dead_code)]
pub fn check_file_safety_annotation(path: &Path) -> Result<bool, String> {
    let file =
        fs::File::open(path).map_err(|e| format!("Failed to open file for safety check: {}", e))?;

    let reader = BufReader::new(file);

    // Check first 20 lines for @safe annotation (before any code)
    for (line_num, line_result) in reader.lines().enumerate() {
        if line_num > 20 {
            break; // Don't look too far
        }

        let line = line_result.map_err(|e| format!("Failed to read line: {}", e))?;
        let trimmed = line.trim();

        // Skip empty lines
        if trimmed.is_empty() {
            continue;
        }

        // Check for @safe annotation in comments
        if trimmed.starts_with("//") {
            if trimmed.contains("@safe") {
                return Ok(true);
            }
        } else if trimmed.starts_with("/*") {
            // Check multi-line comment for @safe
            if line.contains("@safe") {
                return Ok(true);
            }
        } else if !trimmed.starts_with("#") {
            // Found actual code (not preprocessor), stop looking
            break;
        }
    }

    Ok(false) // No @safe annotation found
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn create_temp_cpp_file(content: &str) -> NamedTempFile {
        let mut file = NamedTempFile::with_suffix(".cpp").unwrap();
        file.write_all(content.as_bytes()).unwrap();
        file.flush().unwrap();
        file
    }

    #[allow(dead_code)]
    fn is_libclang_available() -> bool {
        // Try to initialize Clang to check if libclang is available
        // Note: This might fail if another test already initialized Clang
        true // Assume it's available and let individual tests handle errors
    }

    #[test]
    fn test_parse_simple_function() {
        let code = r#"
        void test_function() {
            int x = 42;
        }
        "#;

        let temp_file = create_temp_cpp_file(code);
        let result = parse_cpp_file(temp_file.path());

        match result {
            Ok(ast) => {
                assert_eq!(ast.functions.len(), 1);
                assert_eq!(ast.functions[0].name, "test_function");
            }
            Err(e) if e.contains("already exists") => {
                // Skip if Clang is already initialized by another test
                eprintln!("Skipping test: Clang already initialized by another test");
            }
            Err(e) if e.contains("Failed to initialize Clang") => {
                // Skip if libclang is not available
                eprintln!("Skipping test: libclang not available");
            }
            Err(e) => {
                panic!("Unexpected error: {}", e);
            }
        }
    }

    #[test]
    fn test_parse_function_with_parameters() {
        let code = r#"
        int add(int a, int b) {
            return a + b;
        }
        "#;

        let temp_file = create_temp_cpp_file(code);
        let result = parse_cpp_file(temp_file.path());

        match result {
            Ok(ast) => {
                assert_eq!(ast.functions.len(), 1);
                assert_eq!(ast.functions[0].name, "add");
                assert_eq!(ast.functions[0].parameters.len(), 2);
            }
            Err(e) if e.contains("already exists") => {
                eprintln!("Skipping test: Clang already initialized by another test");
            }
            Err(e) if e.contains("Failed to initialize Clang") => {
                eprintln!("Skipping test: libclang not available");
            }
            Err(e) => {
                panic!("Unexpected error: {}", e);
            }
        }
    }

    #[test]
    fn test_parse_global_variable() {
        let code = r#"
        int global_var = 100;
        
        void func() {}
        "#;

        let temp_file = create_temp_cpp_file(code);
        let result = parse_cpp_file(temp_file.path());

        match result {
            Ok(ast) => {
                assert_eq!(ast.global_variables.len(), 1);
                assert_eq!(ast.global_variables[0].name, "global_var");
            }
            Err(e) if e.contains("already exists") => {
                eprintln!("Skipping test: Clang already initialized by another test");
            }
            Err(e) if e.contains("Failed to initialize Clang") => {
                eprintln!("Skipping test: libclang not available");
            }
            Err(e) => {
                panic!("Unexpected error: {}", e);
            }
        }
    }

    #[test]
    fn test_parse_invalid_file() {
        let temp_dir = tempfile::tempdir().unwrap();
        let invalid_path = temp_dir.path().join("nonexistent.cpp");

        let result = parse_cpp_file(&invalid_path);
        assert!(result.is_err());
    }

    #[test]
    fn test_filter_module_args_for_recovery_std_only() {
        let args = vec![
            "-std=gnu++23".to_string(),
            "-x".to_string(),
            "c++-module".to_string(),
            "-fmodule-file=std=/tmp/std.pcm".to_string(),
            "-fmodule-file=rrr=/tmp/rrr.pcm".to_string(),
            "-fmodule-map-file=/tmp/map.modulemap".to_string(),
            "-fmodules".to_string(),
        ];

        let filtered = filter_module_args_for_recovery(&args, true);
        assert!(filtered.contains(&"-std=gnu++23".to_string()));
        assert!(filtered.contains(&"-fmodule-file=std=/tmp/std.pcm".to_string()));
        assert!(!filtered.iter().any(|arg| arg.contains("rrr.pcm")));
        assert!(!filtered.iter().any(|arg| arg == "-x"));
        assert!(!filtered.iter().any(|arg| arg == "c++-module"));
        assert!(!filtered.iter().any(|arg| arg == "-fmodules"));
    }

    #[test]
    fn test_filter_module_args_for_recovery_no_modules() {
        let args = vec![
            "-std=gnu++23".to_string(),
            "-fmodule-file=std=/tmp/std.pcm".to_string(),
            "-fmodule-file=rrr=/tmp/rrr.pcm".to_string(),
        ];

        let filtered = filter_module_args_for_recovery(&args, false);
        assert_eq!(filtered, vec!["-std=gnu++23".to_string()]);
    }
}
