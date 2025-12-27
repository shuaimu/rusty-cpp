//! Integration tests for string literal safety tracking
//!
//! String literals in C++ have static lifetime and are safe.
//! Explicit char* variables are unsafe because their origin is unknown.

use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

static TEST_COUNTER: AtomicUsize = AtomicUsize::new(0);

fn run_checker(source_code: &str) -> (i32, String) {
    // Use unique temp file per test to avoid contention
    let test_id = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
    let temp_file = std::env::temp_dir().join(format!("string_literal_test_{}.cpp", test_id));
    std::fs::write(&temp_file, source_code).expect("Failed to write temp file");

    // Run the checker
    let output = Command::new("cargo")
        .args(["run", "--quiet", "--", temp_file.to_str().unwrap()])
        .output()
        .expect("Failed to run checker");

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let combined = format!("{}{}", stdout, stderr);

    // Clean up temp file
    let _ = std::fs::remove_file(&temp_file);

    (output.status.code().unwrap_or(-1), combined)
}

#[test]
fn test_string_literal_passed_directly() {
    let code = r#"
// @safe
void log(const char* msg);

// @safe
void test() {
    log("hello");  // OK - string literal is safe
}
"#;
    let (_, output) = run_checker(code);
    assert!(
        output.contains("no violations found"),
        "String literal passed directly should be safe. Output: {}", output
    );
}

#[test]
fn test_multiple_string_literals() {
    let code = r#"
// @safe
void printf_safe(const char* fmt, ...);

// @safe
void test() {
    printf_safe("%s %d\n", "value:", 42);  // OK - all safe
}
"#;
    let (_, output) = run_checker(code);
    assert!(
        output.contains("no violations found"),
        "Multiple string literals should be safe. Output: {}", output
    );
}

#[test]
fn test_char_ptr_variable_unsafe() {
    let code = r#"
// @safe
void test() {
    const char* ptr = "hello";  // ERROR - char* variable in @safe
}
"#;
    let (_, output) = run_checker(code);
    assert!(
        output.contains("Cannot declare 'ptr' with type"),
        "char* variable declaration should be flagged. Output: {}", output
    );
}

#[test]
fn test_char_ptr_in_unsafe_block_ok() {
    let code = r#"
// @safe
void log(const char* msg);

// @safe
void test() {
    // @unsafe
    {
        const char* ptr = "hello";  // OK - in @unsafe block
        log(ptr);
    }
}
"#;
    let (_, output) = run_checker(code);
    assert!(
        output.contains("no violations found"),
        "char* in @unsafe block should be OK. Output: {}", output
    );
}

#[test]
fn test_safe_wrapper_pattern() {
    let code = r#"
void internal_log(const char* msg);

// @safe
void safe_log(const char* msg) {
    // @unsafe
    {
        internal_log(msg);  // OK - inside @unsafe
    }
}

// @safe
void test() {
    safe_log("hello");  // OK - passing literal to safe wrapper
}
"#;
    let (_, output) = run_checker(code);
    assert!(
        output.contains("no violations found"),
        "Safe wrapper pattern should work. Output: {}", output
    );
}

#[test]
fn test_wchar_ptr_variable_unsafe() {
    let code = r#"
// @safe
void test() {
    const wchar_t* wide = L"hello";  // ERROR - wchar_t* is also unsafe
}
"#;
    let (_, output) = run_checker(code);
    assert!(
        output.contains("Cannot declare 'wide' with type"),
        "wchar_t* variable declaration should be flagged. Output: {}", output
    );
}

#[test]
fn test_char_ptr_in_unsafe_function_ok() {
    let code = r#"
// @unsafe
void unsafe_function() {
    const char* ptr = "hello";  // OK - function is @unsafe
}
"#;
    let (_, output) = run_checker(code);
    assert!(
        output.contains("no violations found"),
        "char* in @unsafe function should be OK. Output: {}", output
    );
}
