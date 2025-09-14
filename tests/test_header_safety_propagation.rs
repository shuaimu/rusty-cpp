use std::fs;
use std::process::Command;
use tempfile::TempDir;

#[test]
fn test_header_safety_propagation() {
    let temp_dir = TempDir::new().unwrap();
    
    // Create a header file with @safe annotation
    let header_path = temp_dir.path().join("test.h");
    let header_content = r#"
#ifndef TEST_H
#define TEST_H

// @safe
void safe_function_in_header();

// @unsafe
void unsafe_function_in_header();

// No annotation - should be unsafe by default
void no_annotation_function();

class TestClass {
public:
    // @safe
    void safe_method();
    
    // @unsafe
    void unsafe_method();
};

#endif
"#;
    fs::write(&header_path, header_content).unwrap();
    
    // Create implementation file that includes the header
    let cpp_path = temp_dir.path().join("test.cpp");
    let cpp_content = r#"
#include "test.h"

void safe_function_in_header() {
    int x = 5;
    int* ptr = &x;  // This should be an error - raw pointer in safe function
    *ptr = 10;
}

void unsafe_function_in_header() {
    int x = 5;
    int* ptr = &x;  // This should be OK - unsafe function
    *ptr = 10;
}

void no_annotation_function() {
    int x = 5;
    int* ptr = &x;  // This should be OK - no annotation means unsafe
    *ptr = 10;
}

void TestClass::safe_method() {
    int x = 5;
    int* ptr = &x;  // This should be an error - safe method from header
    *ptr = 10;
}

void TestClass::unsafe_method() {
    int x = 5;
    int* ptr = &x;  // This should be OK - unsafe method
    *ptr = 10;
}
"#;
    fs::write(&cpp_path, cpp_content).unwrap();
    
    // Run the checker with the include path
    let output = Command::new("cargo")
        .args(&["run", "--", 
                &cpp_path.to_string_lossy(),
                "-I", &temp_dir.path().to_string_lossy()])
        .output()
        .expect("Failed to execute rusty-cpp");
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    
    eprintln!("STDOUT:\n{}", stdout);
    eprintln!("STDERR:\n{}", stderr);
    
    // Check that safe functions from header are properly checked
    assert!(stdout.contains("violation") || stderr.contains("violation"), 
            "Expected violations for pointer operations in safe functions");
    
    // Verify specific functions are checked
    assert!(stderr.contains("safe_function_in_header") || stdout.contains("safe_function_in_header"),
            "Expected safe_function_in_header to be checked");
    assert!(stderr.contains("safe_method") || stdout.contains("safe_method"),
            "Expected safe_method to be checked");
}

#[test]
fn test_header_safety_override_by_source() {
    let temp_dir = TempDir::new().unwrap();
    
    // Create a header file with @safe annotation
    let header_path = temp_dir.path().join("override.h");
    let header_content = r#"
#ifndef OVERRIDE_H
#define OVERRIDE_H

// @safe
void function_safe_in_header();

#endif
"#;
    fs::write(&header_path, header_content).unwrap();
    
    // Create implementation file that overrides with @unsafe
    let cpp_path = temp_dir.path().join("override.cpp");
    let cpp_content = r#"
#include "override.h"

// @unsafe
void function_safe_in_header() {
    // Source file annotation should override header
    int x = 5;
    int* ptr = &x;  // This should be OK - overridden as unsafe
    *ptr = 10;
}
"#;
    fs::write(&cpp_path, cpp_content).unwrap();
    
    // Run the checker
    let output = Command::new("cargo")
        .args(&["run", "--", 
                &cpp_path.to_string_lossy(),
                "-I", &temp_dir.path().to_string_lossy()])
        .output()
        .expect("Failed to execute rusty-cpp");
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    
    eprintln!("STDOUT:\n{}", stdout);
    eprintln!("STDERR:\n{}", stderr);
    
    // Should not have violations since source file overrides header
    assert!(!stdout.contains("function_safe_in_header") || 
            stdout.contains("no violations found"),
            "Source file @unsafe should override header @safe");
}

#[test]
fn test_namespace_safety_in_header() {
    let temp_dir = TempDir::new().unwrap();
    
    // Create a header with namespace-level safety
    let header_path = temp_dir.path().join("namespace.h");
    let header_content = r#"
#ifndef NAMESPACE_H
#define NAMESPACE_H

// @safe
namespace SafeNamespace {
    void func_in_safe_namespace();
    void another_func();
}

namespace UnsafeNamespace {
    void func_in_unsafe_namespace();
}

#endif
"#;
    fs::write(&header_path, header_content).unwrap();
    
    // Create implementation
    let cpp_path = temp_dir.path().join("namespace.cpp");
    let cpp_content = r#"
#include "namespace.h"

namespace SafeNamespace {
    void func_in_safe_namespace() {
        int x = 5;
        int* ptr = &x;  // Should be error - in safe namespace
        *ptr = 10;
    }
    
    void another_func() {
        int x = 5;
        int* ptr = &x;  // Should be error - in safe namespace
        *ptr = 10;
    }
}

namespace UnsafeNamespace {
    void func_in_unsafe_namespace() {
        int x = 5;
        int* ptr = &x;  // Should be OK - unsafe namespace
        *ptr = 10;
    }
}
"#;
    fs::write(&cpp_path, cpp_content).unwrap();
    
    // Run the checker
    let output = Command::new("cargo")
        .args(&["run", "--", 
                &cpp_path.to_string_lossy(),
                "-I", &temp_dir.path().to_string_lossy()])
        .output()
        .expect("Failed to execute rusty-cpp");
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    
    eprintln!("STDOUT:\n{}", stdout);
    eprintln!("STDERR:\n{}", stderr);
    
    // Functions in safe namespace should be checked
    assert!(stdout.contains("violation") || stderr.contains("violation"),
            "Expected violations for functions in safe namespace");
}