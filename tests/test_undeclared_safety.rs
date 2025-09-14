use std::fs;
use std::process::Command;
use tempfile::TempDir;

#[test]
fn test_safe_cannot_call_undeclared() {
    let temp_dir = TempDir::new().unwrap();
    
    // Create a C++ file with a safe function calling an undeclared function
    let cpp_path = temp_dir.path().join("test.cpp");
    let cpp_content = r#"
// Function with no safety annotation - undeclared
void undeclared_function() {
    int x = 42;
}

// @safe
void safe_function() {
    undeclared_function();  // ERROR: safe cannot call undeclared
}
"#;
    fs::write(&cpp_path, cpp_content).unwrap();
    
    // Run the checker
    let output = run_checker(&cpp_path);
    
    // Debug: print the output to see what we got
    eprintln!("Output from checker: {}", output);
    
    // Should report an error about calling undeclared function
    assert!(output.contains("undeclared"), 
            "Expected error about undeclared function, got: {}", output);
    assert!(output.contains("must be explicitly marked"),
            "Expected message about explicit marking, got: {}", output);
}

#[test]
fn test_safe_can_call_safe() {
    let temp_dir = TempDir::new().unwrap();
    
    let cpp_path = temp_dir.path().join("test.cpp");
    let cpp_content = r#"
// @safe
void safe_helper() {
    int x = 42;
}

// @safe
void safe_function() {
    safe_helper();  // OK: safe can call safe
}
"#;
    fs::write(&cpp_path, cpp_content).unwrap();
    
    let output = run_checker(&cpp_path);
    
    // Should not report errors about function calls
    assert!(!output.contains("safe_helper") || output.contains("No borrow checking violations"),
            "Should allow safe to call safe, got: {}", output);
}

#[test]
fn test_safe_cannot_call_unsafe() {
    let temp_dir = TempDir::new().unwrap();
    
    let cpp_path = temp_dir.path().join("test.cpp");
    let cpp_content = r#"
// @unsafe
void unsafe_function() {
    int* ptr = nullptr;
    *ptr = 42;
}

// @safe
void safe_function() {
    unsafe_function();  // ERROR: safe cannot call unsafe (without explicit marking)
}
"#;
    fs::write(&cpp_path, cpp_content).unwrap();
    
    let output = run_checker(&cpp_path);
    
    // Should report an error about calling unsafe function
    assert!(output.contains("unsafe_function") && output.contains("unsafe"),
            "Expected error about unsafe function call, got: {}", output);
}

#[test]
fn test_undeclared_can_call_anything() {
    let temp_dir = TempDir::new().unwrap();
    
    let cpp_path = temp_dir.path().join("test.cpp");
    let cpp_content = r#"
// @safe
void safe_function() {
    int x = 42;
}

// @unsafe
void unsafe_function() {
    int* ptr = nullptr;
}

void other_undeclared() {
    int y = 10;
}

// No annotation - undeclared
void undeclared_function() {
    safe_function();      // OK: undeclared can call safe
    unsafe_function();    // OK: undeclared can call unsafe
    other_undeclared();   // OK: undeclared can call undeclared
    
    // Also, pointer operations are allowed in undeclared (treated as unsafe for checking)
    int x = 5;
    int* ptr = &x;
    *ptr = 10;
}
"#;
    fs::write(&cpp_path, cpp_content).unwrap();
    
    let output = run_checker(&cpp_path);
    
    // Should not have errors for undeclared_function
    // (it's not checked for borrow violations since it's not safe)
    assert!(!output.contains("In function 'undeclared_function'"),
            "Undeclared functions should not be checked, got: {}", output);
}

#[test]
fn test_unsafe_can_call_anything() {
    let temp_dir = TempDir::new().unwrap();
    
    let cpp_path = temp_dir.path().join("test.cpp");
    let cpp_content = r#"
// @safe
void safe_function() {
    int x = 42;
}

void undeclared_function() {
    int y = 10;
}

// @unsafe
void unsafe_function() {
    safe_function();       // OK: unsafe can call safe
    undeclared_function(); // OK: unsafe can call undeclared
    
    // Also can do pointer operations
    int* ptr = nullptr;
}
"#;
    fs::write(&cpp_path, cpp_content).unwrap();
    
    let output = run_checker(&cpp_path);
    
    // Should not have errors for unsafe_function
    assert!(!output.contains("In function 'unsafe_function'"),
            "Unsafe functions should not be checked, got: {}", output);
}

// Helper function to run the checker
fn run_checker(cpp_file: &std::path::Path) -> String {
    // Set platform-specific paths
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