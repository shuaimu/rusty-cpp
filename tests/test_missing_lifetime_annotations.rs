/// Tests for detecting missing lifetime annotations in safe functions
///
/// These tests verify that:
/// 1. Safe functions returning references must have lifetime annotations
/// 2. Safe functions calling other safe functions validate lifetime constraints
/// 3. Dangling references are detected even without explicit borrows

use std::path::Path;
use std::process::Command;
use tempfile::{NamedTempFile, TempDir};
use std::io::Write;
use std::fs;

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

#[test]
fn test_safe_function_returning_reference_needs_annotation() {
    let code = r#"
// @safe
const int& return_ref(const int& x) {
    // Missing @lifetime annotation!
    return x;
}

int main() {
    return 0;
}
"#;

    let temp_file = create_temp_cpp_file(code);
    let (success, output) = run_analyzer(temp_file.path());

    assert!(!success, "Should fail - safe function returning reference without lifetime annotation");
    assert!(output.contains("lifetime") || output.contains("annotation"),
            "Should report missing lifetime annotation. Output: {}", output);
}

#[test]
fn test_dangling_reference_direct_local_return() {
    let code = r#"
// @safe
const int& return_local() {
    int local = 42;
    return local;  // Dangling reference!
}

int main() {
    return 0;
}
"#;

    let temp_file = create_temp_cpp_file(code);
    let (success, output) = run_analyzer(temp_file.path());

    assert!(!success, "Should fail - returning reference to local variable");
    assert!(output.contains("dangling") || output.contains("local"),
            "Should detect dangling reference. Output: {}", output);
}

#[test]
fn test_safe_calling_safe_without_annotation_in_header() {
    let temp_dir = TempDir::new().unwrap();

    // Create header WITHOUT lifetime annotation
    let header_content = r#"
#ifndef TEST_H
#define TEST_H

// @safe - but missing @lifetime annotation!
const int& get_ref(const int& x);

#endif
"#;

    let header_path = temp_dir.path().join("test.h");
    fs::write(&header_path, header_content).unwrap();

    // Create cpp file that calls it
    let cpp_content = r#"
#include "test.h"

// @safe
const int& get_ref(const int& x) {
    return x;
}

// @safe
void caller() {
    int value = 42;
    const int& ref = get_ref(value);  // Should error - no lifetime annotation!
}

int main() {
    return 0;
}
"#;

    let cpp_path = temp_dir.path().join("test.cpp");
    fs::write(&cpp_path, cpp_content).unwrap();

    let (success, output) = run_analyzer(&cpp_path);

    assert!(!success, "Should fail - safe function calling safe function without lifetime annotation");
    assert!(output.contains("lifetime") || output.contains("annotation"),
            "Should require lifetime annotation. Output: {}", output);
}

#[test]
fn test_safe_calling_safe_with_correct_annotation() {
    let temp_dir = TempDir::new().unwrap();

    // Create header WITH proper lifetime annotation
    let header_content = r#"
#ifndef TEST_H
#define TEST_H

// @lifetime: (&'a) -> &'a
// @safe
const int& identity(const int& x);

#endif
"#;

    let header_path = temp_dir.path().join("test.h");
    fs::write(&header_path, header_content).unwrap();

    // Create cpp file that calls it
    let cpp_content = r#"
#include "test.h"

// @safe
const int& identity(const int& x) {
    return x;
}

// @safe
void caller() {
    int value = 42;
    const int& ref = identity(value);  // OK - has lifetime annotation
}

int main() {
    return 0;
}
"#;

    let cpp_path = temp_dir.path().join("test.cpp");
    fs::write(&cpp_path, cpp_content).unwrap();

    let (success, output) = run_analyzer(&cpp_path);

    assert!(success, "Should pass - proper lifetime annotation provided. Output: {}", output);
}

#[test]
fn test_dangling_reference_through_function_call() {
    let temp_dir = TempDir::new().unwrap();

    // Create header with WRONG lifetime annotation
    let header_content = r#"
#ifndef TEST_H
#define TEST_H

// @lifetime: () -> &'static  // WRONG - claims to return static reference
// @safe
const int& get_dangling();

#endif
"#;

    let header_path = temp_dir.path().join("test.h");
    fs::write(&header_path, header_content).unwrap();

    let cpp_content = r#"
#include "test.h"

// @safe
const int& get_dangling() {
    int local = 42;
    return local;  // Implementation violates the contract!
}

// @safe
void caller() {
    const int& ref = get_dangling();
    int value = ref;  // Use after free!
}

int main() {
    return 0;
}
"#;

    let cpp_path = temp_dir.path().join("test.cpp");
    fs::write(&cpp_path, cpp_content).unwrap();

    let (success, output) = run_analyzer(&cpp_path);

    assert!(!success, "Should fail - implementation returns dangling reference");
    assert!(output.contains("dangling") || output.contains("local"),
            "Should detect implementation violates lifetime contract. Output: {}", output);
}

