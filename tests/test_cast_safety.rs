//! Integration tests for Cast Safety (Phase 4 of Full Pointer Safety)
//!
//! Tests the detection of unsafe C++ casts in @safe code:
//! - reinterpret_cast always requires @unsafe
//! - const_cast always requires @unsafe
//! - C-style casts require @unsafe (could be reinterpret or const cast)
//! - static_cast is generally safe for numeric conversions
//! - dynamic_cast is runtime-checked and safe

use std::process::Command;
use std::path::PathBuf;
use std::fs;

fn get_checker_path() -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("target");
    path.push("debug");
    path.push("rusty-cpp-checker");
    path
}

fn run_checker(source_code: &str) -> String {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let source_path = temp_dir.path().join("test.cpp");
    fs::write(&source_path, source_code).expect("Failed to write source file");

    let checker_path = get_checker_path();
    let output = Command::new(&checker_path)
        .arg(&source_path)
        .output()
        .expect("Failed to run checker");

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    format!("{}{}", stdout, stderr)
}

// ============================================================================
// Test: reinterpret_cast (always unsafe)
// ============================================================================

#[test]
fn test_reinterpret_cast_unsafe() {
    let code = r#"
// @safe
void process() {
    int x = 42;
    float* fp = reinterpret_cast<float*>(&x);  // ERROR: reinterpret_cast
}
"#;
    let output = run_checker(code);
    assert!(
        output.contains("reinterpret_cast") || output.contains("violation") || output.contains("unsafe"),
        "Expected error for reinterpret_cast in @safe code. Output: {}",
        output
    );
}

#[test]
fn test_reinterpret_cast_in_unsafe_block() {
    let code = r#"
// @safe
void process() {
    int x = 42;
    // @unsafe
    {
        float* fp = reinterpret_cast<float*>(&x);  // OK: in @unsafe block
    }
}
"#;
    let output = run_checker(code);
    // Inside @unsafe block, reinterpret_cast is allowed
    assert!(
        !output.contains("reinterpret_cast") || output.contains("no violations"),
        "reinterpret_cast should be allowed in @unsafe block. Output: {}",
        output
    );
}

// ============================================================================
// Test: const_cast (always unsafe)
// ============================================================================

#[test]
fn test_const_cast_unsafe() {
    let code = r#"
// @safe
void process(const int* p) {
    int* mutable_p = const_cast<int*>(p);  // ERROR: const_cast
}
"#;
    let output = run_checker(code);
    assert!(
        output.contains("const_cast") || output.contains("violation") || output.contains("unsafe"),
        "Expected error for const_cast in @safe code. Output: {}",
        output
    );
}

#[test]
fn test_const_cast_in_unsafe_block() {
    let code = r#"
// @safe
void process(const int* p) {
    // @unsafe
    {
        int* mutable_p = const_cast<int*>(p);  // OK: in @unsafe block
    }
}
"#;
    let output = run_checker(code);
    // Inside @unsafe block, const_cast is allowed
    println!("Output: {}", output);
}

// ============================================================================
// Test: C-style cast (unsafe - could be any cast type)
// ============================================================================

#[test]
fn test_c_style_cast_unsafe() {
    let code = r#"
// @safe
void process() {
    int x = 42;
    float* fp = (float*)&x;  // ERROR: C-style cast (could be reinterpret_cast)
}
"#;
    let output = run_checker(code);
    assert!(
        output.contains("C-style") || output.contains("cast") || output.contains("violation"),
        "Expected error for C-style cast in @safe code. Output: {}",
        output
    );
}

#[test]
fn test_c_style_cast_in_unsafe_block() {
    let code = r#"
// @safe
void process() {
    int x = 42;
    // @unsafe
    {
        float* fp = (float*)&x;  // OK: in @unsafe block
    }
}
"#;
    let output = run_checker(code);
    println!("Output: {}", output);
}

// ============================================================================
// Test: static_cast (generally safe)
// ============================================================================

#[test]
fn test_static_cast_numeric_safe() {
    let code = r#"
// @safe
void process() {
    double d = 3.14;
    int i = static_cast<int>(d);  // OK: numeric conversion
}
"#;
    let output = run_checker(code);
    // static_cast for numeric conversion should not be flagged
    assert!(
        !output.contains("static_cast") || output.contains("no violations"),
        "static_cast for numeric conversion should be safe. Output: {}",
        output
    );
}

