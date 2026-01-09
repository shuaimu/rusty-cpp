// Integration tests for Phase 4: Pointer Aliasing Tracking

use std::process::Command;
use std::fs;

fn run_checker(code: &str) -> String {
    let unique_id = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let thread_id = format!("{:?}", std::thread::current().id());
    let temp_file = format!("/tmp/test_pointer_aliasing_{}_{}.cpp", unique_id, thread_id);

    fs::write(&temp_file, code).unwrap();

    let output = Command::new("cargo")
        .args(&["run", "--release", "--", &temp_file])
        .output()
        .expect("Failed to run checker");

    let _ = fs::remove_file(&temp_file);
    String::from_utf8_lossy(&output.stdout).to_string()
}

// ============================================================================
// MUTABLE REFERENCE ALIASING TESTS
// ============================================================================

#[test]
fn test_mutable_ref_alias_moves_borrow() {
    // When assigning a mutable reference to another, the borrow MOVES
    let code = r#"
// @safe
void test() {
    int x = 42;
    int& r1 = x;    // r1 mutably borrows x
    int& r2 = r1;   // r2 takes over borrow, r1 is moved
    int y = r2;     // OK: use r2
}
"#;
    let output = run_checker(code);
    // Should pass - mutable ref assignment moves the borrow
    assert!(
        output.contains("no violations") || !output.contains("violation"),
        "Mutable ref assignment should move borrow. Output: {}", output
    );
}

#[test]
fn test_mutable_ref_use_after_move() {
    // Using a mutable reference after it was moved should error
    let code = r#"
// @safe
void test() {
    int x = 42;
    int& r1 = x;    // r1 mutably borrows x
    int& r2 = r1;   // r2 takes over borrow, r1 is moved
    int y = r1;     // ERROR: r1 was moved
}
"#;
    let output = run_checker(code);
    assert!(
        output.contains("moved"),
        "Should detect use of moved mutable reference. Output: {}", output
    );
}

// ============================================================================
// IMMUTABLE REFERENCE ALIASING TESTS
// ============================================================================

#[test]
fn test_const_ref_alias_copies_borrow() {
    // When assigning a const reference to another, the borrow is COPIED (both valid)
    let code = r#"
// @safe
void test() {
    int x = 42;
    const int& r1 = x;   // r1 immutably borrows x
    const int& r2 = r1;  // r2 also immutably borrows x
    int y = r1 + r2;     // OK: both are still valid
}
"#;
    let output = run_checker(code);
    assert!(
        output.contains("no violations") || !output.contains("violation"),
        "Const ref assignment should copy borrow. Output: {}", output
    );
}

#[test]
fn test_multiple_const_refs_allowed() {
    // Multiple const references to the same variable should be allowed
    let code = r#"
// @safe
void test() {
    int x = 42;
    const int& r1 = x;   // First immutable borrow
    const int& r2 = x;   // Second immutable borrow - OK
    const int& r3 = x;   // Third immutable borrow - OK
    int sum = r1 + r2 + r3;
}
"#;
    let output = run_checker(code);
    assert!(
        output.contains("no violations") || !output.contains("Cannot create"),
        "Multiple const refs should be allowed. Output: {}", output
    );
}

// ============================================================================
// POINTER ALIASING TESTS (in @unsafe context)
// ============================================================================

#[test]
fn test_pointer_alias_in_unsafe() {
    // Pointer aliasing in @unsafe functions should be allowed
    let code = r#"
// @unsafe
void test() {
    int x = 42;
    int* p = &x;   // OK: in unsafe
    int* q = p;    // OK: in unsafe (aliasing allowed)
    int* r = &x;   // OK: in unsafe (double borrow allowed)
}
"#;
    let output = run_checker(code);
    assert!(
        output.contains("no violations") || !output.contains("violation"),
        "Pointer aliasing in unsafe should be allowed. Output: {}", output
    );
}

// ============================================================================
// REFERENCE CHAIN TESTS
// ============================================================================

#[test]
fn test_mutable_ref_chain() {
    // Chain of mutable reference assignments: each one moves from previous
    let code = r#"
// @safe
void test() {
    int x = 42;
    int& r1 = x;
    int& r2 = r1;  // r1 moved
    int& r3 = r2;  // r2 moved
    int y = r3;    // OK: r3 has the borrow
}
"#;
    let output = run_checker(code);
    assert!(
        output.contains("no violations") || !output.contains("violation"),
        "Mutable ref chain should work. Output: {}", output
    );
}

#[test]
fn test_const_ref_chain() {
    // Chain of const reference assignments: all remain valid
    let code = r#"
// @safe
void test() {
    int x = 42;
    const int& r1 = x;
    const int& r2 = r1;  // r1 still valid
    const int& r3 = r2;  // r1, r2 still valid
    int sum = r1 + r2 + r3;  // All valid
}
"#;
    let output = run_checker(code);
    assert!(
        output.contains("no violations") || !output.contains("violation"),
        "Const ref chain should work. Output: {}", output
    );
}

// ============================================================================
// FUNCTION PARAMETER POINTER TESTS
// ============================================================================

#[test]
fn test_pointer_param_alias_not_tracked_deeply() {
    // Pointers from parameters don't have known sources, so aliasing is shallow
    // Note: Raw pointers require @unsafe in the new safety model
    let code = r#"
// @unsafe
void test(int* p) {
    int* q = p;  // q aliases p (no deep source tracking for params)
}
"#;
    let output = run_checker(code);
    // This should pass because we don't deeply track pointer parameter sources
    // (and because we're in @unsafe context)
    assert!(
        output.contains("no violations") || !output.contains("violation"),
        "Pointer param aliasing should not cause deep tracking issues. Output: {}", output
    );
}
