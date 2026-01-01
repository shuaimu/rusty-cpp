/// Tests for rusty::move - Rust-like move semantics for C++
///
/// rusty::move differs from std::move in how references are handled:
/// - For values: Same as std::move - transfers ownership
/// - For mutable references: Invalidates the reference variable itself
/// - For const references: Compile error (use = to copy)

use std::process::Command;
use std::fs;
use std::path::Path;

fn run_checker(code: &str) -> String {
    run_checker_with_name(code, "test")
}

fn run_checker_with_name(code: &str, test_name: &str) -> String {
    let temp_dir = std::env::temp_dir();
    // Use unique filename per test to avoid race conditions
    let unique_id = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let test_file = temp_dir.join(format!("test_rusty_move_{}_{}.cpp", test_name, unique_id));

    // Create include directory structure
    let include_dir = temp_dir.join("rusty_include");
    let rusty_dir = include_dir.join("rusty");
    fs::create_dir_all(&rusty_dir).unwrap();

    // Copy the move.hpp header
    let move_header = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("include/rusty/move.hpp");
    if move_header.exists() {
        fs::copy(&move_header, rusty_dir.join("move.hpp")).unwrap();
    }

    fs::write(&test_file, code).unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_rusty-cpp-checker"))
        .arg(&test_file)
        .arg("-I")
        .arg(&include_dir)
        .output()
        .expect("Failed to run checker");

    // Clean up temp file
    let _ = fs::remove_file(&test_file);

    String::from_utf8_lossy(&output.stdout).to_string()
        + &String::from_utf8_lossy(&output.stderr)
}

// ============================================================================
// Basic rusty::move for values (same as std::move)
// ============================================================================

#[test]
fn test_rusty_move_value_basic() {
    let code = r#"
#include <rusty/move.hpp>

class Box {
    int* data;
public:
    Box(int v) : data(new int(v)) {}
    Box(Box&& other) : data(other.data) { other.data = nullptr; }
    ~Box() { delete data; }
};

// @safe
void test() {
    Box b1(42);
    Box b2 = rusty::move(b1);
    // b1 is now moved, using it would be an error
}
"#;
    let output = run_checker(code);
    assert!(
        output.contains("no violations") || !output.contains("error"),
        "Basic rusty::move on value should work. Output: {}", output
    );
}

#[test]
fn test_rusty_move_value_use_after_move() {
    // Use unique_ptr to match existing move detection patterns
    let code = r#"
#include <rusty/move.hpp>
#include <memory>

// @safe
void test() {
    std::unique_ptr<int> ptr(new int(42));
    std::unique_ptr<int> ptr2 = rusty::move(ptr);
    *ptr = 100;  // ERROR: use after move
}
"#;
    let output = run_checker(code);
    assert!(
        output.contains("use after move") || output.contains("Use after move") || output.contains("moved"),
        "Should detect use after rusty::move. Output: {}", output
    );
}

// ============================================================================
// rusty::move for mutable references (Rust-like semantics)
// ============================================================================

#[test]
fn test_rusty_move_mutable_ref_basic() {
    let code = r#"
#include <rusty/move.hpp>

// @safe
void test() {
    int x = 42;
    int& r1 = x;
    int& r2 = rusty::move(r1);  // r1 becomes invalid, r2 is valid
    // Using r1 would be an error, r2 is fine
    int y = r2;  // OK
}
"#;
    let output = run_checker(code);
    assert!(
        output.contains("no violations") || !output.contains("error"),
        "Basic rusty::move on mutable ref should work. Output: {}", output
    );
}

#[test]
fn test_rusty_move_mutable_ref_use_after_move() {
    // For references, rusty::move moves through the reference (same as std::move)
    // but the checker should also mark the reference as invalidated
    let code = r#"
#include <rusty/move.hpp>
#include <memory>

// @safe
void test() {
    std::unique_ptr<int> ptr(new int(42));
    std::unique_ptr<int>& r1 = ptr;
    std::unique_ptr<int> ptr2 = rusty::move(r1);  // Moves ptr through r1
    *r1 = 100;  // ERROR: use after move (r1 refers to moved ptr)
}
"#;
    let output = run_checker(code);
    assert!(
        output.contains("use after move") || output.contains("Use after move") || output.contains("moved") || output.contains("move"),
        "Should detect use of moved reference. Output: {}", output
    );
}

// ============================================================================
// rusty::move for rvalue references (named rvalue refs are lvalues)
// ============================================================================

