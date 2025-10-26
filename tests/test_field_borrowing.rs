use std::process::Command;
use std::fs;

/// Integration tests for field borrowing in C++ methods
///
/// These tests check that:
/// - const methods (&self) can only create immutable borrows of fields
/// - non-const methods (&mut self) can create both mutable and immutable borrows
/// - Multiple immutable borrows are allowed
/// - Conflicting borrows (double mutable, mutable+immutable) are detected

#[test]
fn test_const_method_can_borrow_immutably() {
    let test_code = r#"
class Container {
private:
    int value;

public:
    // @safe
    void read_field() const {
        const int& ref = value;  // OK: const method can create immutable borrow
    }
};
"#;

    fs::write("test_const_immut.cpp", test_code).unwrap();

    let output = Command::new("cargo")
        .args(&["run", "--", "test_const_immut.cpp"])
        .output()
        .expect("Failed to run borrow checker");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should NOT detect any errors
    assert!(
        stdout.contains("no violations found") || stdout.contains("✓"),
        "Should allow const method to create immutable borrow. Output: {}",
        stdout
    );

    fs::remove_file("test_const_immut.cpp").unwrap();
}

#[test]
fn test_const_method_cannot_borrow_mutably() {
    let test_code = r#"
class Container {
private:
    int value;

public:
    // @safe
    void bad_const() const {
        int& ref = value;  // ERROR: const method cannot create mutable borrow
    }
};
"#;

    fs::write("test_const_mut.cpp", test_code).unwrap();

    let output = Command::new("cargo")
        .args(&["run", "--", "test_const_mut.cpp"])
        .output()
        .expect("Failed to run borrow checker");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Should detect that const method cannot create mutable borrow
    assert!(
        stdout.contains("Cannot create mutable borrow") && stdout.contains("const method")
        || stderr.contains("Cannot create mutable borrow") && stderr.contains("const method"),
        "Should detect const method trying to create mutable borrow. Output: {}\nError: {}",
        stdout, stderr
    );

    fs::remove_file("test_const_mut.cpp").unwrap();
}

#[test]
fn test_nonconst_method_can_borrow_mutably() {
    let test_code = r#"
class Container {
private:
    int value;

public:
    // @safe
    void modify_field() {
        int& ref = value;  // OK: non-const method can create mutable borrow
        ref = 42;
    }
};
"#;

    fs::write("test_nonconst_mut.cpp", test_code).unwrap();

    let output = Command::new("cargo")
        .args(&["run", "--", "test_nonconst_mut.cpp"])
        .output()
        .expect("Failed to run borrow checker");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should NOT detect any errors
    assert!(
        stdout.contains("no violations found") || stdout.contains("✓"),
        "Should allow non-const method to create mutable borrow. Output: {}",
        stdout
    );

    fs::remove_file("test_nonconst_mut.cpp").unwrap();
}

#[test]
fn test_multiple_immutable_borrows_allowed() {
    let test_code = r#"
class Container {
private:
    int value;

public:
    // @safe
    void multiple_readers() const {
        const int& ref1 = value;  // OK
        const int& ref2 = value;  // OK: multiple immutable borrows allowed
    }
};
"#;

    fs::write("test_multi_immut.cpp", test_code).unwrap();

    let output = Command::new("cargo")
        .args(&["run", "--", "test_multi_immut.cpp"])
        .output()
        .expect("Failed to run borrow checker");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should NOT detect any errors
    assert!(
        stdout.contains("no violations found") || stdout.contains("✓"),
        "Should allow multiple immutable borrows. Output: {}",
        stdout
    );

    fs::remove_file("test_multi_immut.cpp").unwrap();
}

#[test]
fn test_double_mutable_borrow_error() {
    let test_code = r#"
class Container {
private:
    int value;

public:
    // @safe
    void double_mut() {
        int& ref1 = value;  // OK
        int& ref2 = value;  // ERROR: already borrowed mutably
    }
};
"#;

    fs::write("test_double_mut.cpp", test_code).unwrap();

    let output = Command::new("cargo")
        .args(&["run", "--", "test_double_mut.cpp"])
        .output()
        .expect("Failed to run borrow checker");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Should detect double mutable borrow
    assert!(
        stdout.contains("already borrowed mutably")
        || stderr.contains("already borrowed mutably"),
        "Should detect double mutable borrow. Output: {}\nError: {}",
        stdout, stderr
    );

    fs::remove_file("test_double_mut.cpp").unwrap();
}

#[test]
fn test_mutable_then_immutable_conflict() {
    let test_code = r#"
class Container {
private:
    int value;

public:
    // @safe
    void mut_then_immut() {
        int& ref1 = value;        // OK
        const int& ref2 = value;  // ERROR: already borrowed mutably
    }
};
"#;

    fs::write("test_mut_immut.cpp", test_code).unwrap();

    let output = Command::new("cargo")
        .args(&["run", "--", "test_mut_immut.cpp"])
        .output()
        .expect("Failed to run borrow checker");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Should detect conflict
    assert!(
        stdout.contains("already borrowed mutably")
        || stderr.contains("already borrowed mutably"),
        "Should detect mutable-then-immutable conflict. Output: {}\nError: {}",
        stdout, stderr
    );

    fs::remove_file("test_mut_immut.cpp").unwrap();
}

#[test]
fn test_immutable_then_mutable_conflict() {
    let test_code = r#"
class Container {
private:
    int value;

public:
    // @safe
    void immut_then_mut() {
        const int& ref1 = value;  // OK
        int& ref2 = value;        // ERROR: already borrowed immutably
    }
};
"#;

    fs::write("test_immut_mut.cpp", test_code).unwrap();

    let output = Command::new("cargo")
        .args(&["run", "--", "test_immut_mut.cpp"])
        .output()
        .expect("Failed to run borrow checker");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Should detect conflict
    assert!(
        stdout.contains("already borrowed immutably")
        || stderr.contains("already borrowed immutably"),
        "Should detect immutable-then-mutable conflict. Output: {}\nError: {}",
        stdout, stderr
    );

    fs::remove_file("test_immut_mut.cpp").unwrap();
}

#[test]
fn test_nonconst_method_can_borrow_immutably() {
    let test_code = r#"
class Container {
private:
    int value;

public:
    // @safe
    void read_field() {
        const int& ref = value;  // OK: non-const method can create immutable borrow
    }
};
"#;

    fs::write("test_nonconst_immut.cpp", test_code).unwrap();

    let output = Command::new("cargo")
        .args(&["run", "--", "test_nonconst_immut.cpp"])
        .output()
        .expect("Failed to run borrow checker");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should NOT detect any errors
    assert!(
        stdout.contains("no violations found") || stdout.contains("✓"),
        "Should allow non-const method to create immutable borrow. Output: {}",
        stdout
    );

    fs::remove_file("test_nonconst_immut.cpp").unwrap();
}