#[test]
fn test_lifetime_constraint_violation() {
    let temp_dir = TempDir::new().unwrap();

    // Create header with lifetime constraint
    let header_content = r#"
#ifndef TEST_H
#define TEST_H

// @lifetime: (&'a, &'b) -> &'a where 'a: 'b
// @safe
const int& select_first(const int& a, const int& b);

#endif
"#;

    let header_path = temp_dir.path().join("test.h");
    fs::write(&header_path, header_content).unwrap();

    let cpp_content = r#"
#include "test.h"

// @safe
const int& select_first(const int& a, const int& b) {
    return a;  // OK - returns first parameter
}

// @safe
void caller() {
    int long_lived = 42;
    {
        int short_lived = 10;
        // This should error - 'short_lived lifetime < 'long_lived lifetime
        // but constraint requires 'a: 'b (longer: shorter)
        const int& ref = select_first(short_lived, long_lived);  // WRONG
    }
}

int main() {
    return 0;
}
"#;

    let cpp_path = temp_dir.path().join("test.cpp");
    fs::write(&cpp_path, cpp_content).unwrap();

    let (success, output) = run_analyzer(&cpp_path);

    assert!(!success, "Should fail - lifetime constraint violated");
    assert!(output.contains("lifetime") || output.contains("constraint"),
            "Should detect lifetime constraint violation. Output: {}", output);
}

#[test]
fn test_returning_reference_to_temporary() {
    let code = r#"
// @safe
const int& get_ref() {
    return 42;  // Temporary value - dangling reference!
}

int main() {
    return 0;
}
"#;

    let temp_file = create_temp_cpp_file(code);
    let (success, output) = run_analyzer(temp_file.path());

    assert!(!success, "Should fail - returning reference to temporary");
    assert!(output.contains("temporary") || output.contains("dangling"),
            "Should detect reference to temporary. Output: {}", output);
}

#[test]
fn test_owned_return_should_not_need_lifetime() {
    let code = r#"
// @safe
int get_value() {
    int local = 42;
    return local;  // OK - returning by value
}

int main() {
    return 0;
}
"#;

    let temp_file = create_temp_cpp_file(code);
    let (success, output) = run_analyzer(temp_file.path());

    assert!(success, "Should pass - owned return doesn't need lifetime annotation. Output: {}", output);
}

#[test]
fn test_multiple_lifetime_parameters() {
    let temp_dir = TempDir::new().unwrap();

    let header_content = r#"
#ifndef TEST_H
#define TEST_H

// @lifetime: (&'a, &'a) -> &'a
// @safe
const int& max_value(const int& a, const int& b);

#endif
"#;

    let header_path = temp_dir.path().join("test.h");
    fs::write(&header_path, header_content).unwrap();

    let cpp_content = r#"
#include "test.h"

// @safe
const int& max_value(const int& a, const int& b) {
    return (a > b) ? a : b;
}

// @safe
void caller() {
    int x = 10;
    {
        int y = 20;
        const int& result = max_value(x, y);  // result's lifetime tied to both x and y
        // y goes out of scope here
    }
    // result would be dangling if it referred to y!
}

int main() {
    return 0;
}
"#;

    let cpp_path = temp_dir.path().join("test.cpp");
    fs::write(&cpp_path, cpp_content).unwrap();

    let (success, output) = run_analyzer(&cpp_path);

    // This should ideally detect that result might outlive one of its sources
    // For now, we just document the expected behavior
    println!("Multiple lifetime parameters test output: {}", output);
}

// ===== Additional comprehensive tests =====

#[test]
fn test_returning_reference_parameter_with_annotation() {
    let temp_dir = TempDir::new().unwrap();

    let header_content = r#"
#ifndef TEST_H
#define TEST_H

// @lifetime: (&'a) -> &'a
// @safe
const int& identity(const int& x);

#endif
"#;

    let header_path = temp_dir.path().join("test.h");
    fs::write(&header_path, header_content).unwrap();

    let cpp_content = r#"
#include "test.h"

// @safe
const int& identity(const int& x) {
    return x;  // OK - returning parameter
}

int main() {
    int value = 42;
    const int& ref = identity(value);
    return 0;
}
"#;

    let cpp_path = temp_dir.path().join("test.cpp");
    fs::write(&cpp_path, cpp_content).unwrap();

    let (success, output) = run_analyzer(&cpp_path);

    assert!(success, "Should pass - correctly annotated parameter return. Output: {}", output);
}

#[test]
fn test_multiple_returns_one_local() {
    let code = r#"
// @safe
const int& conditional_return(bool flag, const int& param) {
    int local = 42;
    if (flag) {
        return param;  // OK
    } else {
        return local;  // ERROR - returning local
    }
}

int main() {
    return 0;
}
"#;

    let temp_file = create_temp_cpp_file(code);
    let (success, output) = run_analyzer(temp_file.path());

    assert!(!success, "Should fail - one path returns local variable");
    assert!(output.contains("local") || output.contains("dangling") || output.contains("annotation"),
            "Should detect issue with local return. Output: {}", output);
}

