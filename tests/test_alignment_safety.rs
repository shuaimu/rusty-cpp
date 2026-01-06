//! Integration tests for Alignment Safety (Phase 6 of Full Pointer Safety)
//!
//! Tests the detection of misaligned pointer access:
//! - Pointer arithmetic on char* can break alignment
//! - Casts to stricter alignment types
//! - Dereference of misaligned pointers

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
// Test: Properly aligned access (OK)
// ============================================================================

#[test]
fn test_aligned_access() {
    let code = r#"
// @safe
void process() {
    // @unsafe
    {
        int arr[10];
        int* p = arr;  // Properly aligned for int
        // Access via p is aligned
    }
}
"#;
    let output = run_checker(code);
    // Properly aligned - no alignment error
    println!("Output: {}", output);
}

// ============================================================================
// Test: Same-type pointer copy (OK)
// ============================================================================

#[test]
fn test_same_type_copy() {
    let code = r#"
// @safe
void process() {
    // @unsafe
    {
        int x = 42;
        int* p1 = &x;
        int* p2 = p1;  // Copy maintains alignment
    }
}
"#;
    let output = run_checker(code);
    // Same type copy is fine
    println!("Output: {}", output);
}

// ============================================================================
// Test: new expression alignment (OK)
// ============================================================================

#[test]
fn test_new_aligned() {
    let code = r#"
// @safe
void process() {
    // @unsafe
    {
        int* p = new int;  // new returns properly aligned memory
        // Access is aligned
    }
}
"#;
    let output = run_checker(code);
    // new returns aligned memory
    println!("Output: {}", output);
}

// ============================================================================
// Test: @unsafe function (no checks)
// ============================================================================

#[test]
fn test_unsafe_function_no_alignment_check() {
    let code = r#"
// @unsafe
void process() {
    char buffer[64];
    char* cp = buffer;
    cp++;
    int* ip = reinterpret_cast<int*>(cp);  // OK: @unsafe function
}
"#;
    let output = run_checker(code);
    // @unsafe function skips alignment checks
    assert!(
        !output.contains("misalign"),
        "@unsafe function should skip alignment checks. Output: {}",
        output
    );
}

// ============================================================================
// Test: @unsafe block (no checks)
// ============================================================================

#[test]
fn test_unsafe_block_no_alignment_check() {
    let code = r#"
// @safe
void process() {
    // @unsafe
    {
        char buffer[64];
        char* cp = buffer;
        cp++;
        int* ip = reinterpret_cast<int*>(cp);  // OK: inside @unsafe block
    }
}
"#;
    let output = run_checker(code);
    // @unsafe block skips alignment checks
    println!("Output: {}", output);
}

// ============================================================================
// Test: Stack variable alignment (OK)
// ============================================================================

#[test]
fn test_stack_var_aligned() {
    let code = r#"
// @safe
void process() {
    // @unsafe
    {
        long long x = 42;
        long long* p = &x;  // Stack variables are properly aligned
    }
}
"#;
    let output = run_checker(code);
    // Stack variables are aligned
    println!("Output: {}", output);
}

// ============================================================================
// Test: Array element access (OK)
// ============================================================================

#[test]
fn test_array_element_aligned() {
    let code = r#"
// @safe
void process() {
    // @unsafe
    {
        int arr[10];
        int* p = &arr[5];  // Array elements are aligned
    }
}
"#;
    let output = run_checker(code);
    // Array element access is aligned
    println!("Output: {}", output);
}

// ============================================================================
// Test: Static cast same type (OK)
// ============================================================================

#[test]
fn test_static_cast_same_type() {
    let code = r#"
// @safe
void process() {
    // @unsafe
    {
        int x = 42;
        int* p1 = &x;
        int* p2 = static_cast<int*>(p1);  // Same type cast
    }
}
"#;
    let output = run_checker(code);
    // Same type cast preserves alignment
    println!("Output: {}", output);
}

// ============================================================================
// Test: Type alignment requirements
// ============================================================================

#[test]
fn test_type_alignment_requirements() {
    // This is more of a unit test for the type alignment logic
    // The actual detection requires runtime tracking that we don't fully have
    let code = r#"
// @safe
void process() {
    // @unsafe
    {
        char c;
        short s;
        int i;
        long l;
        double d;
        // All properly aligned as individual variables
    }
}
"#;
    let output = run_checker(code);
    // Individual variables are always aligned
    println!("Output: {}", output);
}

// ============================================================================
// Test: Pointer to void (permissive)
// ============================================================================

#[test]
fn test_void_pointer() {
    let code = r#"
// @safe
void process() {
    // @unsafe
    {
        int x = 42;
        void* vp = &x;  // void* is permissive
    }
}
"#;
    let output = run_checker(code);
    // void* is permissive for alignment
    println!("Output: {}", output);
}

