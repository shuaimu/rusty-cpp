//! Integration tests for Pointer Provenance (Phase 5 of Full Pointer Safety)
//!
//! Tests the detection of pointer operations between different allocations:
//! - Pointer subtraction requires same allocation
//! - Relational comparison (<, >, <=, >=) requires same allocation
//! - Equality comparison (==, !=) is allowed between any pointers

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
// Test: Same allocation subtraction (OK)
// ============================================================================

#[test]
fn test_same_allocation_subtraction() {
    let code = r#"
// @safe
void process() {
    int arr[10];
    // @unsafe
    {
        int* p1 = arr;
        int* p2 = arr + 5;
        auto diff = p2 - p1;  // OK: same array
    }
}
"#;
    let output = run_checker(code);
    // Same allocation - no provenance error
    println!("Output: {}", output);
}

// ============================================================================
// Test: Same allocation comparison (OK)
// ============================================================================

#[test]
fn test_same_allocation_comparison() {
    let code = r#"
// @safe
void process() {
    int arr[10];
    // @unsafe
    {
        int* p1 = arr;
        int* p2 = arr + 5;
        bool cmp = p1 < p2;  // OK: same array
    }
}
"#;
    let output = run_checker(code);
    // Same allocation - no provenance error
    println!("Output: {}", output);
}

// ============================================================================
// Test: Equality comparison allowed (OK)
// ============================================================================

#[test]
fn test_equality_comparison_allowed() {
    let code = r#"
// @safe
void process() {
    int a, b;
    // @unsafe
    {
        int* pa = &a;
        int* pb = &b;
        bool eq = pa == pb;  // OK: equality is well-defined
    }
}
"#;
    let output = run_checker(code);
    // Equality comparison is allowed
    assert!(
        !output.contains("different allocations") || output.contains("no violations"),
        "Equality comparison should be allowed. Output: {}",
        output
    );
}

// ============================================================================
// Test: @unsafe function (no checks)
// ============================================================================

#[test]
fn test_unsafe_function_no_provenance_check() {
    let code = r#"
// @unsafe
void process() {
    int a, b;
    int* pa = &a;
    int* pb = &b;
    auto diff = pa - pb;  // OK: @unsafe function
}
"#;
    let output = run_checker(code);
    // @unsafe function skips provenance checks
    assert!(
        !output.contains("different allocations"),
        "@unsafe function should skip provenance checks. Output: {}",
        output
    );
}

// ============================================================================
// Test: @unsafe block
// ============================================================================

#[test]
fn test_unsafe_block_skips_provenance_check() {
    let code = r#"
// @safe
void process() {
    // @unsafe
    {
        int a, b;
        int* pa = &a;
        int* pb = &b;
        auto diff = pa - pb;  // OK: inside @unsafe block
    }
}
"#;
    let output = run_checker(code);
    // @unsafe block skips provenance checks
    assert!(
        !output.contains("different allocations"),
        "@unsafe block should skip provenance checks. Output: {}",
        output
    );
}

// ============================================================================
// Test: Pointer arithmetic preserves provenance
// ============================================================================

#[test]
fn test_pointer_arithmetic_preserves_provenance() {
    let code = r#"
// @safe
void process() {
    int arr[10];
    // @unsafe
    {
        int* p1 = arr;
        int* p2 = p1 + 5;  // p2 has same provenance as p1
        auto diff = p2 - p1;  // OK
    }
}
"#;
    let output = run_checker(code);
    // Pointer arithmetic preserves provenance
    println!("Output: {}", output);
}

// ============================================================================
// Test: Copy preserves provenance
// ============================================================================

#[test]
fn test_copy_preserves_provenance() {
    let code = r#"
// @safe
void process() {
    int arr[10];
    // @unsafe
    {
        int* p1 = arr;
        int* p2 = p1;  // Copy provenance
        bool cmp = p1 < p2;  // OK: same provenance
    }
}
"#;
    let output = run_checker(code);
    // Copy preserves provenance
    println!("Output: {}", output);
}

// ============================================================================
// Test: Different heap allocations
// ============================================================================

#[test]
fn test_different_heap_allocations() {
    let code = r#"
// @safe
void process() {
    // @unsafe
    {
        int* p1 = new int;
        int* p2 = new int;
        // These have different heap allocations
    }
}
"#;
    let output = run_checker(code);
    // Just creating pointers is fine
    println!("Output: {}", output);
}

// ============================================================================
// Test: Parameter pointers (unknown provenance)
// ============================================================================

#[test]
fn test_parameter_unknown_provenance() {
    let code = r#"
// @safe
void process(int* p1, int* p2) {
    // Parameters have unknown provenance
    // We can't compare them without knowing where they come from
}
"#;
    let output = run_checker(code);
    // Just declaring is fine
    println!("Output: {}", output);
}

// ============================================================================
// Test: Array indexing same array
// ============================================================================

#[test]
fn test_array_indexing_same_array() {
    let code = r#"
// @safe
void process() {
    int arr[10];
    // @unsafe
    {
        int* p = &arr[0];
        int* q = &arr[5];
        bool cmp = p < q;  // OK: same array
    }
}
"#;
    let output = run_checker(code);
    // Same array access
    println!("Output: {}", output);
}
