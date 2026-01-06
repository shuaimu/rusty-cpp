// Tests for mixed pointer-reference borrow checking
// Verifies that borrow rules are enforced uniformly for both pointers and references,
// even in @unsafe code.

use std::process::Command;
use std::fs;

fn run_checker(code: &str) -> String {
    let unique_id = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let thread_id = format!("{:?}", std::thread::current().id());
    let temp_file = format!("/tmp/test_mixed_borrow_{}_{}.cpp", unique_id, thread_id);

    fs::write(&temp_file, code).unwrap();

    let output = Command::new("cargo")
        .args(&["run", "--release", "--", &temp_file])
        .output()
        .expect("Failed to run checker");

    let _ = fs::remove_file(&temp_file);
    String::from_utf8_lossy(&output.stdout).to_string()
}

// ============================================================================
// MUTABLE REFERENCE + POINTER CONFLICT TESTS
// ============================================================================

#[test]
fn test_mutable_ref_then_mutable_pointer_conflicts() {
    // Mutable reference then mutable pointer should fail
    let code = r#"
// @unsafe
void test() {
    int x = 42;
    int& ref = x;       // Mutable reference borrows x
    int* ptr = &x;      // ERROR: pointer conflicts with mutable reference
}
"#;
    let output = run_checker(code);
    assert!(
        output.contains("already mutably borrowed") || output.contains("violation"),
        "Should detect pointer-reference conflict. Output: {}", output
    );
}

#[test]
fn test_mutable_pointer_then_mutable_ref_conflicts() {
    // Mutable pointer then mutable reference should fail
    let code = r#"
// @unsafe
void test() {
    int x = 42;
    int* ptr = &x;      // Mutable pointer borrows x
    int& ref = x;       // ERROR: reference conflicts with mutable pointer
}
"#;
    let output = run_checker(code);
    assert!(
        output.contains("already mutably borrowed") || output.contains("violation"),
        "Should detect reference-pointer conflict. Output: {}", output
    );
}

#[test]
fn test_mutable_ref_then_const_pointer_conflicts() {
    // Mutable reference then const pointer should fail (cannot have immutable view of mutably borrowed data)
    let code = r#"
// @unsafe
void test() {
    int x = 42;
    int& ref = x;           // Mutable reference borrows x
    const int* ptr = &x;    // ERROR: cannot create immutable borrow when mutable exists
}
"#;
    let output = run_checker(code);
    assert!(
        output.contains("already mutably borrowed") || output.contains("immutable reference") ||
        output.contains("violation"),
        "Should detect const pointer conflict with mutable ref. Output: {}", output
    );
}

#[test]
fn test_const_ref_then_mutable_pointer_conflicts() {
    // Const reference then mutable pointer should fail
    let code = r#"
// @unsafe
void test() {
    int x = 42;
    const int& ref = x;     // Immutable reference borrows x
    int* ptr = &x;          // ERROR: cannot create mutable borrow when immutable exists
}
"#;
    let output = run_checker(code);
    assert!(
        output.contains("already") || output.contains("borrowed") || output.contains("violation"),
        "Should detect mutable pointer conflict with const ref. Output: {}", output
    );
}

// ============================================================================
// MULTIPLE IMMUTABLE BORROWS (SHOULD BE ALLOWED)
// ============================================================================

#[test]
fn test_const_ref_and_const_pointer_allowed() {
    // Multiple immutable borrows should be allowed
    let code = r#"
// @unsafe
void test() {
    int x = 42;
    const int& ref = x;     // Immutable reference
    const int* ptr = &x;    // OK: another immutable borrow
    int y = ref + *ptr;
}
"#;
    let output = run_checker(code);
    assert!(
        output.contains("no violations") || !output.contains("borrow"),
        "Multiple const borrows should be allowed. Output: {}", output
    );
}

#[test]
fn test_multiple_const_pointers_allowed() {
    // Multiple const pointers should be allowed
    let code = r#"
// @unsafe
void test() {
    int x = 42;
    const int* p1 = &x;
    const int* p2 = &x;
    const int* p3 = &x;
}
"#;
    let output = run_checker(code);
    assert!(
        output.contains("no violations") || !output.contains("borrow"),
        "Multiple const pointers should be allowed. Output: {}", output
    );
}

// ============================================================================
// DOUBLE MUTABLE BORROW TESTS (REFERENCE-REFERENCE)
// ============================================================================

#[test]
fn test_double_mutable_ref_conflicts() {
    // Two mutable references should conflict
    let code = r#"
// @safe
void test() {
    int x = 42;
    int& ref1 = x;
    int& ref2 = x;  // ERROR: double mutable borrow
}
"#;
    let output = run_checker(code);
    assert!(
        output.contains("already mutably borrowed") || output.contains("already borrowed"),
        "Should detect double mutable reference. Output: {}", output
    );
}

#[test]
fn test_double_mutable_pointer_conflicts() {
    // Two mutable pointers should conflict
    let code = r#"
// @unsafe
void test() {
    int x = 42;
    int* p1 = &x;   // Mutable pointer
    int* p2 = &x;   // ERROR: double mutable borrow
}
"#;
    let output = run_checker(code);
    assert!(
        output.contains("already mutably borrowed") || output.contains("violation"),
        "Should detect double mutable pointer. Output: {}", output
    );
}

// ============================================================================
// SCOPE-BASED BORROW ENDING
// ============================================================================

#[test]
fn test_borrow_ends_at_scope_exit() {
    // Borrow should end when scope exits
    let code = r#"
// @unsafe
void test() {
    int x = 42;
    {
        int& ref = x;   // Borrow in inner scope
    }  // Borrow ends here
    int* ptr = &x;      // OK: no active borrow
}
"#;
    let output = run_checker(code);
    // May have other errors but not borrow conflicts
    assert!(
        output.contains("no violations") || !output.contains("already"),
        "Borrow should end at scope exit. Output: {}", output
    );
}

// ============================================================================
// @UNSAFE FUNCTION BORROW CHECKING
// ============================================================================

#[test]
fn test_unsafe_function_still_checks_borrows() {
    // @unsafe function should still check borrow conflicts
    let code = r#"
// @unsafe
void test_unsafe() {
    int x = 42;
    int& ref = x;
    int& ref2 = x;  // ERROR: even @unsafe should detect this
}
"#;
    let output = run_checker(code);
    assert!(
        output.contains("already mutably borrowed") || output.contains("already borrowed"),
        "@unsafe should still check borrow conflicts. Output: {}", output
    );
}

#[test]
fn test_unsafe_block_in_safe_function_checks_borrows() {
    // @unsafe block should still check borrow conflicts
    let code = r#"
// @safe
void test() {
    int x = 42;
    int& ref = x;
    // @unsafe
    {
        int* ptr = &x;  // ERROR: conflicts with ref
    }
}
"#;
    let output = run_checker(code);
    assert!(
        output.contains("already mutably borrowed") || output.contains("violation"),
        "@unsafe block should still check borrow conflicts. Output: {}", output
    );
}
