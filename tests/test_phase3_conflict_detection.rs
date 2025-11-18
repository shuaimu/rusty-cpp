// Phase 3: Multiple Borrow Conflict Detection Tests

use assert_cmd::Command;
use std::fs;
use tempfile::NamedTempFile;

fn run_analyzer_on_code(code: &str, include_paths: &[&str]) -> (String, bool) {
    let mut file = NamedTempFile::new().unwrap();
    fs::write(&file, code).unwrap();

    let mut cmd = Command::cargo_bin("rusty-cpp-checker").unwrap();
    cmd.arg(file.path());
    for path in include_paths {
        cmd.arg("-I").arg(path);
    }

    let output = cmd.output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let full_output = format!("{}\n{}", stdout, stderr);

    let has_violation = full_output.contains("Found") && full_output.contains("violation");
    (full_output, has_violation)
}

#[test]
fn test_phase3_multiple_mutable_borrows_conflict() {
    let code = r#"
// @lifetime: (&'a mut) -> &'a mut int
int& identity(int& x) { return x; }

// @safe
int main() {
    int value = 42;

    // First mutable borrow
    int& ref1 = identity(value);

    // Second mutable borrow - should ERROR
    int& ref2 = identity(value);

    return 0;
}
"#;

    let (output, has_violation) = run_analyzer_on_code(code, &[]);
    println!("Output: {}", output);

    assert!(has_violation, "Should detect conflict between two mutable borrows");
    assert!(output.contains("already mutably borrowed") || output.contains("already borrowed"),
        "Error message should mention already borrowed");
}

#[test]
fn test_phase3_mutable_and_immutable_conflict() {
    let code = r#"
// @lifetime: (&'a) -> &'a int
const int& identity_const(const int& x) { return x; }

// @lifetime: (&'a mut) -> &'a mut int
int& identity_mut(int& x) { return x; }

// @safe
int main() {
    int value = 42;

    // Immutable borrow
    const int& ref1 = identity_const(value);

    // Mutable borrow - should ERROR (can't have mut while immutable exists)
    int& ref2 = identity_mut(value);

    return 0;
}
"#;

    let (output, has_violation) = run_analyzer_on_code(code, &[]);
    println!("Output: {}", output);

    assert!(has_violation, "Should detect conflict between immutable and mutable borrows");
}

#[test]
fn test_phase3_multiple_immutable_borrows_allowed() {
    let code = r#"
// @lifetime: (&'a) -> &'a int
const int& identity(const int& x) { return x; }

// @safe
int main() {
    int value = 42;

    // First immutable borrow
    const int& ref1 = identity(value);

    // Second immutable borrow - should be OK
    const int& ref2 = identity(value);

    return 0;
}
"#;

    let (output, has_violation) = run_analyzer_on_code(code, &[]);
    println!("Output: {}", output);

    assert!(!has_violation, "Multiple immutable borrows should be allowed");
}

#[test]
fn test_phase3_sequential_mutable_borrows_allowed() {
    let code = r#"
// @lifetime: (&'a mut) -> &'a mut int
int& identity(int& x) { return x; }

// @safe
int main() {
    int value = 42;

    {
        // First mutable borrow in inner scope
        int& ref1 = identity(value);
    }  // ref1 ends here

    // Second mutable borrow - should be OK (ref1 is gone)
    int& ref2 = identity(value);

    return 0;
}
"#;

    let (output, has_violation) = run_analyzer_on_code(code, &[]);
    println!("Output: {}", output);

    assert!(!has_violation, "Sequential mutable borrows in different scopes should be allowed");
}

#[test]
fn test_phase3_box_operator_star_conflicts() {
    let code = r#"
#include <rusty/box.hpp>

// @safe
int main() {
    rusty::Box<int> ptr = rusty::Box<int>::make(42);

    // First mutable reference via operator*
    int& ref1 = *ptr;

    // Second mutable reference - should ERROR
    int& ref2 = *ptr;

    return 0;
}
"#;

    let (output, has_violation) = run_analyzer_on_code(code, &["include"]);
    println!("Output: {}", output);

    assert!(has_violation, "Should detect conflict between two mutable borrows via operator*");
    assert!(output.contains("already mutably borrowed") || output.contains("already borrowed"),
        "Error message should mention already borrowed");
}

#[test]
fn test_phase3_different_objects_no_conflict() {
    let code = r#"
// @lifetime: (&'a mut) -> &'a mut int
int& identity(int& x) { return x; }

// @safe
int main() {
    int value1 = 42;
    int value2 = 100;

    // Borrow different objects - should be OK
    int& ref1 = identity(value1);
    int& ref2 = identity(value2);

    return 0;
}
"#;

    let (output, has_violation) = run_analyzer_on_code(code, &[]);
    println!("Output: {}", output);

    assert!(!has_violation, "Borrowing different objects should not conflict");
}

#[test]
fn test_phase3_conflict_prevents_borrow_creation() {
    let code = r#"
// @lifetime: (&'a mut) -> &'a mut int
int& identity(int& x) { return x; }

// @safe
int main() {
    int value = 42;

    int& ref1 = identity(value);  // First borrow
    int& ref2 = identity(value);  // Second borrow - conflict detected, not created
    int& ref3 = identity(value);  // Third attempt

    return 0;
}
"#;

    let (output, has_violation) = run_analyzer_on_code(code, &[]);
    println!("Output: {}", output);

    // We should see multiple conflict errors (one for each attempt)
    assert!(has_violation, "Should detect all conflict attempts");
}
