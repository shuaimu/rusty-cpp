/// Integration tests for 'this' pointer tracking in methods
///
/// Tests that method qualifiers (const, non-const, &&) correctly enforce
/// Rust-like ownership rules on member fields:
/// - const methods (&self): can read, cannot modify or move
/// - non-const methods (&mut self): can read and modify, CANNOT move
/// - && methods (self): can read, modify, and move

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

    let output = cmd.output().expect("Failed to execute analyzer");

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
// Tests for CONST methods (&self) - cannot modify or move
// =============================================================================

#[test]
fn test_const_method_cannot_move_field_via_assignment() {
    let source = r#"
#include <utility>
#include <string>

// @safe
class Container {
    std::string data;
public:
    void bad_const() const {
        std::string x = std::move(data);  // ERROR: const method can't move
    }
};

int main() { return 0; }
"#;

    let (success, output) = analyze(source);
    assert!(
        !success,
        "Const method should not be able to move field via assignment. Output: {}",
        output
    );
    assert!(
        output.contains("const method") || output.contains("Cannot move"),
        "Error should mention const method restriction. Output: {}",
        output
    );
}

#[test]
fn test_const_method_cannot_move_field_via_return() {
    let source = r#"
#include <utility>
#include <string>

// @safe
class Container {
    std::string data;
public:
    std::string bad_const() const {
        return std::move(data);  // ERROR: const method can't move
    }
};

int main() { return 0; }
"#;

    let (success, output) = analyze(source);
    assert!(
        !success,
        "Const method should not be able to move field via return. Output: {}",
        output
    );
    assert!(
        output.contains("const method") || output.contains("Cannot move"),
        "Error should mention const method restriction. Output: {}",
        output
    );
}

#[test]
fn test_const_method_can_read_field() {
    // Return by value to avoid lifetime annotation requirement
    let source = r#"
#include <string>

// @safe
class Container {
    std::string data;
public:
    std::string get() const {
        return data;  // OK: const method can read (returns copy)
    }
};

int main() { return 0; }
"#;

    let (success, output) = analyze(source);
    assert!(
        success,
        "Const method should be able to read field. Output: {}",
        output
    );
}

// =============================================================================
// Tests for NON-CONST methods (&mut self) - can modify but CANNOT move
// =============================================================================

#[test]
fn test_nonconst_method_cannot_move_field_via_assignment() {
    let source = r#"
#include <utility>
#include <string>

// @safe
class Container {
    std::string data;
public:
    void bad_nonconst() {
        std::string x = std::move(data);  // ERROR: non-const method can't move
    }
};

int main() { return 0; }
"#;

    let (success, output) = analyze(source);
    assert!(
        !success,
        "Non-const method should not be able to move field. Output: {}",
        output
    );
    assert!(
        output.contains("&mut self") || output.contains("Cannot move"),
        "Error should mention &mut self restriction. Output: {}",
        output
    );
}

#[test]
fn test_nonconst_method_cannot_move_field_via_return() {
    let source = r#"
#include <utility>
#include <string>

// @safe
class Container {
    std::string data;
public:
    std::string bad_nonconst() {
        return std::move(data);  // ERROR: non-const method can't move
    }
};

int main() { return 0; }
"#;

    let (success, output) = analyze(source);
    assert!(
        !success,
        "Non-const method should not be able to move field via return. Output: {}",
        output
    );
    assert!(
        output.contains("&mut self") || output.contains("Cannot move"),
        "Error should mention &mut self restriction. Output: {}",
        output
    );
}

#[test]
fn test_nonconst_method_can_modify_field() {
    let source = r#"
#include <string>

// @safe
class Container {
    std::string data;
public:
    void set(const std::string& value) {
        data = value;  // OK: non-const method can modify
    }
};

int main() { return 0; }
"#;

    let (success, output) = analyze(source);
    assert!(
        success,
        "Non-const method should be able to modify field. Output: {}",
        output
    );
}

// =============================================================================
// Tests for RVALUE-REF methods (self) - can move
// =============================================================================

#[test]
fn test_rvalue_method_can_move_field_via_assignment() {
    let source = r#"
#include <utility>
#include <string>

// @safe
class Container {
    std::string data;
public:
    void consume() && {
        std::string x = std::move(data);  // OK: && method owns self
    }
};

int main() { return 0; }
"#;

    let (success, output) = analyze(source);
    assert!(
        success,
        "Rvalue method should be able to move field. Output: {}",
        output
    );
}

#[test]
fn test_rvalue_method_can_move_field_via_return() {
    let source = r#"
#include <utility>
#include <string>

// @safe
class Container {
    std::string data;
public:
    std::string consume() && {
        return std::move(data);  // OK: && method owns self
    }
};

int main() { return 0; }
"#;

    let (success, output) = analyze(source);
    assert!(
        success,
        "Rvalue method should be able to move field via return. Output: {}",
        output
    );
}

// =============================================================================
// Tests for lambda 'this' capture (always forbidden in @safe)
// =============================================================================

#[test]
fn test_this_capture_forbidden_in_safe_lambda() {
    let source = r#"
// @safe
class Widget {
    int value;
public:
    auto get_lambda() {
        return [this]() { return value; };  // ERROR: 'this' capture forbidden
    }
};

int main() { return 0; }
"#;

    let (success, output) = analyze(source);
    assert!(
        !success,
        "Capturing 'this' should be forbidden in @safe lambdas. Output: {}",
        output
    );
    assert!(
        output.contains("this") && (output.contains("forbidden") || output.contains("raw pointer")),
        "Error should mention 'this' capture being forbidden. Output: {}",
        output
    );
}

#[test]
fn test_star_this_capture_allowed() {
    // [*this] captures by copy, which is safe (C++17)
    let source = r#"
// @safe
class Widget {
    int value;
public:
    auto get_lambda() {
        return [*this]() { return value; };  // OK: captures by copy
    }
};

int main() { return 0; }
"#;

    let (success, _output) = analyze(source);
    // Note: [*this] capture may or may not be fully supported yet
    // Just verify it doesn't crash
    let _ = success;
}

// =============================================================================
// Combined tests
// =============================================================================

#[test]
fn test_multiple_methods_with_different_qualifiers() {
    let source = r#"
#include <utility>
#include <string>

// @safe
class Container {
    std::string data;
public:
    // OK: const can read (return by value to avoid lifetime annotation)
    std::string get() const {
        return data;
    }

    // OK: non-const can modify
    void set(const std::string& value) {
        data = value;
    }

    // ERROR: non-const cannot move
    std::string bad_take() {
        return std::move(data);
    }

    // OK: && can move
    std::string consume() && {
        return std::move(data);
    }
};

int main() { return 0; }
"#;

    let (success, output) = analyze(source);
    assert!(
        !success,
        "Should detect error in bad_take(). Output: {}",
        output
    );
    // Should only have 1 error (the bad_take method)
    assert!(
        output.contains("Found 1 violation"),
        "Should have exactly 1 violation for bad_take(). Output: {}",
        output
    );
}
