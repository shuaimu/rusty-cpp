/// Tests for lambda capture safety in @safe code with escape analysis
///
/// In @safe code:
/// - Reference captures ([&], [&x]) are ALLOWED if the lambda doesn't escape
/// - Reference captures that ESCAPE are FORBIDDEN - can create dangling references
/// - Copy captures ([x], [=]) are ALWAYS ALLOWED - safe copy semantics
/// - Move captures ([x = std::move(y)]) are ALWAYS ALLOWED - ownership transfer is safe
/// - 'this' capture is ALWAYS FORBIDDEN - 'this' is a raw pointer that can dangle
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
// Tests for ESCAPING lambdas with reference captures (should error)
// =============================================================================

#[test]
fn test_this_ref_capture_forbidden() {
    // Capturing 'this' by reference is always dangerous (always forbidden)
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
// Tests for NON-ESCAPING lambdas with reference captures (now ALLOWED)
// =============================================================================

#[test]
fn test_explicit_ref_capture_non_escaping_allowed() {
    // Explicit reference capture [&x] is now allowed if the lambda doesn't escape
    let source = r#"
// @safe
void test() {
    int x = 42;
    auto f = [&x]() { return x; };  // OK: non-escaping lambda
    int result = f();  // Used immediately
}

int main() { return 0; }
"#;

    let (success, output) = analyze(source);
    assert!(
        success,
        "Non-escaping reference capture [&x] should be allowed in @safe. Output: {}",
        output
    );
}

#[test]
fn test_default_ref_capture_non_escaping_allowed() {
    // Default reference capture [&] is now allowed if the lambda doesn't escape
    let source = r#"
// @safe
void test() {
    int x = 42;
    int y = 10;
    auto f = [&]() { return x + y; };  // OK: non-escaping lambda
    int result = f();  // Used immediately
}

int main() { return 0; }
"#;

    let (success, output) = analyze(source);
    assert!(
        success,
        "Non-escaping default reference capture [&] should be allowed in @safe. Output: {}",
        output
    );
}

#[test]
fn test_mixed_capture_non_escaping_allowed() {
    // Mixed capture with reference is now allowed if lambda doesn't escape
    let source = r#"
// @safe
void test() {
    int x = 42;
    int y = 10;
    auto f = [x, &y]() { return x + y; };  // OK: non-escaping
    int result = f();  // Used immediately
}

int main() { return 0; }
"#;

    let (success, output) = analyze(source);
    assert!(
        success,
        "Non-escaping mixed capture [x, &y] should be allowed in @safe. Output: {}",
        output
    );
}

#[test]
fn test_lambda_in_safe_class_method_non_escaping() {
    // Lambda in @safe class method - non-escaping is now allowed
    let source = r#"
// @safe
class Foo {
public:
    void method() {
        int x = 42;
        auto f = [&x]() { return x; };  // OK: non-escaping
        int result = f();  // Used immediately
    }
};

int main() { return 0; }
"#;

    let (success, output) = analyze(source);
    assert!(
        success,
        "Non-escaping reference capture in @safe class method should be allowed. Output: {}",
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
fn test_nested_lambda_ref_capture() {
    // Nested lambdas with ref capture - verify it doesn't crash
    let source = r#"
// @safe
void test() {
    int x = 42;
    auto outer = [=]() {
        auto inner = [&x]() { return x; };  // Inner has ref capture
        return inner();
    };
}

int main() { return 0; }
"#;

    let (_success, _output) = analyze(source);
    // Just verify it doesn't crash
}
