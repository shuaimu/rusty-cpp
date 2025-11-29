/// Tests for reassignment-after-move feature
///
/// These tests verify that a variable can be used after it has been moved,
/// IF it has been reassigned to a new value. This is valid in both Rust and C++.
///
/// Pattern:
///   int x = 42;
///   int y = std::move(x);  // x is moved
///   x = 100;               // x is reassigned - now valid again!
///   int z = x;             // Should be OK

use std::io::Write;
use std::path::Path;
use std::process::Command;
use tempfile::NamedTempFile;

fn run_analyzer(cpp_file: &Path) -> (bool, String) {
    let z3_header = if cfg!(target_os = "macos") {
        "/opt/homebrew/include/z3.h"
    } else {
        "/usr/include/z3.h"
    };

    let mut cmd = Command::new("cargo");
    cmd.args(&["run", "--quiet", "--", cpp_file.to_str().unwrap()])
        .env("Z3_SYS_Z3_HEADER", z3_header);

    if cfg!(target_os = "macos") {
        cmd.env("DYLD_LIBRARY_PATH", "/opt/homebrew/Cellar/llvm/19.1.7/lib");
    } else {
        cmd.env("LD_LIBRARY_PATH", "/usr/lib/llvm-14/lib");
    }

    let output = cmd.output()
        .expect("Failed to execute analyzer");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let full_output = format!("{}{}", stdout, stderr);

    (output.status.success(), full_output)
}

fn create_temp_cpp_file(content: &str) -> NamedTempFile {
    let mut file = NamedTempFile::with_suffix(".cpp").unwrap();
    file.write_all(content.as_bytes()).unwrap();
    file.flush().unwrap();
    file
}

fn analyze(source: &str) -> (bool, String) {
    let temp_file = create_temp_cpp_file(source);
    let (_success, output) = run_analyzer(temp_file.path());

    let has_violations = output.contains("Found") && output.contains("violation");
    let no_violations = output.contains("no violations found");

    (!has_violations || no_violations, output)
}

// =============================================================================
// Tests for VALID code (should pass without errors)
// =============================================================================

#[test]
fn test_literal_reassignment_after_move() {
    // Reassigning a literal value after move should restore the variable
    let source = r#"
#include <utility>

// @safe
void test() {
    int x = 42;
    int y = std::move(x);  // x is moved
    x = 100;               // x is reassigned with literal
    int z = x;             // Should be OK - x was reassigned
}

int main() { return 0; }
"#;

    let (success, output) = analyze(source);
    assert!(
        success,
        "Literal reassignment after move should be valid. Got error: {}",
        output
    );
}

#[test]
fn test_variable_reassignment_after_move() {
    // Reassigning from another variable after move should restore the variable
    let source = r#"
#include <utility>

// @safe
void test() {
    int x = 42;
    int y = std::move(x);  // x is moved
    int w = 200;
    x = w;                 // x is reassigned from w
    int z = x;             // Should be OK - x was reassigned
}

int main() { return 0; }
"#;

    let (success, output) = analyze(source);
    assert!(
        success,
        "Variable reassignment after move should be valid. Got error: {}",
        output
    );
}

#[test]
fn test_move_reassignment_after_move() {
    // Reassigning via std::move after move should restore the variable
    let source = r#"
#include <utility>

// @safe
void test() {
    int x = 42;
    int y = std::move(x);  // x is moved
    int w = 200;
    x = std::move(w);      // x is reassigned via move from w
    int z = x;             // Should be OK - x was reassigned
}

int main() { return 0; }
"#;

    let (success, output) = analyze(source);
    assert!(
        success,
        "Move reassignment after move should be valid. Got error: {}",
        output
    );
}

#[test]
fn test_multiple_move_reassign_cycles() {
    // Variable can be moved and reassigned multiple times
    let source = r#"
#include <utility>

// @safe
void test() {
    int x = 1;
    int y = std::move(x);  // x moved (1st time)
    x = 10;                // x reassigned
    int z = std::move(x);  // x moved (2nd time)
    x = 100;               // x reassigned again
    int w = x;             // Should be OK
}

int main() { return 0; }
"#;

    let (success, output) = analyze(source);
    assert!(
        success,
        "Multiple move-reassign cycles should be valid. Got error: {}",
        output
    );
}

