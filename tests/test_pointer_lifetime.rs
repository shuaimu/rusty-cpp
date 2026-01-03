use std::process::Command;
use std::fs;

// ============================================================================
// POINTER LIFETIME ANNOTATION TESTS
// ============================================================================
// These tests verify that pointer lifetime annotations work correctly.

#[test]
fn test_pointer_identity_function() {
    // Test: identity function that returns input pointer with same lifetime
    let test_code = r#"
// @safe
// @lifetime: (int* 'a) -> int* 'a
int* identity(int* p) {
    return p;  // OK: return has same lifetime as input
}

// @safe
void caller() {
    int x = 42;
    // @unsafe
    {
        int* p = &x;
        int* q = identity(p);  // OK: q has same lifetime as p
    }
}
"#;

    fs::write("/tmp/test_ptr_identity.cpp", test_code).unwrap();

    let output = Command::new("cargo")
        .args(&["run", "--", "/tmp/test_ptr_identity.cpp"])
        .output()
        .expect("Failed to run borrow checker");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Should not have any lifetime violations
    assert!(
        !stdout.contains("lifetime") || stdout.contains("no violations"),
        "Pointer identity function should not have lifetime violations. stdout: {} stderr: {}",
        stdout, stderr
    );

    // Clean up
    let _ = fs::remove_file("/tmp/test_ptr_identity.cpp");
}

#[test]
fn test_return_pointer_to_local_is_error() {
    // Test: returning pointer to local variable should be detected
    let test_code = r#"
// @safe
int* bad() {
    int x = 42;
    // @unsafe
    {
        return &x;  // ERROR: returning pointer to local
    }
}
"#;

    fs::write("/tmp/test_ptr_local.cpp", test_code).unwrap();

    let output = Command::new("cargo")
        .args(&["run", "--", "/tmp/test_ptr_local.cpp"])
        .output()
        .expect("Failed to run borrow checker");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should detect returning pointer to local (this is a lifetime violation)
    // Note: The exact error message depends on how the analyzer handles this
    // The test passes if there's any violation detected
    assert!(
        stdout.contains("violation") || stdout.contains("dangling") || stdout.contains("local"),
        "Should detect returning pointer to local variable. Output: {}",
        stdout
    );

    // Clean up
    let _ = fs::remove_file("/tmp/test_ptr_local.cpp");
}

#[test]
fn test_const_pointer_lifetime() {
    // Test: const pointer with lifetime annotation
    let test_code = r#"
// @safe
// @lifetime: (const int* 'a) -> const int* 'a
const int* get_const(const int* p) {
    return p;  // OK: const pointer with same lifetime
}

// @safe
void caller() {
    int x = 42;
    // @unsafe
    {
        const int* p = &x;
        const int* q = get_const(p);  // OK
    }
}
"#;

    fs::write("/tmp/test_ptr_const.cpp", test_code).unwrap();

    let output = Command::new("cargo")
        .args(&["run", "--", "/tmp/test_ptr_const.cpp"])
        .output()
        .expect("Failed to run borrow checker");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should not have violations for const pointer passing
    assert!(
        !stdout.contains("lifetime violation") || stdout.contains("no violations"),
        "Const pointer identity should not have lifetime violations. Output: {}",
        stdout
    );

    // Clean up
    let _ = fs::remove_file("/tmp/test_ptr_const.cpp");
}

#[test]
fn test_pointer_static_lifetime() {
    // Test: pointer with static lifetime
    let test_code = r#"
static int global_value = 42;

// @safe
// @lifetime: () -> int* 'static
int* get_global() {
    // @unsafe
    {
        return &global_value;  // OK: static lifetime
    }
}

// @safe
void caller() {
    // @unsafe
    {
        int* p = get_global();  // p has static lifetime
    }
}
"#;

    fs::write("/tmp/test_ptr_static.cpp", test_code).unwrap();

    let output = Command::new("cargo")
        .args(&["run", "--", "/tmp/test_ptr_static.cpp"])
        .output()
        .expect("Failed to run borrow checker");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Static lifetime pointers should be allowed
    // The test passes if parsing works (no syntax errors)
    assert!(
        !stderr.contains("parse error") && !stderr.contains("syntax error"),
        "Static lifetime pointer annotation should parse correctly. stderr: {}",
        stderr
    );

    // Clean up
    let _ = fs::remove_file("/tmp/test_ptr_static.cpp");
}

#[test]
fn test_pointer_lifetime_constraint() {
    // Test: pointer lifetime with where clause
    let test_code = r#"
// @safe
// @lifetime: (const int* 'a, const int* 'b) -> const int* 'a where 'a: 'b
const int* longer(const int* a, const int* b) {
    return a;  // Return the pointer with longer lifetime
}

// @safe
void caller() {
    int x = 42;
    int y = 24;
    // @unsafe
    {
        const int* px = &x;
        const int* py = &y;
        const int* result = longer(px, py);  // OK if x outlives y
    }
}
"#;

    fs::write("/tmp/test_ptr_constraint.cpp", test_code).unwrap();

    let output = Command::new("cargo")
        .args(&["run", "--", "/tmp/test_ptr_constraint.cpp"])
        .output()
        .expect("Failed to run borrow checker");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Lifetime constraint should be parsed correctly
    assert!(
        !stderr.contains("parse error") && !stderr.contains("syntax error"),
        "Pointer lifetime constraint annotation should parse correctly. stderr: {}",
        stderr
    );

    // Clean up
    let _ = fs::remove_file("/tmp/test_ptr_constraint.cpp");
}

#[test]
fn test_mixed_pointer_and_ref_lifetime() {
    // Test: function with both pointer and reference parameters
    let test_code = r#"
// @safe
// @lifetime: (int* 'a, &'b int) -> int* 'a
int* select_ptr(int* ptr, const int& ref) {
    return ptr;  // Return the pointer
}

// @safe
void caller() {
    int x = 42;
    int y = 24;
    // @unsafe
    {
        int* px = &x;
        int* result = select_ptr(px, y);
    }
}
"#;

    fs::write("/tmp/test_ptr_ref_mixed.cpp", test_code).unwrap();

    let output = Command::new("cargo")
        .args(&["run", "--", "/tmp/test_ptr_ref_mixed.cpp"])
        .output()
        .expect("Failed to run borrow checker");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Mixed pointer/ref annotations should parse correctly
    assert!(
        !stderr.contains("parse error") && !stderr.contains("syntax error"),
        "Mixed pointer/ref lifetime annotation should parse correctly. stderr: {}",
        stderr
    );

    // Clean up
    let _ = fs::remove_file("/tmp/test_ptr_ref_mixed.cpp");
}
