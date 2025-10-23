use std::fs;
use std::process::Command;
use tempfile::TempDir;

#[test]
fn test_safe_cannot_use_std_vector() {
    let temp_dir = TempDir::new().unwrap();
    
    let cpp_path = temp_dir.path().join("test.cpp");
    let cpp_content = r#"
#include <vector>

// @safe
void safe_function() {
    std::vector<int> vec;
    vec.push_back(42);  // ERROR: undeclared
}
"#;
    fs::write(&cpp_path, cpp_content).unwrap();
    
    let output = run_checker(&cpp_path);
    
    // Should report violations for using undeclared STL functions
    assert!(output.contains("violation") || output.contains("undeclared"),
            "Expected error about undeclared STL functions, got: {}", output);
}

#[test]
fn test_safe_cannot_use_std_string() {
    let temp_dir = TempDir::new().unwrap();
    
    let cpp_path = temp_dir.path().join("test.cpp");
    let cpp_content = r#"
#include <string>

// @safe
void safe_function() {
    std::string str = "hello";
    str.append(" world");  // ERROR: undeclared
}
"#;
    fs::write(&cpp_path, cpp_content).unwrap();
    
    let output = run_checker(&cpp_path);
    
    // Should report violations
    assert!(output.contains("violation") || output.contains("undeclared"),
            "Expected error about undeclared STL functions, got: {}", output);
}

#[test]
fn test_unsafe_can_use_std() {
    let temp_dir = TempDir::new().unwrap();
    
    let cpp_path = temp_dir.path().join("test.cpp");
    let cpp_content = r#"
#include <vector>
#include <string>

// @unsafe
void unsafe_function() {
    std::vector<int> vec;
    vec.push_back(42);  // OK in unsafe
    
    std::string str = "hello";
    str.append(" world");  // OK in unsafe
}
"#;
    fs::write(&cpp_path, cpp_content).unwrap();
    
    let output = run_checker(&cpp_path);
    
    // Should not have violations for unsafe function
    assert!(output.contains("no violations found") || 
            !output.contains("unsafe_function"),
            "Unsafe functions should be able to use STL, got: {}", output);
}

#[test]
fn test_undeclared_can_use_std() {
    let temp_dir = TempDir::new().unwrap();
    
    let cpp_path = temp_dir.path().join("test.cpp");
    let cpp_content = r#"
#include <vector>

// No annotation - undeclared
void undeclared_function() {
    std::vector<int> vec;
    vec.push_back(42);  // OK in undeclared
}
"#;
    fs::write(&cpp_path, cpp_content).unwrap();
    
    let output = run_checker(&cpp_path);
    
    // Should not check undeclared functions
    assert!(output.contains("no violations found") || 
            !output.contains("undeclared_function"),
            "Undeclared functions should not be checked, got: {}", output);
}

#[test]
fn test_whitelisted_std_functions() {
    let temp_dir = TempDir::new().unwrap();
    
    let cpp_path = temp_dir.path().join("test.cpp");
    let cpp_content = r#"
#include <cstdio>
#include <iostream>

// @safe
void safe_function() {
    // These are whitelisted
    printf("Hello\n");  // OK: whitelisted
    std::cout << "World\n";  // OK: cout is whitelisted
}
"#;
    fs::write(&cpp_path, cpp_content).unwrap();
    
    let output = run_checker(&cpp_path);
    
    // Should not have violations for whitelisted functions
    assert!(output.contains("no violations found") || output.contains("✓"),
            "Whitelisted functions should be allowed, got: {}", output);
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
    
    if !cfg!(target_os = "macos") {
        cmd.env("LD_LIBRARY_PATH", "/usr/lib/llvm-14/lib");
    }
    
    let output = cmd.output().expect("Failed to execute checker");
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    format!("{}{}", stdout, stderr)
}