#[test]
fn test_nested_function_calls_without_annotations() {
    let code = r#"
// @safe
const int& get_ref(const int& x) {
    return x;
}

// @safe
const int& get_nested(const int& x) {
    return get_ref(x);  // Calls another safe function without annotation
}

int main() {
    return 0;
}
"#;

    let temp_file = create_temp_cpp_file(code);
    let (success, output) = run_analyzer(temp_file.path());

    assert!(!success, "Should fail - functions returning references need annotations");
    assert!(output.contains("annotation"),
            "Should require lifetime annotations. Output: {}", output);
}

#[test]
fn test_returning_field_reference() {
    let code = r#"
struct Data {
    int value;
};

// @safe
const int& get_value(const Data& d) {
    return d.value;  // Returning reference to field
}

int main() {
    return 0;
}
"#;

    let temp_file = create_temp_cpp_file(code);
    let (success, output) = run_analyzer(temp_file.path());

    // This should require annotation even though it's returning a field
    assert!(!success || output.contains("annotation"),
            "Should require lifetime annotation for field reference. Output: {}", output);
}

#[test]
fn test_safe_and_unsafe_mixed() {
    let code = r#"
// @unsafe
const int& unsafe_return_local() {
    int local = 42;
    return local;  // OK in unsafe - no checking
}

// @safe
const int& safe_return_local() {
    int local = 42;
    return local;  // ERROR in safe
}

int main() {
    return 0;
}
"#;

    let temp_file = create_temp_cpp_file(code);
    let (success, output) = run_analyzer(temp_file.path());

    assert!(!success, "Should fail - safe function returns local");
    // Should only report error for the safe function, not unsafe
    assert!(output.contains("safe_return_local") || output.contains("annotation"),
            "Should detect error in safe function. Output: {}", output);
}

#[test]
fn test_returning_array_element() {
    let code = r#"
// @safe
const int& get_first(const int* arr) {
    return arr[0];  // Returning element from array parameter
}

int main() {
    return 0;
}
"#;

    let temp_file = create_temp_cpp_file(code);
    let (success, output) = run_analyzer(temp_file.path());

    // Should require annotation for reference return
    assert!(!success || output.contains("annotation"),
            "Should require lifetime annotation. Output: {}", output);
}

#[test]
fn test_void_return_no_annotation_needed() {
    let code = r#"
// @safe
void do_something(const int& x) {
    int y = x + 1;
}

int main() {
    return 0;
}
"#;

    let temp_file = create_temp_cpp_file(code);
    let (success, output) = run_analyzer(temp_file.path());

    assert!(success, "Should pass - void return doesn't need annotation. Output: {}", output);
}

#[test]
fn test_returning_static_variable() {
    let code = r#"
// @safe
const int& get_static() {
    static int value = 42;
    return value;  // Returning static - technically safe but needs annotation
}

int main() {
    return 0;
}
"#;

    let temp_file = create_temp_cpp_file(code);
    let (success, output) = run_analyzer(temp_file.path());

    // Should require annotation since we can't distinguish static from local easily
    assert!(!success || output.contains("annotation"),
            "Should require lifetime annotation. Output: {}", output);
}

#[test]
fn test_returning_global_reference() {
    let code = r#"
int global_value = 42;

// @safe
const int& get_global() {
    return global_value;  // Returning global reference
}

int main() {
    return 0;
}
"#;

    let temp_file = create_temp_cpp_file(code);
    let (success, output) = run_analyzer(temp_file.path());

    // Should require annotation
    assert!(!success || output.contains("annotation"),
            "Should require lifetime annotation. Output: {}", output);
}

#[test]
fn test_const_and_non_const_overloads() {
    let temp_dir = TempDir::new().unwrap();

    let header_content = r#"
#ifndef TEST_H
#define TEST_H

// @lifetime: (&'a) -> &'a
// @safe
int& get_value(int& x);

// @lifetime: (&'a) -> &'a
// @safe
const int& get_value(const int& x);

#endif
"#;

    let header_path = temp_dir.path().join("test.h");
    fs::write(&header_path, header_content).unwrap();

    let cpp_content = r#"
#include "test.h"

// @safe
int& get_value(int& x) {
    return x;
}

// @safe
const int& get_value(const int& x) {
    return x;
}

int main() {
    return 0;
}
"#;

    let cpp_path = temp_dir.path().join("test.cpp");
    fs::write(&cpp_path, cpp_content).unwrap();

    let (success, output) = run_analyzer(&cpp_path);

    assert!(success, "Should pass - both overloads properly annotated. Output: {}", output);
}
