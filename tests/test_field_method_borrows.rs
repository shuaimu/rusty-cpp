// Tests for field-level borrow tracking through method calls
// Phase 2: Check for conflicts when calling methods on borrowed fields
// Phase 3: Track return value borrows from method calls on fields

use std::process::Command;
use std::fs;

/// Test that calling a method on a field that's borrowed causes an error
#[test]
fn test_method_call_on_borrowed_field() {
    let test_code = r#"
#include <string>

struct Inner {
    std::string data;

    void modify() { data = "modified"; }
    const std::string& get() const { return data; }
};

struct Outer {
    Inner inner;
};

// @safe
void test() {
    Outer o;
    const std::string& ref = o.inner.get();  // Borrow from inner
    o.inner.modify();  // ERROR: inner is borrowed by ref
}
"#;

    fs::write("test_field_borrow1.cpp", test_code).unwrap();

    let output = Command::new("cargo")
        .args(&["run", "--", "test_field_borrow1.cpp"])
        .output()
        .expect("Failed to run borrow checker");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Should detect that inner is borrowed
    assert!(stdout.contains("borrowed") || stdout.contains("Cannot call method"),
            "Should detect borrow conflict. Output: {}\nError: {}", stdout, stderr);

    // Clean up
    let _ = fs::remove_file("test_field_borrow1.cpp");
}

/// Test that consecutive method calls on same field work (no persistent borrow)
#[test]
fn test_consecutive_method_calls_ok() {
    let test_code = r#"
struct Inner {
    int data;

    void modify() { data = 1; }
};

struct Outer {
    Inner inner;
};

// @safe
void test() {
    Outer o;
    o.inner.modify();  // First call
    o.inner.modify();  // Second call - should be OK
}
"#;

    fs::write("test_field_borrow2.cpp", test_code).unwrap();

    let output = Command::new("cargo")
        .args(&["run", "--", "test_field_borrow2.cpp"])
        .output()
        .expect("Failed to run borrow checker");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should NOT detect borrow conflict between consecutive calls
    // (May have other errors like unsafe function calls, but not borrow conflicts)
    assert!(!stdout.contains("field is borrowed"),
            "Should NOT detect borrow conflict for consecutive calls. Output: {}", stdout);

    // Clean up
    let _ = fs::remove_file("test_field_borrow2.cpp");
}

/// Test that method return value borrows from receiver field
#[test]
fn test_method_return_borrows_from_field() {
    let test_code = r#"
#include <string>

struct Container {
    std::string data;

    const std::string& get() const { return data; }
    void set(const std::string& s) { data = s; }
};

struct Wrapper {
    Container container;
};

// @safe
void test() {
    Wrapper w;
    const std::string& ref = w.container.get();  // ref borrows from w.container
    w.container.set("new");  // ERROR: w.container is borrowed by ref
}
"#;

    fs::write("test_field_borrow3.cpp", test_code).unwrap();

    let output = Command::new("cargo")
        .args(&["run", "--", "test_field_borrow3.cpp"])
        .output()
        .expect("Failed to run borrow checker");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Should detect borrow conflict
    assert!(stdout.contains("borrowed") || stdout.contains("Cannot call method"),
            "Should detect borrow conflict. Output: {}\nError: {}", stdout, stderr);

    // Clean up
    let _ = fs::remove_file("test_field_borrow3.cpp");
}

/// Test that borrow ends when reference goes out of scope
#[test]
fn test_borrow_ends_at_scope_exit() {
    let test_code = r#"
struct Inner {
    int data;

    int& get() { return data; }
    void set(int v) { data = v; }
};

struct Outer {
    Inner inner;
};

// @safe
void test() {
    Outer o;
    {
        int& ref = o.inner.get();  // Borrow in inner scope
    }  // Borrow ends here
    o.inner.set(42);  // OK - no longer borrowed
}
"#;

    fs::write("test_field_borrow4.cpp", test_code).unwrap();

    let output = Command::new("cargo")
        .args(&["run", "--", "test_field_borrow4.cpp"])
        .output()
        .expect("Failed to run borrow checker");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should NOT detect borrow conflict (borrow ended with scope)
    assert!(!stdout.contains("field is borrowed"),
            "Should NOT detect borrow conflict after scope exit. Output: {}", stdout);

    // Clean up
    let _ = fs::remove_file("test_field_borrow4.cpp");
}

/// Test nested field access borrow tracking
#[test]
fn test_nested_field_borrow() {
    let test_code = r#"
#include <string>

struct Deepest {
    std::string value;

    const std::string& get() const { return value; }
    void set(const std::string& s) { value = s; }
};

struct Middle {
    Deepest deep;
};

struct Top {
    Middle mid;
};

// @safe
void test() {
    Top t;
    const std::string& ref = t.mid.deep.get();  // Borrow from t.mid.deep
    t.mid.deep.set("new");  // ERROR: t.mid.deep is borrowed
}
"#;

    fs::write("test_field_borrow5.cpp", test_code).unwrap();

    let output = Command::new("cargo")
        .args(&["run", "--", "test_field_borrow5.cpp"])
        .output()
        .expect("Failed to run borrow checker");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    // Should detect borrow conflict on nested field
    assert!(stdout.contains("borrowed") || stdout.contains("Cannot call method"),
            "Should detect borrow conflict on nested field. Output: {}\nError: {}", stdout, stderr);

    // Clean up
    let _ = fs::remove_file("test_field_borrow5.cpp");
}
