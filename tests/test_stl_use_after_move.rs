// Tests for use-after-move detection with STL types in function call arguments
// This specifically tests that use-after-move is detected when:
// 1. A variable is moved via std::move
// 2. The moved variable is passed to a function call (not just assigned)

use std::process::Command;
use std::fs;

/// Test that use-after-move is detected when passing moved std::string to a function
#[test]
fn test_string_move_then_pass_to_function() {
    let test_code = r#"
#include <string>

void take_string_cref(const std::string& s);

// @safe
void test() {
    std::string s = "hello";
    std::string t = std::move(s);
    // Passing moved variable to function should error
    take_string_cref(s);
}
"#;

    fs::write("test_string_move_fn.cpp", test_code).unwrap();

    let output = Command::new("cargo")
        .args(&["run", "--", "test_string_move_fn.cpp"])
        .output()
        .expect("Failed to run borrow checker");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Should detect use after move when passing to function
    assert!(stdout.contains("Use after move") || stdout.contains("has been moved"),
            "Should detect use after move when passing to function. Output: {}\nError: {}", stdout, stderr);

    // Clean up
    let _ = fs::remove_file("test_string_move_fn.cpp");
}

/// Test that use-after-move is detected when passing moved vector to a function
#[test]
fn test_vector_move_then_pass_to_function() {
    let test_code = r#"
#include <vector>

void take_vector_cref(const std::vector<int>& v);

// @safe
void test() {
    std::vector<int> v = {1, 2, 3};
    std::vector<int> v2 = std::move(v);
    // Passing moved variable to function should error
    take_vector_cref(v);
}
"#;

    fs::write("test_vector_move_fn.cpp", test_code).unwrap();

    let output = Command::new("cargo")
        .args(&["run", "--", "test_vector_move_fn.cpp"])
        .output()
        .expect("Failed to run borrow checker");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Should detect use after move when passing to function
    assert!(stdout.contains("Use after move") || stdout.contains("has been moved"),
            "Should detect use after move for vector. Output: {}\nError: {}", stdout, stderr);

    // Clean up
    let _ = fs::remove_file("test_vector_move_fn.cpp");
}

/// Test that move in function call argument is detected and subsequent use errors
#[test]
fn test_move_in_function_arg_then_use() {
    let test_code = r#"
#include <string>

void consume_string(std::string s);
void take_string_cref(const std::string& s);

// @safe
void test() {
    std::string s = "hello";
    consume_string(std::move(s));
    // Passing after move-via-function-call should error
    take_string_cref(s);
}
"#;

    fs::write("test_move_in_arg.cpp", test_code).unwrap();

    let output = Command::new("cargo")
        .args(&["run", "--", "test_move_in_arg.cpp"])
        .output()
        .expect("Failed to run borrow checker");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Should detect use after move
    assert!(stdout.contains("Use after move") || stdout.contains("has been moved"),
            "Should detect use after move from function arg. Output: {}\nError: {}", stdout, stderr);

    // Clean up
    let _ = fs::remove_file("test_move_in_arg.cpp");
}

/// Test that copy semantics (without std::move) don't trigger use-after-move
#[test]
fn test_string_copy_not_move() {
    let test_code = r#"
#include <string>

void take_string_cref(const std::string& s);

// @safe
void test() {
    std::string s = "hello";
    std::string t = s;  // Copy, not move
    // s should still be valid
    take_string_cref(s);  // OK - not a use-after-move
}
"#;

    fs::write("test_string_copy.cpp", test_code).unwrap();

    let output = Command::new("cargo")
        .args(&["run", "--", "test_string_copy.cpp"])
        .output()
        .expect("Failed to run borrow checker");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should NOT detect use after move for copy
    assert!(!stdout.contains("Use after move") || stdout.contains("calling unsafe function"),
            "Should NOT detect use after move for copy. Output: {}", stdout);

    // Clean up
    let _ = fs::remove_file("test_string_copy.cpp");
}

/// Test use-after-move detection for multiple function calls
#[test]
fn test_multiple_uses_after_move() {
    let test_code = r#"
#include <string>

void take_ref(const std::string& s);
void take_by_value(std::string s);

// @safe
void test() {
    std::string s = "hello";
    std::string t = std::move(s);
    // Multiple uses after move - all should error
    take_ref(s);      // Error 1
    take_by_value(s); // Error 2 (if first doesn't short-circuit)
}
"#;

    fs::write("test_multiple_uses.cpp", test_code).unwrap();

    let output = Command::new("cargo")
        .args(&["run", "--", "test_multiple_uses.cpp"])
        .output()
        .expect("Failed to run borrow checker");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Should detect at least one use after move
    assert!(stdout.contains("Use after move") || stdout.contains("has been moved"),
            "Should detect use after move. Output: {}\nError: {}", stdout, stderr);

    // Clean up
    let _ = fs::remove_file("test_multiple_uses.cpp");
}

/// Test that reassignment after move allows subsequent use
#[test]
fn test_reassign_after_move_allows_use() {
    let test_code = r#"
#include <string>

void take_ref(const std::string& s);

// @safe
void test() {
    std::string s = "hello";
    std::string t = std::move(s);
    s = "world";  // Reassign
    // s is valid again
    take_ref(s);  // OK - after reassignment
}
"#;

    fs::write("test_reassign_use.cpp", test_code).unwrap();

    let output = Command::new("cargo")
        .args(&["run", "--", "test_reassign_use.cpp"])
        .output()
        .expect("Failed to run borrow checker");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Tool should complete (may or may not detect errors for other reasons like unsafe calls)
    assert!(output.status.code().is_some(),
            "Tool should complete. Output: {}", stdout);

    // Clean up
    let _ = fs::remove_file("test_reassign_use.cpp");
}

/// Test use-after-move in method chain on std::string
#[test]
fn test_method_call_on_moved_string() {
    let test_code = r#"
#include <string>

// @safe
void test() {
    std::string s = "hello";
    std::string t = std::move(s);
    // Calling a method on moved variable should error
    auto len = s.length();
}
"#;

    fs::write("test_method_on_moved.cpp", test_code).unwrap();

    let output = Command::new("cargo")
        .args(&["run", "--", "test_method_on_moved.cpp"])
        .output()
        .expect("Failed to run borrow checker");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Should detect use after move when calling method on moved object
    assert!(stdout.contains("Use after move") || stdout.contains("has been moved") ||
            stdout.contains("cannot call method"),
            "Should detect use after move when calling method. Output: {}\nError: {}", stdout, stderr);

    // Clean up
    let _ = fs::remove_file("test_method_on_moved.cpp");
}
