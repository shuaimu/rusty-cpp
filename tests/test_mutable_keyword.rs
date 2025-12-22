use std::path::Path;
use std::process::Command;
use tempfile::NamedTempFile;
use std::io::Write;
use std::env;

fn run_analyzer(cpp_file: &Path) -> (bool, String) {
    // Set Z3 header path based on platform
    let z3_header = if cfg!(target_os = "macos") {
        "/opt/homebrew/include/z3.h"
    } else {
        "/usr/include/z3.h"
    };

    let mut cmd = Command::new("cargo");
    cmd.args(&["run", "--quiet", "--", cpp_file.to_str().unwrap()])
        .env("Z3_SYS_Z3_HEADER", z3_header);

    // Set library paths based on platform
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

#[test]
fn test_mutable_in_safe_class_error() {
    let code = r#"
// @safe
class Counter {
    mutable int count;
public:
    // @safe
    void increment() const {
        count++;
    }
};
    "#;

    let temp_file = create_temp_cpp_file(code);
    let (_success, output) = run_analyzer(temp_file.path());

    assert!(output.contains("Mutable field"), "Error should mention mutable field, got: {}", output);
    assert!(output.contains("count"), "Error should mention field name, got: {}", output);
    assert!(output.contains("UnsafeCell"), "Error should suggest UnsafeCell, got: {}", output);
}

#[test]
fn test_mutable_in_unsafe_class_ok() {
    let code = r#"
// No @safe annotation - class is undeclared/unsafe
class Counter {
    mutable int count;
public:
    void increment() const {
        count++;
    }
};
    "#;

    let temp_file = create_temp_cpp_file(code);
    let (success, output) = run_analyzer(temp_file.path());

    assert!(success, "Analysis should succeed without errors, got: {}", output);
    assert!(output.contains("no violations"), "Should have no violations for undeclared class, got: {}", output);
}

#[test]
fn test_multiple_mutable_fields_in_safe_class() {
    let code = r#"
// @safe
class Cache {
    mutable int hits;
    mutable int misses;
    int total;
public:
    // @safe
    void recordHit() const {
        hits++;
    }
};
    "#;

    let temp_file = create_temp_cpp_file(code);
    let (_success, output) = run_analyzer(temp_file.path());

    assert!(output.contains("hits"), "Should report error for 'hits' field, got: {}", output);
    assert!(output.contains("misses"), "Should report error for 'misses' field, got: {}", output);
}

#[test]
fn test_non_mutable_field_ok() {
    let code = r#"
// @safe
class Counter {
    int count;  // Not mutable
public:
    // @safe
    void increment() {  // Non-const method
        count++;
    }
};
    "#;

    let temp_file = create_temp_cpp_file(code);
    let (success, output) = run_analyzer(temp_file.path());

    assert!(success, "Analysis should succeed without errors, got: {}", output);
    assert!(output.contains("no violations"), "Should have no violations for non-mutable field, got: {}", output);
}

#[test]
fn test_mutable_with_safe_class() {
    // With the new two-state model (Safe/Unsafe), mutable fields are checked
    // at the CLASS level. A @safe class should not have mutable fields.
    let code = r#"
// @safe
class Counter {
    mutable int count;
public:
    void increment() const {
        count++;
    }

    void reset() {
        count = 0;
    }
};
    "#;

    let temp_file = create_temp_cpp_file(code);
    let (_success, output) = run_analyzer(temp_file.path());

    assert!(output.contains("count"), "Error should mention field name, got: {}", output);
    assert!(output.contains("Mutable field"), "Should report mutable field error, got: {}", output);
}

#[test]
fn test_mutable_with_unsafe_class_allowed() {
    // With the new two-state model, unannotated classes are @unsafe by default.
    // Mutable fields are allowed in unsafe classes.
    let code = r#"
class Counter {
    mutable int count;
public:
    // @safe - method can be safe even in unsafe class
    void increment() const {
        count++;
    }

    void reset() {
        count = 0;
    }
};
    "#;

    let temp_file = create_temp_cpp_file(code);
    let (success, output) = run_analyzer(temp_file.path());

    // Should pass - unannotated class is @unsafe, mutable is allowed
    assert!(success, "Mutable fields should be allowed in unsafe class, got: {}", output);
}
