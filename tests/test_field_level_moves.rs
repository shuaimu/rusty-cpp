// Tests for field-level partial move tracking
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

// Helper to create a temporary C++ file and analyze it
fn analyze_cpp_code(code: &str, expected_errors: &[&str], should_have_errors: bool) {
    let temp_file = create_temp_cpp_file(code);
    let (_success, output) = run_analyzer(temp_file.path());

    if should_have_errors {
        for expected in expected_errors {
            assert!(
                output.contains(expected),
                "Expected error containing '{}' but got output:\n{}\nCode:\n{}",
                expected,
                output,
                code
            );
        }
    } else {
        // Check for any error indicators
        // Don't match "no violations found!"
        if output.contains("✗") || output.contains("violation(s)") ||
           (output.contains("violation") && !output.contains("no violations found")) {
            panic!(
                "Expected no errors but got output:\n{}\nCode:\n{}",
                output,
                code
            );
        }
    }
}

#[test]
fn test_simple_field_move() {
    let code = r#"
#include <utility>

struct Container {
    int data;
};

// @safe
void test() {
    Container c;
    int x = std::move(c.data);
    // This should be OK - moved field, unmoved object
}
"#;
    analyze_cpp_code(code, &[], false);
}

#[test]
fn test_use_moved_field() {
    let code = r#"
#include <utility>

struct Container {
    int data;
};

// @safe
void test() {
    Container c;
    int x = std::move(c.data);
    int y = c.data;  // ERROR: field has been moved
}
"#;
    analyze_cpp_code(code, &["moved"], true);
}

#[test]
fn test_move_whole_after_partial_move() {
    let code = r#"
#include <utility>

struct Container {
    int data;
    int other;
};

// @safe
void test() {
    Container c;
    int x = std::move(c.data);
    Container c2 = std::move(c);  // ERROR: cannot move whole object
}
"#;
    analyze_cpp_code(code, &["partially moved"], true);
}

#[test]
fn test_access_unmoved_field_after_partial_move() {
    let code = r#"
#include <utility>

struct Container {
    int data;
    int other;
};

// @safe
void test() {
    Container c;
    int x = std::move(c.data);
    int y = c.other;  // OK: other field not moved
}
"#;
    analyze_cpp_code(code, &[], false);
}

#[test]
fn test_multiple_field_moves() {
    let code = r#"
#include <utility>

struct Container {
    int data;
    int other;
    int third;
};

// @safe
void test() {
    Container c;
    int x = std::move(c.data);
    int y = std::move(c.other);
    int z = c.third;  // OK: third not moved
}
"#;
    analyze_cpp_code(code, &[], false);
}

#[test]
fn test_move_field_from_moved_object() {
    let code = r#"
#include <utility>

struct Container {
    int data;
};

// @safe
void test() {
    Container c;
    Container c2 = std::move(c);
    int x = std::move(c.data);  // ERROR: c has been moved
}
"#;
    analyze_cpp_code(code, &["has been moved"], true);
}

#[test]
fn test_field_move_with_borrow() {
    let code = r#"
#include <utility>

struct Container {
    int data;
};

// @safe
void test() {
    Container c;
    const Container& ref = c;
    int x = std::move(c.data);  // ERROR: c is borrowed
}
"#;
    analyze_cpp_code(code, &["borrowed"], true);
}

#[test]
fn test_field_move_after_borrow_ends() {
    let code = r#"
#include <utility>

struct Container {
    int data;
};

// @safe
void test() {
    Container c;
    {
        const Container& ref = c;
    }
    // Borrow ended - should be OK
    int x = std::move(c.data);
}
"#;
    analyze_cpp_code(code, &[], false);
}

#[test]
fn test_field_move_in_conditional() {
    let code = r#"
#include <utility>

struct Container {
    int data;
};

// @safe
void test(bool cond) {
    Container c;
    if (cond) {
        int x = std::move(c.data);
    }
    // Field might be moved - conservative check
    int y = c.data;  // Should ERROR: might be moved
}
"#;
    analyze_cpp_code(code, &["moved"], true);
}

#[test]
fn test_field_move_in_both_branches() {
    let code = r#"
#include <utility>

struct Container {
    int data;
    int other;
};

// @safe
void test(bool cond) {
    Container c;
    if (cond) {
        int x = std::move(c.data);
    } else {
        int y = std::move(c.other);
    }
    // Different fields moved in each branch
    // Whole object cannot be moved
    Container c2 = std::move(c);  // ERROR
}
"#;
    analyze_cpp_code(code, &["partially moved"], true);
}

#[test]
fn test_field_move_in_loop() {
    let code = r#"
#include <utility>

struct Container {
    int data;
};

// @safe
void test() {
    Container c;
    for (int i = 0; i < 2; i++) {
        int x = std::move(c.data);  // ERROR on 2nd iteration
    }
}
"#;
    analyze_cpp_code(code, &["moved"], true);
}

#[test]
fn test_different_objects_field_moves() {
    let code = r#"
#include <utility>

struct Container {
    int data;
};

// @safe
void test() {
    Container c1;
    Container c2;
    int x = std::move(c1.data);
    int y = std::move(c2.data);
    // Both objects partially moved
    Container a = std::move(c1);  // ERROR
    Container b = std::move(c2);  // ERROR
}
"#;
    analyze_cpp_code(code, &["partially moved", "partially moved"], true);
}

#[test]
fn test_nested_field_access() {
    // This is a more complex case that might not be fully supported yet
    let code = r#"
#include <utility>

struct Inner {
    int value;
};

struct Outer {
    Inner inner;
};

// @safe
void test() {
    Outer o;
    // For now, we only track one level - o.inner.value would be treated as a field "inner" of "o"
    // This test verifies current behavior
}
"#;
    analyze_cpp_code(code, &[], false);
}

#[test]
fn test_field_use_after_move() {
    let code = r#"
#include <utility>

struct Container {
    int data;
};

// @safe
void use_int(int x) {}

// @safe
void test() {
    Container c;
    int x = std::move(c.data);
    use_int(c.data);  // ERROR: use after move
}
"#;
    analyze_cpp_code(code, &["moved"], true);
}

#[test]
fn test_field_double_move() {
    let code = r#"
#include <utility>

struct Container {
    int data;
};

// @safe
void test() {
    Container c;
    int x = std::move(c.data);
    int y = std::move(c.data);  // ERROR: already moved
}
"#;
    analyze_cpp_code(code, &["already been moved"], true);
}

#[test]
fn test_unmoved_fields_accessible() {
    let code = r#"
#include <utility>

struct Container {
    int a;
    int b;
    int c;
    int d;
};

// @safe
void test() {
    Container obj;
    int x = std::move(obj.a);
    int y = std::move(obj.c);
    // b and d should still be accessible
    int z1 = obj.b;
    int z2 = obj.d;
}
"#;
    analyze_cpp_code(code, &[], false);
}

#[test]
fn test_whole_object_move_prevents_field_access() {
    let code = r#"
#include <utility>

struct Container {
    int data;
};

// @safe
void test() {
    Container c;
    Container c2 = std::move(c);
    int x = c.data;  // ERROR: c has been moved
}
"#;
    analyze_cpp_code(code, &["has been moved"], true);
}
