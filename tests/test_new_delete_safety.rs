use std::process::Command;
use std::fs;

// ============================================================================
// NEW/DELETE DETECTION TESTS
// ============================================================================

#[test]
fn test_new_in_safe_function_forbidden() {
    // new expression should be forbidden in @safe code
    let test_code = r#"
// @safe
void test() {
    int* p = new int(42);  // ERROR: new requires unsafe
}
"#;

    fs::write("test_new_safe.cpp", test_code).unwrap();

    let output = Command::new("cargo")
        .args(&["run", "--", "test_new_safe.cpp"])
        .output()
        .expect("Failed to run borrow checker");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should detect new as unsafe operation
    assert!(stdout.contains("new"),
            "Should detect 'new' in @safe code. Output: {}", stdout);
    assert!(stdout.contains("unsafe") || stdout.contains("pointer operations"),
            "Should report pointer operations require unsafe. Output: {}", stdout);

    // Clean up
    let _ = fs::remove_file("test_new_safe.cpp");
}

#[test]
fn test_delete_in_safe_function_forbidden() {
    // delete expression should be forbidden in @safe code
    let test_code = r#"
// @safe
void test(int* p) {
    delete p;  // ERROR: delete requires unsafe
}
"#;

    fs::write("test_delete_safe.cpp", test_code).unwrap();

    let output = Command::new("cargo")
        .args(&["run", "--", "test_delete_safe.cpp"])
        .output()
        .expect("Failed to run borrow checker");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should detect delete as unsafe operation
    assert!(stdout.contains("delete"),
            "Should detect 'delete' in @safe code. Output: {}", stdout);
    assert!(stdout.contains("unsafe") || stdout.contains("pointer operations"),
            "Should report pointer operations require unsafe. Output: {}", stdout);

    // Clean up
    let _ = fs::remove_file("test_delete_safe.cpp");
}

#[test]
fn test_new_array_in_safe_function_forbidden() {
    // new[] expression should be forbidden in @safe code
    let test_code = r#"
// @safe
void test() {
    int* arr = new int[10];  // ERROR: new[] requires unsafe
}
"#;

    fs::write("test_new_array_safe.cpp", test_code).unwrap();

    let output = Command::new("cargo")
        .args(&["run", "--", "test_new_array_safe.cpp"])
        .output()
        .expect("Failed to run borrow checker");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should detect new as unsafe operation
    assert!(stdout.contains("new"),
            "Should detect 'new[]' in @safe code. Output: {}", stdout);

    // Clean up
    let _ = fs::remove_file("test_new_array_safe.cpp");
}

#[test]
fn test_new_allowed_in_unsafe_function() {
    // new expression should be allowed in @unsafe code
    let test_code = r#"
// @unsafe
void test() {
    int* p = new int(42);  // OK: in unsafe function
    delete p;              // OK: in unsafe function
}
"#;

    fs::write("test_new_unsafe_func.cpp", test_code).unwrap();

    let output = Command::new("cargo")
        .args(&["run", "--", "test_new_unsafe_func.cpp"])
        .output()
        .expect("Failed to run borrow checker");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should NOT detect any violations
    assert!(stdout.contains("no violations") || stdout.contains("✓"),
            "new/delete should be allowed in @unsafe functions. Output: {}", stdout);

    // Clean up
    let _ = fs::remove_file("test_new_unsafe_func.cpp");
}

#[test]
fn test_new_allowed_in_unsafe_block() {
    // new expression should be allowed in @unsafe block within @safe function
    let test_code = r#"
// @safe
void test() {
    // @unsafe
    {
        int* p = new int(42);  // OK: in @unsafe block
        delete p;              // OK: in @unsafe block
    }
}
"#;

    fs::write("test_new_unsafe_block.cpp", test_code).unwrap();

    let output = Command::new("cargo")
        .args(&["run", "--", "test_new_unsafe_block.cpp"])
        .output()
        .expect("Failed to run borrow checker");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should NOT detect any violations
    assert!(stdout.contains("no violations") || stdout.contains("✓"),
            "new/delete should be allowed in @unsafe blocks. Output: {}", stdout);

    // Clean up
    let _ = fs::remove_file("test_new_unsafe_block.cpp");
}

#[test]
fn test_multiple_new_delete_violations() {
    // Multiple new/delete expressions should all be detected
    let test_code = r#"
// @safe
void test1() {
    int* p = new int(42);  // ERROR 1
}

// @safe
void test2(int* p) {
    delete p;  // ERROR 2
}

// @safe
void test3() {
    int* arr = new int[10];  // ERROR 3
}

// @safe
void test4(int* arr) {
    delete[] arr;  // ERROR 4
}
"#;

    fs::write("test_multiple_new_delete.cpp", test_code).unwrap();

    let output = Command::new("cargo")
        .args(&["run", "--", "test_multiple_new_delete.cpp"])
        .output()
        .expect("Failed to run borrow checker");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should detect all 4 violations
    assert!(stdout.contains("4 violation"),
            "Should detect 4 violations (2 new, 2 delete). Output: {}", stdout);

    // Clean up
    let _ = fs::remove_file("test_multiple_new_delete.cpp");
}
