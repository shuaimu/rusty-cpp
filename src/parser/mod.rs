use clang::{Clang, Entity, EntityKind, Index};
use std::path::Path;

pub mod ast_visitor;
pub mod annotations;
pub mod header_cache;
pub mod safety_annotations;
pub mod type_annotations;
pub mod external_annotations;

pub use ast_visitor::{CppAst, Function, Statement, Expression, MethodQualifier};
pub use header_cache::HeaderCache;
#[allow(unused_imports)]
pub use ast_visitor::{Variable, SourceLocation};

use std::fs;
use std::io::{BufRead, BufReader};

#[allow(dead_code)]
pub fn parse_cpp_file(path: &Path) -> Result<CppAst, String> {
    parse_cpp_file_with_includes(path, &[])
}

#[allow(dead_code)]
pub fn parse_cpp_file_with_includes(path: &Path, include_paths: &[std::path::PathBuf]) -> Result<CppAst, String> {
    parse_cpp_file_with_includes_and_defines(path, include_paths, &[])
}

pub fn parse_cpp_file_with_includes_and_defines(path: &Path, include_paths: &[std::path::PathBuf], defines: &[String]) -> Result<CppAst, String> {
    // Initialize Clang
    let clang = Clang::new()
        .map_err(|e| format!("Failed to initialize Clang: {:?}", e))?;
    
    let index = Index::new(&clang, false, false);
    
    // Build arguments with include paths and defines
    let mut args = vec![
        "-std=c++17".to_string(), 
        "-xc++".to_string(),
        // Add flags to make parsing more lenient
        "-fno-delayed-template-parsing".to_string(),
        "-fparse-all-comments".to_string(),
        // Suppress certain errors that don't affect borrow checking
        "-Wno-everything".to_string(),
        // Don't fail on missing includes
        "-Wno-error".to_string(),
    ];
    
    // Add include paths
    for include_path in include_paths {
        args.push(format!("-I{}", include_path.display()));
    }
    
    // Add preprocessor definitions
    for define in defines {
        args.push(format!("-D{}", define));
    }
    
    // Parse the translation unit with skip function bodies option for better error recovery
    let tu = index
        .parser(path)
        .arguments(&args.iter().map(|s| s.as_str()).collect::<Vec<_>>())
        .detailed_preprocessing_record(true)
        .skip_function_bodies(false)  // We need function bodies for analysis
        .incomplete(true)  // Allow incomplete translation units
        .parse()
        .map_err(|e| format!("Failed to parse file: {:?}", e))?;
    
    // Check for diagnostics but only fail on fatal errors
    let diagnostics = tu.get_diagnostics();
    let mut has_fatal = false;
    if !diagnostics.is_empty() {
        for diag in &diagnostics {
            // Only fail on fatal errors, ignore regular errors
            if diag.get_severity() >= clang::diagnostic::Severity::Fatal {
                has_fatal = true;
                eprintln!("Fatal error: {}", diag.get_text());
            } else if diag.get_severity() >= clang::diagnostic::Severity::Error {
                // Log errors but don't fail
                eprintln!("Warning (suppressed error): {}", diag.get_text());
            }
        }
    }
    
    if has_fatal {
        return Err("Fatal parsing errors encountered".to_string());
    }
    
    // Visit the AST
    let mut ast = CppAst::new();
    let root = tu.get_entity();
    let mut visited_files = std::collections::HashSet::new();
    visit_entity(&root, &mut ast, &mut visited_files);
    
    Ok(ast)
}

fn visit_entity(entity: &Entity, ast: &mut CppAst, visited_files: &mut std::collections::HashSet<String>) {
    // Check location and filter files
    if let Some(location) = entity.get_location() {
        if let Some(file) = location.get_file_location().file {
            let path = file.get_path();
            let path_str = path.to_string_lossy().to_string();
            
            // Skip system headers and standard library
            if path_str.starts_with("/usr/include") || 
               path_str.starts_with("/usr/lib") ||
               path_str.starts_with("/usr/local/include") ||
               path_str.starts_with("/System") ||
               path_str.starts_with("/Library/Developer") ||
               path_str.starts_with("/Applications/Xcode") ||
               path_str.starts_with("/opt/") ||
               path_str.contains("/bits/") ||
               path_str.contains("/c++/") ||
               path_str.contains("/__") ||
               path_str.contains("/stdlib") ||
               path_str.contains("/sys/") ||
               path_str.contains("/linux/") ||
               path_str.contains("/asm/") ||
               path_str.contains("/gnu/") {
                return; // Skip system headers
            }
            
            // Skip third-party libraries (except rustycpp for testing)
            if path_str.contains("/third-party/") && 
               !path_str.contains("/third-party/rustycpp") {
                // But allow certain important third-party headers
                if !path_str.contains("/third-party/erpc/src/rpc.h") {
                    return;
                }
            }
            
            // Track processed files to avoid infinite recursion
            if !location.is_in_main_file() {
                if visited_files.contains(&path_str) {
                    return; // Already processed this file
                }
                visited_files.insert(path_str.clone());
                
                // Limit the number of files we process
                if visited_files.len() > 100 {
                    return; // Too many files, stop processing
                }
            }
        }
    }
    
    match entity.get_kind() {
        EntityKind::FunctionDecl | EntityKind::Method => {
            if entity.is_definition() {
                let func = ast_visitor::extract_function(entity);
                ast.functions.push(func);
            }
        }
        EntityKind::VarDecl => {
            let var = ast_visitor::extract_variable(entity);
            ast.global_variables.push(var);
        }
        _ => {}
    }
    
    // Recursively visit children
    for child in entity.get_children() {
        visit_entity(&child, ast, visited_files);
    }
}

/// Check if the file has @safe annotation at the beginning
#[allow(dead_code)]
pub fn check_file_safety_annotation(path: &Path) -> Result<bool, String> {
    let file = fs::File::open(path)
        .map_err(|e| format!("Failed to open file for safety check: {}", e))?;
    
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
    use tempfile::NamedTempFile;
    use std::io::Write;

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
        true  // Assume it's available and let individual tests handle errors
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
}