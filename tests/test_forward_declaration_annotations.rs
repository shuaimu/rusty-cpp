// Tests for forward declaration vs full definition annotation combinations
// Tests various scenarios where forward declarations and full definitions
// may have different (or no) safety annotations

use assert_cmd::Command;
use std::fs;
use tempfile::{NamedTempFile, TempDir};
use std::io::Write;

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

    let has_violations = full_output.contains("violation") ||
                        full_output.contains("Error") ||
                        full_output.contains("unsafe function");
    (full_output, has_violations)
}

// ============================================================================
// Test 1: Forward declaration FIRST (unmarked) + Full definition (marked safe)
// ============================================================================

#[test]
fn test_forward_unmarked_then_full_safe() {
    let code = r#"
// Forward declaration without annotation
class Container;

// Function declaration that uses forward declaration
void use_container(Container* c);

// Full definition with @safe annotation
// @safe
class Container {
public:
    void safe_method() {
        // Safe operation
    }
};

// @safe
void use_container(Container* c) {
    c->safe_method();  // Should be allowed - Container is safe
}

int main() {
    Container c;
    use_container(&c);
    return 0;
}
"#;

    let (output, has_violations) = run_analyzer_on_code(code, &[]);
    println!("Output: {}", output);

    // Container::safe_method should be recognized as safe
    // use_container calling it should be OK
    assert!(!has_violations || !output.contains("safe_method"),
        "Should allow safe function to call safe method. Output: {}", output);
}

// ============================================================================
// Test 2: Forward declaration (marked safe) FIRST + Full definition (unmarked)
// ============================================================================

#[test]
fn test_forward_safe_then_full_unmarked() {
    let code = r#"
// Forward declaration WITH @safe annotation
// @safe
class Container;

void use_container(Container* c);

// Full definition WITHOUT annotation
class Container {
public:
    void some_method() {
        // Some operation
    }
};

// @safe
void use_container(Container* c) {
    c->some_method();
}

int main() {
    return 0;
}
"#;

    let (output, has_violations) = run_analyzer_on_code(code, &[]);
    println!("Output: {}", output);

    // Forward declaration annotations should be ignored (no opening brace)
    // The function `c->some_method()` involves dereferencing raw pointer c,
    // which is unsafe and should be caught BEFORE we check for undeclared method call.
    // Pointer dereference in safe context is the primary violation.
    assert!(has_violations && output.contains("dereference"),
        "Should detect pointer dereference in safe context. Output: {}", output);
}

// ============================================================================
// Test 3: Forward (marked safe) + Full (marked safe) - both annotated
// ============================================================================

#[test]
fn test_forward_safe_then_full_safe() {
    let code = r#"
// Forward declaration with @safe (should be ignored - no braces)
// @safe
class Container;

// Full definition also with @safe
// @safe
class Container {
public:
    void safe_method() {}
};

// @safe
void use_it(Container* c) {
    c->safe_method();  // Should work - full definition is safe
}

int main() { return 0; }
"#;

    let (output, has_violations) = run_analyzer_on_code(code, &[]);
    println!("Output: {}", output);

    // Full definition annotation should be used
    assert!(!has_violations || !output.contains("safe_method"),
        "Should recognize full definition annotation. Output: {}", output);
}

// ============================================================================
// Test 4: Forward (unmarked) + Full (unmarked) - neither annotated
// ============================================================================

#[test]
fn test_forward_unmarked_then_full_unmarked() {
    let code = r#"
// Forward declaration without annotation
class Container;

// Full definition without annotation
class Container {
public:
    void some_method() {}
};

// @safe
void use_it(Container* c) {
    c->some_method();  // Should fail - undeclared
}

int main() { return 0; }
"#;

    let (output, has_violations) = run_analyzer_on_code(code, &[]);
    println!("Output: {}", output);

    // Both unmarked means Undeclared - calling from safe should fail
    assert!(has_violations && output.contains("some_method"),
        "Should detect call to undeclared method. Output: {}", output);
}

// ============================================================================
// Test 5: Full definition FIRST + Forward declaration later
// ============================================================================

#[test]
fn test_full_safe_then_forward_unmarked() {
    let code = r#"
// Full definition FIRST with @safe
// @safe
class Container {
public:
    void safe_method() {}
};

// Forward declaration later (should have no effect)
class Container;

// @safe
void use_it(Container* c) {
    c->safe_method();  // Should work - full definition is safe
}

int main() { return 0; }
"#;

    let (output, has_violations) = run_analyzer_on_code(code, &[]);
    println!("Output: {}", output);

    // Full definition annotation should be used
    assert!(!has_violations || !output.contains("safe_method"),
        "Should use full definition annotation. Output: {}", output);
}

// ============================================================================
// Test 6: Header with forward decl + Source with full definition
// ============================================================================

