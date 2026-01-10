use clang::{Clang, Entity, EntityKind, Index};
use std::path::Path;

pub mod ast_visitor;
pub mod annotations;
pub mod header_cache;
pub mod safety_annotations;
pub mod external_annotations;

pub use ast_visitor::{CppAst, Function, Statement, Expression, MethodQualifier, MoveKind, CastKind};
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
    let main_file_path = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    visit_entity(&root, &mut ast, &main_file_path);

    Ok(ast)
}

fn visit_entity(entity: &Entity, ast: &mut CppAst, main_file: &Path) {
    use crate::debug_println;
    // Only debug function and class-related entities
    if matches!(entity.get_kind(), EntityKind::FunctionDecl | EntityKind::Method | EntityKind::FunctionTemplate | EntityKind::ClassTemplate) {
        // Check if this is a template specialization
        let template_kind = entity.get_template_kind();
        let template_args = entity.get_template_arguments();
        debug_println!("DEBUG PARSE: Visiting entity: kind={:?}, name={:?}, is_definition={}, template_kind={:?}, has_template_args={}",
            entity.get_kind(), entity.get_name(), entity.is_definition(), template_kind, template_args.is_some());
    }

    // Extract entities from all files (main file and headers)
    // The analysis phase will distinguish between system headers and user code
    // System headers: track for safety status, but skip borrow checking
    // User code: full borrow checking and safety analysis
    let _main_file = main_file; // Keep parameter for future use

    match entity.get_kind() {
        EntityKind::FunctionDecl | EntityKind::Method => {
            debug_println!("DEBUG PARSE: Found FunctionDecl: name={:?}, is_definition={}, kind={:?}",
                entity.get_name(), entity.is_definition(), entity.get_kind());
            // Extract all function definitions (from main file and headers)
            if entity.is_definition() {
                let func = ast_visitor::extract_function(entity);
                ast.functions.push(func);
            }
        }
        EntityKind::FunctionTemplate => {
            // Template free functions: extract the template declaration to analyze with generic types
            // We don't need instantiations - our borrow/move checking works on generic types!
            debug_println!("DEBUG PARSE: Found FunctionTemplate: {:?}, is_definition={}",
                entity.get_name(), entity.is_definition());

            // The FunctionTemplateDecl IS the function entity in LibClang
            // Its children are: TemplateTypeParameter, ParmDecl, CompoundStmt
            // We extract the function directly from this entity
            if entity.is_definition() {
                debug_println!("DEBUG PARSE: Extracting template function from FunctionTemplate entity");
                let func = ast_visitor::extract_function(entity);
                debug_println!("DEBUG PARSE: Extracted template function: {}", func.name);
                ast.functions.push(func);
            }
        }
        EntityKind::ClassTemplate => {
            // Phase 3: Template classes
            debug_println!("DEBUG PARSE: Found ClassTemplate: {:?}, is_definition={}",
                entity.get_name(), entity.is_definition());

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
            debug_println!("DEBUG PARSE: Found ClassDecl/StructDecl: {:?}, is_definition={}",
                entity.get_name(), entity.is_definition());

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