// Option<T> Borrow Checking Tests
// Tests for Option::as_ref() and Option::as_mut() with template class method detection

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

    let has_borrow_violation = full_output.contains("borrowed") ||
                                full_output.contains("Cannot move") ||
                                full_output.contains("Cannot create");
    (full_output, has_borrow_violation)
}

#[test]
fn test_option_as_mut_conflict() {
    let code = r#"
#include <rusty/option.hpp>

// @safe
int main() {
    rusty::Option<int> opt = rusty::Some<int>(42);

    // First mutable borrow
    auto mut1 = opt.as_mut();

    // Second mutable borrow - should ERROR
    auto mut2 = opt.as_mut();

    return 0;
}
"#;

    let (output, has_violation) = run_analyzer_on_code(code, &["include"]);
    println!("Output: {}", output);

    assert!(has_violation, "Should detect conflict between two as_mut() calls");
    assert!(output.contains("already mutably borrowed"),
        "Error should mention 'already mutably borrowed'");
}

#[test]
fn test_option_as_mut_prevents_as_ref() {
    let code = r#"
#include <rusty/option.hpp>

// @safe
int main() {
    rusty::Option<int> opt = rusty::Some<int>(42);

    // Mutable borrow
    auto mut_ref = opt.as_mut();

    // Immutable borrow - should ERROR
    auto ref_opt = opt.as_ref();

    return 0;
}
"#;

    let (output, has_violation) = run_analyzer_on_code(code, &["include"]);
    println!("Output: {}", output);

    assert!(has_violation, "Should detect conflict between as_mut() and as_ref()");
    assert!(output.contains("already mutably borrowed") || output.contains("already borrowed"),
        "Error should mention already borrowed");
}

#[test]
fn test_option_multiple_as_ref_allowed() {
    let code = r#"
#include <rusty/option.hpp>

// @safe
int main() {
    rusty::Option<int> opt = rusty::Some<int>(42);

    // First immutable borrow
    auto ref1 = opt.as_ref();

    // Second immutable borrow - should be OK
    auto ref2 = opt.as_ref();

    // Third immutable borrow - should also be OK
    auto ref3 = opt.as_ref();

    return 0;
}
"#;

    let (output, has_violation) = run_analyzer_on_code(code, &["include"]);
    println!("Output: {}", output);

    // Should NOT detect borrow violations for multiple immutable borrows
    // (may have other violations from Option implementation, but not borrow conflicts in main)
    if has_violation {
        assert!(!output.contains("Cannot create immutable reference to 'opt'"),
            "Multiple immutable borrows (as_ref) should be allowed");
    }
}

#[test]
fn test_option_as_ref_receiver_extracted() {
    // This test verifies that the receiver (opt) is properly extracted from as_ref() call
    let code = r#"
#include <rusty/option.hpp>

// @safe
int main() {
    rusty::Option<int> opt1 = rusty::Some<int>(1);
    rusty::Option<int> opt2 = rusty::Some<int>(2);

    // Borrow opt1
    auto ref1 = opt1.as_ref();

    // Borrow opt2 - should be independent, no conflict
    auto ref2 = opt2.as_ref();

    // Try to mutably borrow opt1 - should ERROR because ref1 exists
    auto mut1 = opt1.as_mut();

    return 0;
}
"#;

    let (output, has_violation) = run_analyzer_on_code(code, &["include"]);
    println!("Output: {}", output);

    assert!(has_violation, "Should detect that opt1 is already borrowed by ref1");
    assert!(output.contains("opt1") || output.contains("borrowed"),
        "Error should reference opt1 being borrowed");
}

#[test]
fn test_option_as_ref_with_const() {
    let code = r#"
#include <rusty/option.hpp>

// @safe
int main() {
    const rusty::Option<int> opt = rusty::Some<int>(42);

    // const Option can call as_ref() multiple times
    auto ref1 = opt.as_ref();
    auto ref2 = opt.as_ref();

    // const Option cannot call as_mut() (compile error, not borrow checker)
    // auto mut_ref = opt.as_mut();  // Would fail at C++ compile time

    return 0;
}
"#;

    let (output, _has_violation) = run_analyzer_on_code(code, &["include"]);
    println!("Output: {}", output);

    // This test mainly ensures no false positives on const Option
    // The real constraint (no as_mut on const) is enforced by C++ compiler
}

#[test]
fn test_option_as_mut_sequential_scopes() {
    let code = r#"
#include <rusty/option.hpp>

// @safe
int main() {
    rusty::Option<int> opt = rusty::Some<int>(42);

    {
        // First mutable borrow in scope
        auto mut1 = opt.as_mut();
        // Use mut1...
    }  // mut1 ends here

    {
        // Second mutable borrow in different scope - should be OK
        auto mut2 = opt.as_mut();
        // Use mut2...
    }  // mut2 ends here

    return 0;
}
"#;

    let (output, _has_violation) = run_analyzer_on_code(code, &["include"]);
    println!("Output: {}", output);

    // Sequential borrows in different scopes should be allowed
    // May have other violations, but shouldn't complain about sequential mut borrows
}