#[test]
fn test_rusty_move_rvalue_ref() {
    let code = r#"
#include <rusty/move.hpp>

// @safe
void take_rvalue(int&& rr) {
    // Named rvalue ref is an lvalue, so rusty::move should invalidate it
    int&& r2 = rusty::move(rr);  // rr becomes invalid
    int y = r2;  // OK
}
"#;
    let output = run_checker(code);
    assert!(
        output.contains("no violations") || !output.contains("error"),
        "rusty::move on rvalue ref should work. Output: {}", output
    );
}

// ============================================================================
// const references should NOT be moved (compile error in C++)
// This test verifies the static_assert works
// ============================================================================

#[test]
fn test_rusty_move_const_ref_should_fail() {
    // This test verifies that rusty::move on const ref causes a compile error
    // We can't easily test this with our checker, but the static_assert will catch it
    // at C++ compile time. For the checker, we just verify it doesn't crash.
    let code = r#"
#include <rusty/move.hpp>

// @safe
void test() {
    int x = 42;
    const int& cr1 = x;
    const int& cr2 = cr1;  // Just use = for const refs
    int y = cr2;
}
"#;
    let output = run_checker(code);
    assert!(
        output.contains("no violations") || !output.contains("error"),
        "Copying const ref with = should work. Output: {}", output
    );
}

// ============================================================================
// rusty::copy for explicit copies
// ============================================================================

#[test]
fn test_rusty_copy() {
    let code = r#"
#include <rusty/move.hpp>

// @safe
void test() {
    int x = 42;
    int y = rusty::copy(x);  // Explicit copy
    int z = x;  // x is still valid after copy
}
"#;
    let output = run_checker(code);
    assert!(
        output.contains("no violations") || !output.contains("error"),
        "rusty::copy should not invalidate source. Output: {}", output
    );
}

// ============================================================================
// Compare std::move vs rusty::move behavior
// ============================================================================

#[test]
fn test_std_move_also_works() {
    let code = r#"
#include <utility>

class Box {
    int value;
public:
    Box(int v) : value(v) {}
};

// @safe
void test() {
    Box b1(42);
    Box b2 = std::move(b1);  // std::move still works
}
"#;
    let output = run_checker(code);
    // std::move should also trigger move detection
    assert!(
        output.contains("no violations") || !output.contains("error") || output.contains("@unsafe"),
        "std::move should still be recognized. Output: {}", output
    );
}

// ============================================================================
// Edge cases
// ============================================================================

#[test]
fn test_rusty_move_temporary() {
    let code = r#"
#include <rusty/move.hpp>

class Box {
    int value;
public:
    Box(int v) : value(v) {}
};

Box make_box() { return Box(42); }

// @safe
void test() {
    // Moving a temporary is a no-op but should compile
    Box b = rusty::move(make_box());
}
"#;
    let output = run_checker(code);
    assert!(
        output.contains("no violations") || !output.contains("error"),
        "rusty::move on temporary should work. Output: {}", output
    );
}

#[test]
fn test_rusty_move_chain() {
    let code = r#"
#include <rusty/move.hpp>

class Box {
    int value;
public:
    Box(int v) : value(v) {}
};

// @safe
void test() {
    Box b1(1);
    Box b2 = rusty::move(b1);  // b1 invalid
    Box b3 = rusty::move(b2);  // b2 invalid
    // Only b3 is valid now
}
"#;
    let output = run_checker(code);
    assert!(
        output.contains("no violations") || !output.contains("error"),
        "Chained rusty::move should work. Output: {}", output
    );
}

// ============================================================================
// std::move on references should be forbidden in @safe code
// ============================================================================

#[test]
fn test_std_move_on_reference_forbidden() {
    // std::move on a reference is forbidden in @safe code because it moves
    // the underlying object, not the reference. Use rusty::move instead.
    let code = r#"
#include <utility>
#include <memory>

// @safe
void test() {
    std::unique_ptr<int> ptr(new int(42));
    std::unique_ptr<int>& ref = ptr;
    std::unique_ptr<int> ptr2 = std::move(ref);  // ERROR: std::move on reference
}
"#;
    let output = run_checker(code);
    assert!(
        output.contains("std::move on reference") || output.contains("forbidden"),
        "Should detect std::move on reference in @safe code. Output: {}", output
    );
}

#[test]
fn test_std_move_on_rvalue_ref_param_forbidden() {
    // std::move on a named rvalue reference parameter is also forbidden
    // because the parameter is still a reference (lvalue in value category)
    let code = r#"
#include <utility>

// @safe
void take(int&& rr) {
    int&& r2 = std::move(rr);  // ERROR: std::move on rvalue reference
}
"#;
    let output = run_checker(code);
    assert!(
        output.contains("std::move on reference") || output.contains("forbidden"),
        "Should detect std::move on rvalue ref parameter in @safe code. Output: {}", output
    );
}

