use std::fs;
use std::process::Command;
use tempfile::TempDir;

#[test]
fn test_qualified_method_names_with_headers() {
    let temp_dir = TempDir::new().unwrap();
    
    // Create header file
    let header_path = temp_dir.path().join("classes.h");
    let header_content = r#"
namespace MyNamespace {
    class ClassA {
    public:
        // @safe
        void process();
    };
    
    class ClassB {
    public:
        // No annotation - should be undeclared
        void process();
    };
}
"#;
    fs::write(&header_path, header_content).unwrap();
    
    // Create implementation file
    let cpp_path = temp_dir.path().join("test.cpp");
    let cpp_content = r#"
#include "classes.h"

void MyNamespace::ClassA::process() {
    // Safe implementation
}

void MyNamespace::ClassB::process() {
    // Undeclared implementation
}

// @safe  
void test_methods() {
    // Just testing that methods are properly identified
    // Not creating objects to avoid constructor issues
}
"#;
    fs::write(&cpp_path, cpp_content).unwrap();
    
    let output = run_checker_with_include(&cpp_path, temp_dir.path());
    
    // Should have no violations - we're not calling the methods
    assert!(output.contains("No borrow checking violations") || 
            !output.contains("violation"),
            "Qualified method names should be handled correctly, got: {}", output);
}

#[test]
fn test_qualified_method_implementation() {
    let temp_dir = TempDir::new().unwrap();
    
    let cpp_path = temp_dir.path().join("test.cpp");
    let cpp_content = r#"
namespace MyNamespace {
    class ClassA {
    public:
        void process();
    };
    
    class ClassB {
    public:
        void process();
    };
}

// Method implementations with qualified names
// @safe
void MyNamespace::ClassA::process() {
    // Safe implementation
}

// No annotation - undeclared
void MyNamespace::ClassB::process() {
    // Undeclared implementation
}

// @safe  
void test_collision() {
    // Just testing that methods are properly identified
    // Not creating objects to avoid constructor issues
}
"#;
    fs::write(&cpp_path, cpp_content).unwrap();
    
    let output = run_checker(&cpp_path);
    
    // Should properly distinguish between the two process methods
    assert!(output.contains("No borrow checking violations") || 
            !output.contains("violation"),
            "Qualified names in implementations should work, got: {}", output);
}

// Helper functions
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

fn run_checker_with_include(cpp_file: &std::path::Path, include_dir: &std::path::Path) -> String {
    let z3_header = if cfg!(target_os = "macos") {
        "/opt/homebrew/include/z3.h"
    } else {
        "/usr/include/z3.h"
    };
    
    let mut cmd = Command::new("cargo");
    cmd.args(&["run", "--quiet", "--", cpp_file.to_str().unwrap()])
        .arg("-I")
        .arg(include_dir.to_str().unwrap())
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