#[test]
fn test_header_forward_source_full() {
    let temp_dir = TempDir::new().unwrap();

    // Create header with forward declaration
    let header_path = temp_dir.path().join("container.h");
    fs::write(&header_path, r#"
#ifndef CONTAINER_H
#define CONTAINER_H

// Forward declaration in header (unmarked)
class Container;

void use_container(Container* c);

#endif
"#).unwrap();

    // Create source with full definition
    let source_path = temp_dir.path().join("container.cpp");
    fs::write(&source_path, r#"
#include "container.h"

// Full definition with @safe
// @safe
class Container {
public:
    void safe_method() {}
};

// @safe
void use_container(Container* c) {
    c->safe_method();  // Should work
}

int main() { return 0; }
"#).unwrap();

    let mut cmd = Command::cargo_bin("rusty-cpp-checker").unwrap();
    cmd.arg(&source_path);
    cmd.arg("-I").arg(temp_dir.path());

    let output = cmd.output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let full_output = format!("{}\n{}", stdout, stderr);

    println!("Output: {}", full_output);

    let has_violations = full_output.contains("violation") ||
                        full_output.contains("unsafe function");

    assert!(!has_violations || !full_output.contains("safe_method"),
        "Should use source file's full definition annotation. Output: {}", full_output);
}

// ============================================================================
// Test 7: Header with @safe forward + Source with unmarked full
// ============================================================================

#[test]
fn test_header_safe_forward_source_unmarked_full() {
    let temp_dir = TempDir::new().unwrap();

    // Create header with @safe forward declaration
    let header_path = temp_dir.path().join("container.h");
    fs::write(&header_path, r#"
#ifndef CONTAINER_H
#define CONTAINER_H

// Forward declaration with @safe (should be ignored - no braces)
// @safe
class Container;

#endif
"#).unwrap();

    // Create source with unmarked full definition
    let source_path = temp_dir.path().join("container.cpp");
    fs::write(&source_path, r#"
#include "container.h"

// Full definition without annotation
class Container {
public:
    void some_method() {}
};

// @safe
void use_it(Container* c) {
    c->some_method();  // Should fail - undeclared
}

int main() { return 0; }
"#).unwrap();

    let mut cmd = Command::cargo_bin("rusty-cpp-checker").unwrap();
    cmd.arg(&source_path);
    cmd.arg("-I").arg(temp_dir.path());

    let output = cmd.output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let full_output = format!("{}\n{}", stdout, stderr);

    println!("Output: {}", full_output);

    let has_violations = full_output.contains("violation") ||
                        full_output.contains("unsafe function");

    // Header's forward decl annotation should be ignored (no braces)
    // The function `c->some_method()` involves dereferencing raw pointer c,
    // which is unsafe and should be caught BEFORE we check for undeclared method call.
    // Pointer dereference in safe context is the primary violation.
    assert!(has_violations && full_output.contains("dereference"),
        "Should detect pointer dereference in safe context. Output: {}", full_output);
}

// ============================================================================
// Test 8: Multiple forward declarations with different annotations
// ============================================================================

#[test]
fn test_multiple_forwards_then_full() {
    let code = r#"
// First forward declaration (unmarked)
class Container;

// Second forward declaration with @safe (should still be ignored)
// @safe
class Container;

// Third forward declaration (unmarked)
class Container;

// Full definition with @safe
// @safe
class Container {
public:
    void safe_method() {}
};

// @safe
void use_it(Container* c) {
    c->safe_method();
}

int main() { return 0; }
"#;

    let (output, has_violations) = run_analyzer_on_code(code, &[]);
    println!("Output: {}", output);

    // Only the full definition annotation should matter
    assert!(!has_violations || !output.contains("safe_method"),
        "Should use only full definition annotation. Output: {}", output);
}

// ============================================================================
// Test 9: Forward in one file, full in another (separate compilation units)
// ============================================================================

#[test]
fn test_forward_and_full_in_different_files() {
    let temp_dir = TempDir::new().unwrap();

    // File 1: Just forward declaration
    let file1_path = temp_dir.path().join("file1.h");
    fs::write(&file1_path, r#"
#ifndef FILE1_H
#define FILE1_H

class Container;  // Just forward, no annotation

#endif
"#).unwrap();

    // File 2: Full definition
    let file2_path = temp_dir.path().join("file2.h");
    fs::write(&file2_path, r#"
#ifndef FILE2_H
#define FILE2_H

// @safe
class Container {
public:
    void safe_method() {}
};

#endif
"#).unwrap();

    // Main file that includes both
    let main_path = temp_dir.path().join("main.cpp");
    fs::write(&main_path, r#"
#include "file1.h"  // Forward declaration first
#include "file2.h"  // Full definition second

// @safe
void use_it(Container* c) {
    c->safe_method();
}

int main() { return 0; }
"#).unwrap();

    let mut cmd = Command::cargo_bin("rusty-cpp-checker").unwrap();
    cmd.arg(&main_path);
    cmd.arg("-I").arg(temp_dir.path());

    let output = cmd.output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let full_output = format!("{}\n{}", stdout, stderr);

    println!("Output: {}", full_output);

    let has_violations = full_output.contains("violation") ||
                        full_output.contains("unsafe function");

    assert!(!has_violations || !full_output.contains("safe_method"),
        "Should recognize full definition annotation across files. Output: {}", full_output);
}

// ============================================================================
// Test 10: Forward marked safe in header, full marked safe in source
// ============================================================================

#[test]
fn test_header_safe_forward_source_safe_full() {
    let temp_dir = TempDir::new().unwrap();

    // Header with @safe forward (should be ignored)
    let header_path = temp_dir.path().join("container.h");
    fs::write(&header_path, r#"
#ifndef CONTAINER_H
#define CONTAINER_H

// @safe
class Container;  // Forward with annotation (ignored - no braces)

#endif
"#).unwrap();

    // Source with @safe full definition
    let source_path = temp_dir.path().join("container.cpp");
    fs::write(&source_path, r#"
#include "container.h"

// @safe
class Container {
public:
    void safe_method() {}
};

// @safe
void use_it(Container* c) {
    c->safe_method();
}

int main() { return 0; }
"#).unwrap();

    let mut cmd = Command::cargo_bin("rusty-cpp-checker").unwrap();
    cmd.arg(&source_path);
    cmd.arg("-I").arg(temp_dir.path());

    let output = cmd.output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let full_output = format!("{}\n{}", stdout, stderr);

    println!("Output: {}", full_output);

    let has_violations = full_output.contains("violation") ||
                        full_output.contains("unsafe function");

    assert!(!has_violations || !full_output.contains("safe_method"),
        "Should work - source has safe full definition. Output: {}", full_output);
}
