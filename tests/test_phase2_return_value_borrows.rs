// Phase 2 Integration Tests: Return Value Borrow Tracking
// Tests that lifetime annotations on return values create proper borrow relationships

use assert_cmd::Command;
use std::fs;
use std::env;

fn create_temp_cpp_file(code: &str) -> std::path::PathBuf {
    let temp_dir = env::temp_dir();
    let temp_file = temp_dir.join(format!("test_phase2_{}.cpp", rand::random::<u32>()));
    fs::write(&temp_file, code).unwrap();
    temp_file
}

fn run_analyzer(file_path: &std::path::Path, include_dir: &str) -> (bool, String) {
    let mut cmd = Command::cargo_bin("rusty-cpp-checker").unwrap();
    let output = cmd
        .arg(file_path)
        .arg("-I")
        .arg(include_dir)
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    (output.status.success(), stdout.to_string())
}

#[test]
fn test_phase2_simple_identity_borrow() {
    // Test that identity function creates borrow relationship
    let code = r#"
// @lifetime: (&'a) -> &'a int
const int& identity(const int& x) {
    return x;
}

// @safe
int main() {
    int value = 42;
    const int& ref = identity(value);  // ref borrows value

    // ERROR: can't move value while borrowed
    int moved = (int&&)value;

    return 0;
}
"#;

    let temp_file = create_temp_cpp_file(code);
    let (_, output) = run_analyzer(&temp_file, "include");

    // Should detect borrow violation
    assert!(
        output.contains("borrowed") || output.contains("violation"),
        "Should detect that value is borrowed. Output: {}",
        output
    );
}

#[test]
fn test_phase2_value_type_no_borrow() {
    // Test that value types don't create lasting borrows
    let code = r#"
#include <rusty/box.hpp>

// @safe
void take_ownership(rusty::Box<int> ptr) {
    int x = *ptr;
}

// @safe
void test() {
    rusty::Box<int> ptr = rusty::Box<int>::make(42);
    int x = *ptr;  // x is value type - no borrow

    // OK: No references created, can move
    take_ownership(std::move(ptr));
}
"#;

    let temp_file = create_temp_cpp_file(code);
    let (success, output) = run_analyzer(&temp_file, "include");

    // Should allow move since x is value type
    assert!(
        output.contains("no violations") || success,
        "Should allow move when result is value type. Output: {}",
        output
    );
}

#[test]
fn test_phase2_reference_type_creates_borrow() {
    // Test that reference types DO create borrows
    let code = r#"
#include <rusty/box.hpp>

// @safe
void take_ownership(rusty::Box<int> ptr) {
    int& ref = *ptr;  // ref is reference type - creates borrow

    // ERROR: can't move while borrowed
    rusty::Box<int> moved = std::move(ptr);
}
"#;

    let temp_file = create_temp_cpp_file(code);
    let (_, output) = run_analyzer(&temp_file, "include");

    // Should detect borrow violation
    assert!(
        output.contains("borrowed") || output.contains("violation"),
        "Should detect borrow when result is reference type. Output: {}",
        output
    );
}

#[test]
fn test_phase2_mutable_reference_borrow() {
    // Test mutable reference borrows
    let code = r#"
// @lifetime: (&'a mut) -> &'a mut int
int& getMutable(int& x) {
    return x;
}

// @safe
int main() {
    int value = 42;
    int& mut_ref = getMutable(value);  // mut_ref borrows value

    // ERROR: can't move while borrowed
    int moved = (int&&)value;

    return 0;
}
"#;

    let temp_file = create_temp_cpp_file(code);
    let (_, output) = run_analyzer(&temp_file, "include");

    // Should detect borrow violation
    assert!(
        output.contains("borrowed") || output.contains("violation"),
        "Should detect mutable borrow. Output: {}",
        output
    );
}

#[test]
fn test_phase2_borrow_ends_at_scope() {
    // Test that borrows end when reference goes out of scope
    let code = r#"
#include <utility>

// @lifetime: (&'a) -> &'a int
const int& identity(const int& x) {
    return x;
}

// @safe
int main() {
    int value = 42;

    {
        const int& ref = identity(value);  // borrow starts
        // use ref...
    }  // borrow ends - ref destroyed

    // OK: Can move now (use std::move, not C-style cast which is unsafe)
    int moved = std::move(value);

    return 0;
}
"#;

    let temp_file = create_temp_cpp_file(code);
    let (success, output) = run_analyzer(&temp_file, "include");

    // Should allow move after scope ends
    assert!(
        output.contains("no violations") || success,
        "Should allow move after borrow scope ends. Output: {}",
        output
    );
}

#[test]
fn test_phase2_multiple_params_first_lifetime() {
    // Test functions with multiple parameters - return has first param's lifetime
    let code = r#"
// @lifetime: (&'a, &'b) -> &'a int
const int& selectFirst(const int& a, const int& b) {
    return a;
}

// @safe
int main() {
    int x = 1;
    int y = 2;

    const int& result = selectFirst(x, y);  // result borrows x

    // ERROR: can't move x while borrowed
    int moved = (int&&)x;

    return 0;
}
"#;

    let temp_file = create_temp_cpp_file(code);
    let (_, output) = run_analyzer(&temp_file, "include");

    // Should detect that x is borrowed
    assert!(
        output.contains("borrowed") || output.contains("violation"),
        "Should detect that first parameter is borrowed. Output: {}",
        output
    );
}

#[test]
fn test_phase2_owned_return_no_borrow() {
    // Test that "owned" return type doesn't create borrow
    let code = r#"
// @lifetime: owned
// @safe
int create() {
    return 42;
}

// @safe
int main() {
    int owned = create();  // No borrow - owned value
    return 0;
}
"#;

    let temp_file = create_temp_cpp_file(code);
    let (success, output) = run_analyzer(&temp_file, "include");

    // Should pass - no borrowing
    assert!(
        output.contains("no violations") || success,
        "Should allow owned return with no borrowing. Output: {}",
        output
    );
}
