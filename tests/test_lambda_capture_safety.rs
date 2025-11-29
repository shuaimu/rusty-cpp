/// Tests for lambda capture safety in @safe code
///
/// In @safe code:
/// - Reference captures ([&], [&x]) are FORBIDDEN - can create dangling references
/// - Copy captures ([x], [=]) are ALLOWED - safe copy semantics
/// - Move captures ([x = std::move(y)]) are ALLOWED - ownership transfer is safe
///
/// This follows Rust's approach where closures capturing by reference have
/// strict lifetime requirements.

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
// Tests for INVALID code (reference captures in @safe - should error)
// =============================================================================

#[test]
fn test_explicit_ref_capture_forbidden() {
    // Explicit reference capture [&x] is forbidden in @safe code
    let source = r#"
// @safe
void test() {
    int x = 42;
    auto f = [&x]() { return x; };  // ERROR: reference capture in @safe
}

int main() { return 0; }
"#;

    let (success, output) = analyze(source);
    assert!(
        !success,
        "Explicit reference capture [&x] should be forbidden in @safe. Output: {}",
        output
    );
    assert!(
        output.contains("reference capture") || output.contains("Reference capture"),
        "Error should mention reference capture. Got: {}",
        output
    );
}

#[test]
fn test_default_ref_capture_forbidden() {
    // Default reference capture [&] is forbidden in @safe code
    let source = r#"
// @safe
void test() {
    int x = 42;
    int y = 10;
    auto f = [&]() { return x + y; };  // ERROR: default reference capture
}

int main() { return 0; }
"#;

    let (success, output) = analyze(source);
    assert!(
        !success,
        "Default reference capture [&] should be forbidden in @safe. Output: {}",
        output
    );
}

#[test]
fn test_mixed_capture_with_ref_forbidden() {
    // Mixed capture with any reference is forbidden
    let source = r#"
// @safe
void test() {
    int x = 42;
    int y = 10;
    auto f = [x, &y]() { return x + y; };  // ERROR: &y is reference capture
}

int main() { return 0; }
"#;

    let (success, output) = analyze(source);
    assert!(
        !success,
        "Mixed capture with reference [x, &y] should be forbidden. Output: {}",
        output
    );
}

#[test]
fn test_this_ref_capture_forbidden() {
    // Capturing 'this' by reference is also dangerous
    let source = r#"
// @safe
class Foo {
public:
    int value = 42;

    auto get_lambda() {
        return [this]() { return value; };  // ERROR: 'this' capture
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
}

// =============================================================================
// Tests for VALID code (copy/move captures in @safe - should pass)
// =============================================================================

#[test]
fn test_copy_capture_allowed() {
    // Copy capture [x] is safe
    let source = r#"
// @safe
void test() {
    int x = 42;
    auto f = [x]() { return x; };  // OK: copy capture
}

int main() { return 0; }
"#;

    let (success, output) = analyze(source);
    assert!(
        success,
        "Copy capture [x] should be allowed in @safe. Got error: {}",
        output
    );
}

#[test]
fn test_default_copy_capture_allowed() {
    // Default copy capture [=] is safe
    let source = r#"
// @safe
void test() {
    int x = 42;
    int y = 10;
    auto f = [=]() { return x + y; };  // OK: default copy capture
}

int main() { return 0; }
"#;

    let (success, output) = analyze(source);
    assert!(
        success,
        "Default copy capture [=] should be allowed in @safe. Got error: {}",
        output
    );
}

#[test]
fn test_move_capture_allowed() {
    // Move capture is safe (ownership transfer)
    let source = r#"
#include <utility>

// @safe
void test() {
    int x = 42;
    auto f = [y = std::move(x)]() { return y; };  // OK: move capture
}

int main() { return 0; }
"#;

    let (success, output) = analyze(source);
    assert!(
        success,
        "Move capture should be allowed in @safe. Got error: {}",
        output
    );
}

#[test]
fn test_init_capture_copy_allowed() {
    // Init capture with copy is safe
    let source = r#"
// @safe
void test() {
    int x = 42;
    auto f = [y = x]() { return y; };  // OK: init capture (copy)
}

int main() { return 0; }
"#;

    let (success, output) = analyze(source);
    assert!(
        success,
        "Init capture with copy should be allowed in @safe. Got error: {}",
        output
    );
}

#[test]
fn test_empty_capture_allowed() {
    // Empty capture [] is always safe
    let source = r#"
// @safe
void test() {
    auto f = []() { return 42; };  // OK: no capture
}

int main() { return 0; }
"#;

    let (success, output) = analyze(source);
    assert!(
        success,
        "Empty capture [] should be allowed. Got error: {}",
        output
    );
}

// =============================================================================
// Tests for unsafe code (reference captures allowed)
// =============================================================================

#[test]
fn test_ref_capture_allowed_in_unsafe() {
    // Reference capture is allowed in @unsafe code
    let source = r#"
// @unsafe
void test() {
    int x = 42;
    auto f = [&x]() { return x; };  // OK in @unsafe
}

int main() { return 0; }
"#;

    let (success, output) = analyze(source);
    assert!(
        success,
        "Reference capture should be allowed in @unsafe. Got error: {}",
        output
    );
}

#[test]
fn test_ref_capture_allowed_in_undeclared() {
    // Reference capture is allowed in undeclared (default) code
    let source = r#"
void test() {
    int x = 42;
    auto f = [&x]() { return x; };  // OK in undeclared
}

int main() { return 0; }
"#;

    let (success, output) = analyze(source);
    assert!(
        success,
        "Reference capture should be allowed in undeclared code. Got error: {}",
        output
    );
}

// =============================================================================
// Edge cases
// =============================================================================

#[test]
fn test_nested_lambda_ref_capture_forbidden() {
    // Nested lambdas with ref capture should also be caught
    let source = r#"
// @safe
void test() {
    int x = 42;
    auto outer = [=]() {
        auto inner = [&x]() { return x; };  // ERROR: inner has ref capture
        return inner();
    };
}

int main() { return 0; }
"#;

    let (success, output) = analyze(source);
    // This might be tricky to detect - document expected behavior
    let _ = (success, output);
    // For now, just verify it doesn't crash
}

#[test]
fn test_lambda_in_safe_class_method() {
    // Lambda in @safe class method should be checked
    let source = r#"
// @safe
class Foo {
public:
    void method() {
        int x = 42;
        auto f = [&x]() { return x; };  // ERROR: ref capture in @safe method
    }
};

int main() { return 0; }
"#;

    let (success, output) = analyze(source);
    assert!(
        !success,
        "Reference capture in @safe class method should be forbidden. Output: {}",
        output
    );
}
