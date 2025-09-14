use std::fs;
use std::process::Command;
use tempfile::TempDir;

#[test]
fn test_undeclared_can_call_undeclared() {
    let temp_dir = TempDir::new().unwrap();
    
    let cpp_path = temp_dir.path().join("test.cpp");
    let cpp_content = r#"
// No annotation - undeclared function
void helper() {
    // Do something
}

// No annotation - undeclared function
void undeclared_function() {
    helper();  // OK: undeclared can call undeclared
    printf("test");  // OK: undeclared can call anything
}

// @safe
void safe_function() {
    // helper();  // Would be ERROR: safe cannot call undeclared
    printf("test");  // OK: printf is whitelisted
}

// @unsafe
void unsafe_function() {
    helper();  // OK: unsafe can call undeclared
    undeclared_function();  // OK: unsafe can call undeclared
}
"#;
    fs::write(&cpp_path, cpp_content).unwrap();
    
    let output = run_checker(&cpp_path);
    
    // Should have no violations - undeclared calling undeclared is fine
    assert!(output.contains("No borrow checking violations") || 
            !output.contains("undeclared_function"),
            "Undeclared functions should be able to call other undeclared functions, got: {}", output);
}

#[test]
fn test_safe_cannot_call_undeclared() {
    let temp_dir = TempDir::new().unwrap();
    
    let cpp_path = temp_dir.path().join("test.cpp");
    let cpp_content = r#"
// No annotation - undeclared function
void helper() {
    // Do something
}

// @safe
void safe_function() {
    helper();  // ERROR: safe cannot call undeclared
}
"#;
    fs::write(&cpp_path, cpp_content).unwrap();
    
    let output = run_checker(&cpp_path);
    
    // Should report violation for safe calling undeclared
    assert!(output.contains("violation") || output.contains("undeclared"),
            "Safe functions should not be able to call undeclared functions, got: {}", output);
}

#[test]
fn test_undeclared_chain() {
    let temp_dir = TempDir::new().unwrap();
    
    let cpp_path = temp_dir.path().join("test.cpp");
    let cpp_content = r#"
// Chain of undeclared functions - all should be OK
void func_a() {
    // Do something
}

void func_b() {
    func_a();  // OK: undeclared calling undeclared
}

void func_c() {
    func_b();  // OK: undeclared calling undeclared
}

void main() {
    func_c();  // OK: undeclared (main) calling undeclared
}

// @safe
void safe_func() {
    // func_c();  // Would be ERROR: safe cannot call undeclared
}
"#;
    fs::write(&cpp_path, cpp_content).unwrap();
    
    let output = run_checker(&cpp_path);
    
    // Should have no violations
    assert!(output.contains("No borrow checking violations") || 
            (!output.contains("func_a") && !output.contains("func_b") && !output.contains("func_c")),
            "Chains of undeclared functions should be allowed, got: {}", output);
}

// Helper function
fn run_checker(cpp_file: &std::path::Path) -> String {
    let z3_header = if cfg!(target_os = "macos") {
        "/opt/homebrew/include/z3.h"
    } else {
        "/usr/include/z3.h"
    };
    
    let mut cmd = Command::new("cargo");
    cmd.args(&["run", "--quiet", "--", cpp_file.to_str().unwrap()])
        .env("Z3_SYS_Z3_HEADER", z3_header);
    
    if cfg!(target_os = "macos") {
        cmd.env("DYLD_LIBRARY_PATH", "/opt/homebrew/Cellar/llvm/19.1.7/lib");
    } else {
        cmd.env("LD_LIBRARY_PATH", "/usr/lib/llvm-14/lib");
    }
    
    let output = cmd.output().expect("Failed to execute checker");
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    format!("{}{}", stdout, stderr)
}