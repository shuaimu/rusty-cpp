// Phase 4: Transitive Borrow Tracking Tests

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

    // Check specifically for borrow-related violations (not safety annotation violations)
    let has_borrow_violation = full_output.contains("borrowed") ||
                                full_output.contains("Cannot move");
    (full_output, has_borrow_violation)
}

#[test]
fn test_phase4_two_level_transitive_borrow() {
    let code = r#"
#include <rusty/box.hpp>

// @lifetime: (&'a) -> &'a int
const int& identity(const int& x) { return x; }

// @safe
int main() {
    rusty::Box<int> value = rusty::Box<int>::make(42);
    int& ref1 = *value;
    const int& ref2 = identity(ref1);
    rusty::Box<int> moved = std::move(value);  // ERROR: transitive borrow
    return 0;
}
"#;

    let (output, has_violation) = run_analyzer_on_code(code, &["include"]);
    println!("Output: {}", output);
    assert!(has_violation, "Should detect transitive borrow preventing move");
}

#[test]
fn test_phase4_three_level_borrow_chain() {
    let code = r#"
#include <rusty/box.hpp>

// @lifetime: (&'a) -> &'a int
const int& identity(const int& x) { return x; }

// @safe
int main() {
    rusty::Box<int> value = rusty::Box<int>::make(42);
    int& ref1 = *value;
    const int& ref2 = identity(ref1);
    const int& ref3 = identity(ref2);
    rusty::Box<int> moved = std::move(value);  // ERROR: three-level chain
    return 0;
}
"#;

    let (output, has_violation) = run_analyzer_on_code(code, &["include"]);
    println!("Output: {}", output);
    assert!(has_violation, "Should detect three-level transitive borrow");
}

#[test]
fn test_phase4_move_allowed_after_all_borrows_end() {
    let code = r#"
#include <rusty/box.hpp>

// @lifetime: (&'a) -> &'a int
const int& identity(const int& x) { return x; }

// @safe
int main() {
    rusty::Box<int> value = rusty::Box<int>::make(42);
    {
        int& ref1 = *value;
        {
            const int& ref2 = identity(ref1);
        }  // ref2 ends
    }  // ref1 ends
    rusty::Box<int> moved = std::move(value);  // OK: all borrows ended
    return 0;
}
"#;

    let (output, has_violation) = run_analyzer_on_code(code, &["include"]);
    println!("Output: {}", output);
    assert!(!has_violation, "Move should be allowed after all borrows end");
}

#[test]
fn test_phase4_box_with_transitive_borrows() {
    let code = r#"
#include <rusty/box.hpp>

// @lifetime: (&'a) -> &'a int
const int& identity(const int& x) { return x; }

// @safe
int main() {
    rusty::Box<int> ptr = rusty::Box<int>::make(42);
    int& ref1 = *ptr;
    const int& ref2 = identity(ref1);
    rusty::Box<int> moved = std::move(ptr);  // ERROR: transitive chain
    return 0;
}
"#;

    let (output, has_violation) = run_analyzer_on_code(code, &["include"]);
    println!("Output: {}", output);
    assert!(has_violation, "Should detect transitive borrow through Box");
}

#[test]
fn test_phase4_mutable_transitive_borrows() {
    let code = r#"
#include <rusty/box.hpp>

// @lifetime: (&'a mut) -> &'a mut int
int& identity_mut(int& x) { return x; }

// @safe
int main() {
    rusty::Box<int> value = rusty::Box<int>::make(42);
    int& ref1 = *value;
    int& ref2 = identity_mut(ref1);
    rusty::Box<int> moved = std::move(value);  // ERROR: mutable chain
    return 0;
}
"#;

    let (output, has_violation) = run_analyzer_on_code(code, &["include"]);
    println!("Output: {}", output);
    assert!(has_violation, "Should detect mutable transitive borrow");
}

#[test]
fn test_phase4_error_message_shows_borrowers() {
    let code = r#"
#include <rusty/box.hpp>

// @lifetime: (&'a) -> &'a int
const int& identity(const int& x) { return x; }

// @safe
int main() {
    rusty::Box<int> value = rusty::Box<int>::make(42);
    int& ref1 = *value;
    const int& ref2 = identity(ref1);
    rusty::Box<int> moved = std::move(value);  // ERROR
    return 0;
}
"#;

    let (output, has_violation) = run_analyzer_on_code(code, &["include"]);
    println!("Output: {}", output);
    assert!(has_violation, "Should have error");
    // Error message should show transitive borrowers
    assert!(output.contains("ref1") && output.contains("ref2"), 
        "Error should mention both borrowers in the chain");
}