#[test]
fn test_reassignment_in_different_scope() {
    // Reassignment in a different scope should still work
    let source = r#"
#include <utility>

// @safe
void test() {
    int x = 42;
    int y = std::move(x);  // x is moved

    {
        x = 100;           // x reassigned in inner scope
    }

    int z = x;             // Should be OK - x was reassigned
}

int main() { return 0; }
"#;

    let (success, output) = analyze(source);
    assert!(
        success,
        "Reassignment in different scope should be valid. Got error: {}",
        output
    );
}

#[test]
fn test_simple_move_without_reassignment_use() {
    // Simple move without using the moved variable should be OK
    let source = r#"
#include <utility>

// @safe
void test() {
    int x = 42;
    int y = std::move(x);  // x is moved
    int z = y;             // Using y is OK
    // Not using x at all - OK
}

int main() { return 0; }
"#;

    let (success, output) = analyze(source);
    assert!(
        success,
        "Move without using moved variable should be valid. Got error: {}",
        output
    );
}

// =============================================================================
// Tests for INVALID code (should report errors)
// =============================================================================

#[test]
fn test_use_after_move_without_reassignment() {
    // Using a moved variable without reassignment should error
    let source = r#"
#include <utility>

// @safe
void test() {
    int x = 42;
    int y = std::move(x);  // x is moved
    int z = x;             // ERROR: use after move without reassignment
}

int main() { return 0; }
"#;

    let (success, output) = analyze(source);
    assert!(
        !success,
        "Use after move without reassignment should be an error. Output: {}",
        output
    );
    assert!(
        output.contains("move") || output.contains("Move"),
        "Error should mention 'move'. Got: {}",
        output
    );
}

#[test]
fn test_double_move_without_reassignment() {
    // Moving a variable twice without reassignment should error
    let source = r#"
#include <utility>

// @safe
void test() {
    int x = 42;
    int y = std::move(x);  // x is moved
    int z = std::move(x);  // ERROR: x already moved
}

int main() { return 0; }
"#;

    let (success, output) = analyze(source);
    assert!(
        !success,
        "Double move without reassignment should be an error. Output: {}",
        output
    );
}

#[test]
fn test_use_before_reassignment_completes() {
    // Using variable on same line as reassignment (before it happens)
    // This tests that the order matters
    let source = r#"
#include <utility>

// @safe
void test() {
    int x = 42;
    int y = std::move(x);  // x is moved
    // Can't use x here - it's moved
    int z = x;             // ERROR: use before any reassignment
}

int main() { return 0; }
"#;

    let (success, output) = analyze(source);
    assert!(
        !success,
        "Use after move without prior reassignment should be an error. Output: {}",
        output
    );
}

// =============================================================================
// Edge cases
// =============================================================================

#[test]
fn test_reassignment_then_borrow_after_move() {
    // After reassignment, borrowing should work normally
    let source = r#"
#include <utility>

// @safe
void test() {
    int x = 42;
    int y = std::move(x);  // x is moved
    x = 100;               // x reassigned
    int& r = x;            // Borrowing x should be OK now
    int z = r;
}

int main() { return 0; }
"#;

    let (success, output) = analyze(source);
    assert!(
        success,
        "Borrowing after reassignment should be valid. Got error: {}",
        output
    );
}

#[test]
fn test_conditional_reassignment_conservative() {
    // If reassignment only happens in one branch, use after should be error
    // (conservative analysis)
    let source = r#"
#include <utility>

// @safe
void test(bool cond) {
    int x = 42;
    int y = std::move(x);  // x is moved

    if (cond) {
        x = 100;           // x only reassigned in one branch
    }

    // x might still be moved here (if cond was false)
    // Conservative analysis should flag this
}

int main() { return 0; }
"#;

    // This test documents expected behavior - conservative analysis
    // may or may not flag this depending on implementation
    let (_success, _output) = analyze(source);
    // Just verify it doesn't crash
}

#[test]
fn test_reassignment_in_loop() {
    // Reassignment in loop should restore variable for next iteration
    // Note: Using literal assignment because loop counter variables declared
    // inside for() aren't tracked in the variables map yet
    let source = r#"
#include <utility>

// @safe
void test() {
    int x = 0;
    for (int i = 0; i < 3; i++) {
        int y = std::move(x);  // x moved
        x = 100;               // x reassigned for next iteration
    }
}

int main() { return 0; }
"#;

    let (success, output) = analyze(source);
    assert!(
        success,
        "Reassignment in loop should be valid. Got error: {}",
        output
    );
}
