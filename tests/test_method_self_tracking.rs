use std::process::Command;
use std::fs;

/// Integration tests for Rust-like self tracking in C++ methods
///
/// These tests check that:
/// - const methods (&self) cannot move or modify fields
/// - non-const methods (&mut self) can modify but CANNOT move fields
/// - && methods (self) can move fields
///
/// NOTE: These tests are currently IGNORED because the parser doesn't yet
/// generate MoveField/UseField/BorrowField IR statements from method bodies.
/// Once parser enhancement is complete, remove #[ignore] to enable these tests.

#[test]
fn test_const_method_cannot_move_field() {
    let test_code = r#"
#include <memory>

class Container {
private:
    std::unique_ptr<int> data;

public:
    // @safe
    // const method (like Rust's &self) - should NOT be able to move fields
    std::unique_ptr<int> bad_const_method() const {
        // ERROR: Cannot move field from const method
        return std::move(data);
    }
};
"#;

    fs::write("test_const_move.cpp", test_code).unwrap();

    let output = Command::new("cargo")
        .args(&["run", "--", "test_const_move.cpp"])
        .output()
        .expect("Failed to run borrow checker");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Should detect that const method cannot move field
    assert!(
        stdout.contains("Cannot move field") && stdout.contains("const method")
        || stderr.contains("Cannot move field") && stderr.contains("const method"),
        "Should detect const method trying to move field. Output: {}\nError: {}",
        stdout, stderr
    );

    fs::remove_file("test_const_move.cpp").unwrap();
}

#[test]
fn test_nonconst_method_cannot_move_field() {
    let test_code = r#"
#include <memory>

class Container {
private:
    std::unique_ptr<int> data;

public:
    // @safe
    // non-const method (like Rust's &mut self) - can modify but NOT move
    void bad_mut_method() {
        // ERROR: Cannot move field from &mut self method
        auto temp = std::move(data);
    }
};
"#;

    fs::write("test_mut_move.cpp", test_code).unwrap();

    let output = Command::new("cargo")
        .args(&["run", "--", "test_mut_move.cpp"])
        .output()
        .expect("Failed to run borrow checker");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Should detect that &mut self method cannot move field
    assert!(
        stdout.contains("Cannot move field") && stdout.contains("&mut self")
        || stderr.contains("Cannot move field") && stderr.contains("&mut self"),
        "Should detect &mut self method trying to move field. Output: {}\nError: {}",
        stdout, stderr
    );

    fs::remove_file("test_mut_move.cpp").unwrap();
}

#[test]
fn test_rvalue_method_can_move_field() {
    let test_code = r#"
#include <memory>

class Container {
private:
    std::unique_ptr<int> data;

public:
    // @safe
    // && method (like Rust's self) - CAN move fields
    std::unique_ptr<int> consume() && {
        // OK: && method has full ownership
        return std::move(data);
    }
};

// @safe
void test() {
    Container c;
    auto result = std::move(c).consume();  // OK
}
"#;

    fs::write("test_rvalue_move.cpp", test_code).unwrap();

    let output = Command::new("cargo")
        .args(&["run", "--", "test_rvalue_move.cpp"])
        .output()
        .expect("Failed to run borrow checker");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should NOT detect any errors
    assert!(
        stdout.contains("no violations found") || stdout.contains("✓"),
        "Should allow && method to move field. Output: {}",
        stdout
    );

    fs::remove_file("test_rvalue_move.cpp").unwrap();
}

#[test]

fn test_const_method_cannot_modify_field() {
    let test_code = r#"
class Container {
private:
    int value;

public:
    // @safe
    // const method cannot modify fields
    void bad_const() const {
        // ERROR: Cannot modify field in const method
        value = 42;
    }
};
"#;

    fs::write("test_const_modify.cpp", test_code).unwrap();

    let output = Command::new("cargo")
        .args(&["run", "--", "test_const_modify.cpp"])
        .output()
        .expect("Failed to run borrow checker");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Should detect that const method cannot modify field
    assert!(
        stdout.contains("Cannot modify field") && stdout.contains("const method")
        || stderr.contains("Cannot modify field") && stderr.contains("const method"),
        "Should detect const method trying to modify field. Output: {}\nError: {}",
        stdout, stderr
    );

    fs::remove_file("test_const_modify.cpp").unwrap();
}

#[test]

fn test_nonconst_method_can_modify_field() {
    let test_code = r#"
class Container {
private:
    int value;

public:
    // @safe
    // non-const method CAN modify fields
    void modify(int new_val) {
        // OK: &mut self can modify
        value = new_val;
    }
};
"#;

    fs::write("test_mut_modify.cpp", test_code).unwrap();

    let output = Command::new("cargo")
        .args(&["run", "--", "test_mut_modify.cpp"])
        .output()
        .expect("Failed to run borrow checker");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should NOT detect any errors
    assert!(
        stdout.contains("no violations found") || stdout.contains("✓"),
        "Should allow &mut self to modify field. Output: {}",
        stdout
    );

    fs::remove_file("test_mut_modify.cpp").unwrap();
}

#[test]

fn test_multiple_field_moves_in_rvalue_method() {
    let test_code = r#"
#include <memory>

class Container {
private:
    std::unique_ptr<int> data1;
    std::unique_ptr<int> data2;

public:
    // @safe
    // && method can move multiple fields
    void consume_all() && {
        auto temp1 = std::move(data1);  // OK
        auto temp2 = std::move(data2);  // OK
    }
};
"#;

    fs::write("test_multi_move.cpp", test_code).unwrap();

    let output = Command::new("cargo")
        .args(&["run", "--", "test_multi_move.cpp"])
        .output()
        .expect("Failed to run borrow checker");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should NOT detect any errors
    assert!(
        stdout.contains("no violations found") || stdout.contains("✓"),
        "Should allow && method to move multiple fields. Output: {}",
        stdout
    );

    fs::remove_file("test_multi_move.cpp").unwrap();
}
