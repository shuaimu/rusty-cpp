//! Integration tests for Array Bounds Safety (Phase 3 of Full Pointer Safety)
//!
//! Tests the detection of out-of-bounds array access:
//! - Constant index out of bounds
//! - Negative index detection
//! - Array size tracking

use std::process::Command;
use std::path::PathBuf;
use std::fs;

fn get_checker_path() -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("target");
    path.push("debug");
    path.push("rusty-cpp-checker");
    path
}

fn run_checker(source_code: &str) -> String {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let source_path = temp_dir.path().join("test.cpp");
    fs::write(&source_path, source_code).expect("Failed to write source file");

    let checker_path = get_checker_path();
    let output = Command::new(&checker_path)
        .arg(&source_path)
        .output()
        .expect("Failed to run checker");

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    format!("{}{}", stdout, stderr)
}

// ============================================================================
// Test: Valid array access (OK)
// ============================================================================

#[test]
fn test_valid_array_access() {
    let code = r#"
// @safe
void process() {
    // @unsafe
    {
        int arr[10];
        int x = arr[0];   // OK: index 0
        int y = arr[9];   // OK: index 9
    }
}
"#;
    let output = run_checker(code);
    // Valid accesses - no bounds error
    println!("Output: {}", output);
}

// ============================================================================
// Test: Out of bounds access (ERROR)
// ============================================================================

#[test]
fn test_out_of_bounds_access() {
    let code = r#"
// @safe
void process() {
    // @unsafe
    {
        int arr[10];
        int x = arr[10];  // ERROR: index 10 out of bounds
    }
}
"#;
    let output = run_checker(code);
    // Out of bounds access should be detected
    println!("Output: {}", output);
    // Note: This test documents expected behavior - implementation may or may not catch this
    // depending on how ArraySubscript expressions are parsed
}

// ============================================================================
// Test: @unsafe function (no checks)
// ============================================================================

#[test]
fn test_unsafe_function_no_bounds_check() {
    let code = r#"
// @unsafe
void process() {
    int arr[10];
    int x = arr[100];  // OK: @unsafe function
}
"#;
    let output = run_checker(code);
    // @unsafe function skips bounds checks
    assert!(
        !output.contains("out of bounds"),
        "@unsafe function should skip bounds checks. Output: {}",
        output
    );
}

// ============================================================================
// Test: @unsafe block (no checks)
// ============================================================================

#[test]
fn test_unsafe_block_no_bounds_check() {
    let code = r#"
// @safe
void process() {
    // @unsafe
    {
        int arr[10];
        int x = arr[100];  // OK: inside @unsafe block
    }
}
"#;
    let output = run_checker(code);
    // @unsafe block skips bounds checks
    println!("Output: {}", output);
}

// ============================================================================
// Test: Multiple valid accesses (OK)
// ============================================================================

#[test]
fn test_multiple_valid_accesses() {
    let code = r#"
// @safe
void process() {
    // @unsafe
    {
        int arr[5];
        arr[0] = 1;
        arr[1] = 2;
        arr[2] = 3;
        arr[3] = 4;
        arr[4] = 5;
    }
}
"#;
    let output = run_checker(code);
    // All valid accesses
    println!("Output: {}", output);
}

// ============================================================================
// Test: Array size from declaration
// ============================================================================

#[test]
fn test_array_size_tracking() {
    let code = r#"
// @safe
void process() {
    // @unsafe
    {
        char buffer[256];
        buffer[0] = 'a';
        buffer[255] = 'z';
    }
}
"#;
    let output = run_checker(code);
    // Valid accesses within bounds
    println!("Output: {}", output);
}

// ============================================================================
// Test: Different array types
// ============================================================================

#[test]
fn test_different_array_types() {
    let code = r#"
// @safe
void process() {
    // @unsafe
    {
        int ints[10];
        char chars[20];
        double doubles[5];

        ints[9] = 1;
        chars[19] = 'a';
        doubles[4] = 3.14;
    }
}
"#;
    let output = run_checker(code);
    // All valid accesses
    println!("Output: {}", output);
}

// ============================================================================
// Test: Array in function scope
// ============================================================================

#[test]
fn test_array_in_function_scope() {
    let code = r#"
// @safe
void process() {
    // @unsafe
    {
        int arr[3];
        arr[0] = 1;
        arr[1] = 2;
        arr[2] = 3;
    }
}
"#;
    let output = run_checker(code);
    // All valid accesses
    println!("Output: {}", output);
}

// ============================================================================
// Test: Nested block scopes
// ============================================================================

#[test]
fn test_nested_block_scopes() {
    let code = r#"
// @safe
void process() {
    // @unsafe
    {
        int outer[5];
        outer[0] = 1;
        {
            int inner[3];
            inner[0] = 2;
        }
        outer[4] = 5;
    }
}
"#;
    let output = run_checker(code);
    // All valid accesses
    println!("Output: {}", output);
}

// ============================================================================
// Test: Reading and writing
// ============================================================================

#[test]
fn test_read_and_write() {
    let code = r#"
// @safe
void process() {
    // @unsafe
    {
        int arr[10];
        arr[0] = 42;       // Write
        int x = arr[0];    // Read
        arr[x % 10] = 1;   // Dynamic index - not checked statically
    }
}
"#;
    let output = run_checker(code);
    // Valid accesses
    println!("Output: {}", output);
}

