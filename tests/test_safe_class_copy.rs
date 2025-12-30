/// Tests for @safe class copy semantics
///
/// @safe classes should not have copy operations (like Rust's move-by-default).
/// This enforces Rust-like ownership semantics where types are moved, not copied.

use std::process::Command;
use std::fs;

fn run_checker(code: &str) -> (bool, String) {
    run_checker_with_name(code, "test")
}

fn run_checker_with_name(code: &str, test_name: &str) -> (bool, String) {
    let temp_dir = std::env::temp_dir();
    let unique_id = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let test_file = temp_dir.join(format!("test_safe_copy_{}_{}.cpp", test_name, unique_id));

    fs::write(&test_file, code).unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_rusty-cpp-checker"))
        .arg(&test_file)
        .output()
        .expect("Failed to run checker");

    let _ = fs::remove_file(&test_file);

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let combined = stdout + &stderr;

    let success = output.status.success() &&
                  (combined.contains("no violations") || !combined.contains("error"));

    (success, combined)
}

// ============================================================================
// @safe class with copy constructor should be rejected
// ============================================================================

#[test]
fn test_safe_class_with_copy_constructor_fails() {
    let code = r#"
// @safe
class SafeBox {
    int value;
public:
    SafeBox(int v) : value(v) {}
    SafeBox(const SafeBox& other) : value(other.value) {}  // ERROR: copy constructor
};

int main() { return 0; }
"#;
    let (success, output) = run_checker(code);
    assert!(
        !success,
        "@safe class with copy constructor should be rejected. Output: {}", output
    );
    assert!(
        output.contains("copy constructor") || output.contains("cannot have"),
        "Error should mention copy constructor. Output: {}", output
    );
}

#[test]
fn test_safe_class_with_copy_assignment_fails() {
    let code = r#"
// @safe
class SafeBox {
    int value;
public:
    SafeBox(int v) : value(v) {}
    SafeBox& operator=(const SafeBox& other) {  // ERROR: copy assignment
        value = other.value;
        return *this;
    }
};

int main() { return 0; }
"#;
    let (success, output) = run_checker(code);
    assert!(
        !success,
        "@safe class with copy assignment should be rejected. Output: {}", output
    );
    assert!(
        output.contains("copy assignment") || output.contains("cannot have"),
        "Error should mention copy assignment. Output: {}", output
    );
}

// ============================================================================
// @safe class with DELETED copy operations should be allowed
// ============================================================================

#[test]
fn test_safe_class_with_deleted_copy_constructor_passes() {
    let code = r#"
// @safe
class SafeBox {
    int value;
public:
    SafeBox(int v) : value(v) {}
    SafeBox(const SafeBox&) = delete;  // OK: explicitly deleted
    SafeBox& operator=(const SafeBox&) = delete;  // OK: explicitly deleted
};

int main() { return 0; }
"#;
    let (success, output) = run_checker(code);
    assert!(
        success,
        "@safe class with deleted copy operations should be allowed. Output: {}", output
    );
}

#[test]
fn test_safe_class_with_move_only_passes() {
    let code = r#"
// @safe
class SafeBox {
    int value;
public:
    SafeBox(int v) : value(v) {}
    SafeBox(SafeBox&& other) : value(other.value) { other.value = 0; }  // OK: move
    // @lifetime: (&'a mut, SafeBox&&) -> &'a mut SafeBox
    SafeBox& operator=(SafeBox&& other) {  // OK: move assignment
        value = other.value;
        other.value = 0;
        return *this;
    }
    SafeBox(const SafeBox&) = delete;
    SafeBox& operator=(const SafeBox&) = delete;
};

int main() { return 0; }
"#;
    let (success, output) = run_checker(code);
    assert!(
        success,
        "@safe class with move-only semantics should be allowed. Output: {}", output
    );
}

// ============================================================================
// @unsafe class with copy operations should be allowed
// ============================================================================

#[test]
fn test_unsafe_class_with_copy_allowed() {
    let code = r#"
// @unsafe
class UnsafeBox {
    int value;
public:
    UnsafeBox(int v) : value(v) {}
    UnsafeBox(const UnsafeBox& other) : value(other.value) {}  // OK in @unsafe
    UnsafeBox& operator=(const UnsafeBox& other) {
        value = other.value;
        return *this;
    }
};

int main() { return 0; }
"#;
    let (success, output) = run_checker(code);
    assert!(
        success,
        "@unsafe class can have copy operations. Output: {}", output
    );
}

#[test]
fn test_unannotated_class_with_copy_allowed() {
    let code = r#"
// No annotation - defaults to @unsafe
class Box {
    int value;
public:
    Box(int v) : value(v) {}
    Box(const Box& other) : value(other.value) {}  // OK: no @safe annotation
};

int main() { return 0; }
"#;
    let (success, output) = run_checker(code);
    assert!(
        success,
        "Unannotated class can have copy operations. Output: {}", output
    );
}

// ============================================================================
// @safe class with no copy operations at all should be allowed
// ============================================================================

#[test]
fn test_safe_class_no_copy_operations_passes() {
    let code = r#"
// @safe
class SafeBox {
    int value;
public:
    SafeBox(int v) : value(v) {}
    // No copy constructor or copy assignment - implicitly deleted due to no definition
};

int main() { return 0; }
"#;
    let (success, output) = run_checker(code);
    assert!(
        success,
        "@safe class with no copy operations should be allowed. Output: {}", output
    );
}

// ============================================================================
// Both copy constructor and copy assignment detected
// ============================================================================

#[test]
fn test_safe_class_with_both_copy_operations_fails() {
    let code = r#"
// @safe
class SafeBox {
    int value;
public:
    SafeBox(int v) : value(v) {}
    SafeBox(const SafeBox& other) : value(other.value) {}  // ERROR
    SafeBox& operator=(const SafeBox& other) {  // ERROR
        value = other.value;
        return *this;
    }
};

int main() { return 0; }
"#;
    let (success, output) = run_checker(code);
    assert!(
        !success,
        "@safe class with both copy operations should be rejected. Output: {}", output
    );
    // Should mention both
    assert!(
        output.contains("copy constructor") || output.contains("copy"),
        "Error should mention copy operations. Output: {}", output
    );
}