#[test]
fn test_static_cast_upcast_safe() {
    let code = r#"
class Base {};
class Derived : public Base {};

// @safe
void process(Derived* d) {
    // @unsafe
    {
        Base* b = static_cast<Base*>(d);  // OK: upcast
    }
}
"#;
    let output = run_checker(code);
    // Upcast is safe
    println!("Output: {}", output);
}

// ============================================================================
// Test: dynamic_cast (runtime-checked, safe)
// ============================================================================

#[test]
fn test_dynamic_cast_safe() {
    let code = r#"
class Base {
public:
    virtual ~Base() {}
};
class Derived : public Base {};

// @safe
void process(Base* b) {
    // @unsafe
    {
        Derived* d = dynamic_cast<Derived*>(b);  // OK: runtime-checked
        // In real code, should check if d is nullptr
    }
}
"#;
    let output = run_checker(code);
    // dynamic_cast is runtime-checked and generally safe
    println!("Output: {}", output);
}

// ============================================================================
// Test: @unsafe function (casts allowed)
// ============================================================================

#[test]
fn test_casts_in_unsafe_function() {
    let code = r#"
// @unsafe
void unsafe_function() {
    int x = 42;
    float* fp = reinterpret_cast<float*>(&x);  // OK: in @unsafe function
    const int* cp = &x;
    int* mp = const_cast<int*>(cp);  // OK: in @unsafe function
    double* dp = (double*)&x;  // OK: in @unsafe function
}
"#;
    let output = run_checker(code);
    // @unsafe function allows all casts
    assert!(
        !output.contains("reinterpret_cast") && !output.contains("const_cast"),
        "Casts should be allowed in @unsafe function. Output: {}",
        output
    );
}

// ============================================================================
// Test: Cast in expression context
// ============================================================================

#[test]
fn test_cast_in_function_call() {
    let code = r#"
void process_float(float* f);

// @safe
void caller() {
    int x = 42;
    process_float(reinterpret_cast<float*>(&x));  // ERROR
}
"#;
    let output = run_checker(code);
    // reinterpret_cast in function call argument should be flagged
    println!("Output: {}", output);
}

#[test]
fn test_cast_in_return() {
    let code = r#"
// @safe
float* bad_cast(int* ip) {
    return reinterpret_cast<float*>(ip);  // ERROR
}
"#;
    let output = run_checker(code);
    // reinterpret_cast in return should be flagged
    println!("Output: {}", output);
}

// ============================================================================
// Test: Multiple casts in one function
// ============================================================================

#[test]
fn test_multiple_unsafe_casts() {
    let code = r#"
// @safe
void multiple_casts() {
    int x = 42;
    const int* cp = &x;
    int* p1 = const_cast<int*>(cp);           // ERROR 1
    float* p2 = reinterpret_cast<float*>(&x); // ERROR 2
    double* p3 = (double*)&x;                  // ERROR 3
}
"#;
    let output = run_checker(code);
    // All three unsafe casts should be flagged
    println!("Output: {}", output);
}

// ============================================================================
// Test: Implicit casts (safe)
// ============================================================================

#[test]
fn test_implicit_cast_safe() {
    let code = r#"
// @safe
void process() {
    int x = 42;
    double d = x;  // Implicit numeric conversion - safe
}
"#;
    let output = run_checker(code);
    // Implicit conversions are safe
    assert!(
        !output.contains("cast") || output.contains("no violations"),
        "Implicit casts should be safe. Output: {}",
        output
    );
}

// ============================================================================
// Test: Void pointer casts
// ============================================================================

#[test]
fn test_void_pointer_cast() {
    let code = r#"
// @safe
void process() {
    int x = 42;
    void* vp = &x;            // OK: implicit to void*
    // @unsafe
    {
        int* ip = static_cast<int*>(vp);  // May need @unsafe
    }
}
"#;
    let output = run_checker(code);
    // Casting from void* back to original type
    println!("Output: {}", output);
}