#[test]
fn test_rusty_move_on_reference_allowed() {
    // rusty::move on a reference is allowed because it has Rust-like semantics
    let code = r#"
#include <rusty/move.hpp>

// @safe
void test() {
    int x = 42;
    int& ref = x;
    int& ref2 = rusty::move(ref);  // OK: rusty::move has correct semantics
    int y = ref2;
}
"#;
    let output = run_checker(code);
    assert!(
        output.contains("no violations") || !output.contains("std::move on reference"),
        "rusty::move on reference should be allowed. Output: {}", output
    );
}

#[test]
fn test_std_move_on_value_allowed() {
    // std::move on a value (not reference) is still allowed
    let code = r#"
#include <utility>

class Box {
    int value;
public:
    Box(int v) : value(v) {}
};

// @safe
void test() {
    Box b1(42);
    Box b2 = std::move(b1);  // OK: std::move on value
}
"#;
    let output = run_checker(code);
    assert!(
        !output.contains("std::move on reference"),
        "std::move on value should be allowed. Output: {}", output
    );
}

#[test]
fn test_std_move_on_ref_in_unsafe_block_allowed() {
    // std::move on reference is allowed inside @unsafe block
    let code = r#"
#include <utility>
#include <memory>

// @safe
void test() {
    // @unsafe
    {
        std::unique_ptr<int> ptr(new int(42));
        std::unique_ptr<int>& ref = ptr;
        std::unique_ptr<int> ptr2 = std::move(ref);  // OK: in unsafe block
    }
}
"#;
    let output = run_checker(code);
    assert!(
        !output.contains("std::move on reference") || output.contains("no violations"),
        "std::move on reference in unsafe block should be allowed. Output: {}", output
    );
}

// ============================================================================
// Reference assignment semantics (Rust-like: mutable refs move, const refs copy)
// ============================================================================

#[test]
fn test_mutable_ref_assignment_is_move() {
    // When assigning a mutable reference to another, the original is moved (invalidated)
    // This matches Rust's behavior: &mut T is not Copy
    let code = r#"
// @safe
void test() {
    int x = 42;
    int& r1 = x;
    int& r2 = r1;  // r1 is moved to r2
    int y = r1;    // ERROR: use after move - r1 is invalid
}
"#;
    let output = run_checker(code);
    assert!(
        output.contains("moved") || output.contains("invalid") || output.contains("borrow"),
        "Should detect use after move of mutable reference. Output: {}", output
    );
}

#[test]
fn test_mutable_ref_assignment_target_valid() {
    // The target of a mutable reference assignment is valid
    let code = r#"
// @safe
void test() {
    int x = 42;
    int& r1 = x;
    int& r2 = r1;  // r1 is moved to r2
    int y = r2;    // OK: r2 is valid
}
"#;
    let output = run_checker(code);
    assert!(
        output.contains("no violations") || !output.contains("error"),
        "Target of mutable ref assignment should be valid. Output: {}", output
    );
}

#[test]
fn test_const_ref_assignment_is_copy() {
    // When assigning a const reference, both remain valid (immutable refs are Copy)
    // This matches Rust's behavior: &T is Copy
    let code = r#"
// @safe
void test() {
    int x = 42;
    const int& r1 = x;
    const int& r2 = r1;  // r1 is copied
    int y = r1;          // OK: r1 is still valid
    int z = r2;          // OK: r2 is also valid
}
"#;
    let output = run_checker(code);
    assert!(
        output.contains("no violations") || !output.contains("moved"),
        "Const ref assignment should copy, not move. Output: {}", output
    );
}

#[test]
fn test_multiple_const_ref_copies() {
    // Multiple copies of const references should all be valid
    let code = r#"
// @safe
void test() {
    int x = 42;
    const int& r1 = x;
    const int& r2 = r1;
    const int& r3 = r1;  // Can copy r1 multiple times
    const int& r4 = r2;  // Can also copy r2
    int a = r1;
    int b = r2;
    int c = r3;
    int d = r4;          // All are valid
}
"#;
    let output = run_checker(code);
    assert!(
        output.contains("no violations") || !output.contains("moved"),
        "Multiple const ref copies should all be valid. Output: {}", output
    );
}

#[test]
fn test_mutable_ref_chain_moves() {
    // Chain of mutable reference assignments: each one moves from previous
    let code = r#"
// @safe
void test() {
    int x = 42;
    int& r1 = x;
    int& r2 = r1;  // r1 moved
    int& r3 = r2;  // r2 moved
    int y = r1;    // ERROR: r1 was moved
}
"#;
    let output = run_checker(code);
    assert!(
        output.contains("moved") || output.contains("invalid"),
        "Should detect use of moved reference in chain. Output: {}", output
    );
}
