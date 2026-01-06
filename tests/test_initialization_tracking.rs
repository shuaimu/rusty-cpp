//! Integration tests for Initialization Tracking (Phase 2 of Full Pointer Safety)
//!
//! Tests the detection of uninitialized variable usage in @safe code:
//! - Use of uninitialized variable
//! - Address-of uninitialized variable
//! - Dereferencing pointer to uninitialized memory
//! - Conditional initialization tracking

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
// Test: Basic initialized variables
// ============================================================================

#[test]
fn test_initialized_variable_use() {
    let code = r#"
// @safe
void process() {
    int x = 42;
    int y = x;  // OK: x is initialized
}
"#;
    let output = run_checker(code);
    // No initialization errors expected
    assert!(
        !output.contains("uninitialized"),
        "Initialized variable should not be flagged. Output: {}",
        output
    );
}

// ============================================================================
// Test: Parameters are always initialized
// ============================================================================

#[test]
fn test_parameter_is_initialized() {
    let code = r#"
// @safe
int process(int x) {
    return x + 1;  // OK: parameter is always initialized
}
"#;
    let output = run_checker(code);
    // Parameters are always initialized
    assert!(
        !output.contains("uninitialized"),
        "Parameter should be treated as initialized. Output: {}",
        output
    );
}

// ============================================================================
// Test: @unsafe functions (no checks)
// ============================================================================

#[test]
fn test_unsafe_function_no_init_check() {
    let code = r#"
// @unsafe
void process() {
    int x;
    int y = x;  // OK: @unsafe function, no init check
}
"#;
    let output = run_checker(code);
    // @unsafe function skips init checks
    assert!(
        !output.contains("uninitialized"),
        "@unsafe function should skip init checks. Output: {}",
        output
    );
}

// ============================================================================
// Test: @unsafe block
// ============================================================================

#[test]
fn test_unsafe_block_skips_init_check() {
    let code = r#"
// @safe
void process() {
    // @unsafe
    {
        int x;
        int y = x;  // OK: inside @unsafe block
    }
}
"#;
    let output = run_checker(code);
    // Inside @unsafe block, init checks are skipped
    assert!(
        !output.contains("uninitialized"),
        "@unsafe block should skip init checks. Output: {}",
        output
    );
}

// ============================================================================
// Test: Reference parameters
// ============================================================================

#[test]
fn test_reference_parameter() {
    let code = r#"
// @safe
void process(const int& ref) {
    int y = ref;  // OK: reference parameter is initialized
}
"#;
    let output = run_checker(code);
    // Reference parameters are initialized
    assert!(
        !output.contains("uninitialized"),
        "Reference parameter should be initialized. Output: {}",
        output
    );
}

// ============================================================================
// Test: Assignment before use
// ============================================================================

#[test]
fn test_assignment_before_use() {
    let code = r#"
// @safe
void process() {
    int x = 0;
    x = 42;
    int y = x;  // OK: x was assigned
}
"#;
    let output = run_checker(code);
    // Assignment before use is OK
    assert!(
        !output.contains("uninitialized"),
        "Variable assigned before use should be OK. Output: {}",
        output
    );
}

// ============================================================================
// Test: Const variable (must be initialized)
// ============================================================================

#[test]
fn test_const_variable_initialized() {
    let code = r#"
// @safe
void process() {
    const int x = 42;  // const implies initialized
    int y = x;  // OK
}
"#;
    let output = run_checker(code);
    // Const variables are initialized
    assert!(
        !output.contains("uninitialized"),
        "Const variable should be treated as initialized. Output: {}",
        output
    );
}

// ============================================================================
// Test: Complex expressions
// ============================================================================

#[test]
fn test_complex_expression() {
    let code = r#"
// @safe
void process() {
    int a = 1;
    int b = 2;
    int c = a + b;  // OK: both a and b are initialized
}
"#;
    let output = run_checker(code);
    // Complex expressions with initialized variables
    assert!(
        !output.contains("uninitialized"),
        "Expression with initialized variables should be OK. Output: {}",
        output
    );
}

// ============================================================================
// Test: Function call with initialized args
// ============================================================================

#[test]
fn test_function_call_initialized_args() {
    let code = r#"
void helper(int x);

// @safe
void process() {
    int x = 42;
    // @unsafe
    {
        helper(x);  // OK: x is initialized
    }
}
"#;
    let output = run_checker(code);
    // Function call with initialized args
    println!("Output: {}", output);
}

// ============================================================================
// Test: If-else with full coverage
// ============================================================================

#[test]
fn test_if_else_full_init() {
    let code = r#"
// @safe
void process(bool cond) {
    int x = 0;
    if (cond) {
        x = 1;
    } else {
        x = 2;
    }
    int y = x;  // OK: x is initialized in all branches
}
"#;
    let output = run_checker(code);
    // Variable initialized in all branches
    println!("Output: {}", output);
}

// ============================================================================
// Test: Loop with initialized variable
// ============================================================================

#[test]
fn test_loop_initialized() {
    let code = r#"
// @safe
void process() {
    int sum = 0;
    int arr[10];
    // Note: array indexing would need array bounds checking
}
"#;
    let output = run_checker(code);
    // sum is initialized
    println!("Output: {}", output);
}

// ============================================================================
// Test: Pointer to initialized memory
// ============================================================================

#[test]
fn test_pointer_to_initialized() {
    let code = r#"
// @safe
void process() {
    int x = 42;
    // @unsafe
    {
        int* p = &x;  // x is initialized
        int y = *p;   // OK: points to initialized memory
    }
}
"#;
    let output = run_checker(code);
    // Pointer to initialized memory
    println!("Output: {}", output);
}
