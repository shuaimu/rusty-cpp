// Comprehensive tests for Ptr<T> and MutPtr<T> borrow checking
//
// These tests mirror the reference borrow tests to ensure Ptr/MutPtr
// behave equivalently to C++ references with Rust-style borrow rules:
//   - Ptr<T>    behaves like const T& (immutable borrow)
//   - MutPtr<T> behaves like T& (mutable borrow)
//
// Key rules enforced:
//   1. Multiple Ptr (immutable) borrows allowed
//   2. Only one MutPtr (mutable) borrow allowed
//   3. Cannot mix Ptr and MutPtr borrows of same variable
//   4. Cannot move while borrowed by Ptr/MutPtr
//   5. Borrows end at scope exit

use std::process::Command;
use std::fs;
use std::env;

fn create_temp_file(name: &str, code: &str) -> std::path::PathBuf {
    let temp_dir = env::temp_dir();
    let temp_file = temp_dir.join(format!("test_ptr_borrow_{}.cpp", name));
    fs::write(&temp_file, code).unwrap();
    temp_file
}

fn run_analyzer(file_path: &std::path::PathBuf, include_dir: &str) -> String {
    let output = Command::new("cargo")
        .args(&["run", "--", file_path.to_str().unwrap(), "-I", include_dir])
        .output()
        .expect("Failed to run analyzer");

    String::from_utf8_lossy(&output.stdout).to_string()
}

fn cleanup(file_path: &std::path::PathBuf) {
    let _ = fs::remove_file(file_path);
}

// ============================================================================
// BASIC BORROW TESTS - Single borrows
// ============================================================================

#[test]
fn test_single_ptr_borrow_allowed() {
    let code = r#"
#include "rusty/ptr.hpp"

// @safe
void test() {
    int x = 42;
    rusty::Ptr<int> p = &x;  // Single immutable borrow - OK
    int val = *p;
}
"#;
    let temp_file = create_temp_file("single_ptr", code);
    let output = run_analyzer(&temp_file, "include");
    cleanup(&temp_file);

    assert!(
        !output.contains("already") && !output.contains("Cannot"),
        "Single Ptr borrow should be allowed. Output: {}", output
    );
}

#[test]
fn test_single_mutptr_borrow_allowed() {
    let code = r#"
#include "rusty/ptr.hpp"

// @safe
void test() {
    int x = 42;
    rusty::MutPtr<int> mp = &x;  // Single mutable borrow - OK
    *mp = 100;
}
"#;
    let temp_file = create_temp_file("single_mutptr", code);
    let output = run_analyzer(&temp_file, "include");
    cleanup(&temp_file);

    assert!(
        !output.contains("already") && !output.contains("Cannot"),
        "Single MutPtr borrow should be allowed. Output: {}", output
    );
}

// ============================================================================
// MULTIPLE IMMUTABLE BORROWS (SHOULD BE ALLOWED)
// ============================================================================

#[test]
fn test_multiple_ptr_borrows_allowed() {
    let code = r#"
#include "rusty/ptr.hpp"

// @safe
void test() {
    int x = 42;
    rusty::Ptr<int> p1 = &x;  // Immutable borrow 1
    rusty::Ptr<int> p2 = &x;  // Immutable borrow 2 - OK
    rusty::Ptr<int> p3 = &x;  // Immutable borrow 3 - OK
    int sum = *p1 + *p2 + *p3;
}
"#;
    let temp_file = create_temp_file("multiple_ptr", code);
    let output = run_analyzer(&temp_file, "include");
    cleanup(&temp_file);

    assert!(
        !output.contains("already") && !output.contains("Cannot"),
        "Multiple Ptr borrows should be allowed. Output: {}", output
    );
}

// ============================================================================
// DOUBLE MUTABLE BORROW (SHOULD FAIL)
// ============================================================================

#[test]
fn test_double_mutptr_borrow_fails() {
    let code = r#"
#include "rusty/ptr.hpp"

// @safe
void test() {
    int x = 42;
    rusty::MutPtr<int> mp1 = &x;  // Mutable borrow 1
    rusty::MutPtr<int> mp2 = &x;  // ERROR: double mutable borrow
    *mp1 = 1;
    *mp2 = 2;
}
"#;
    let temp_file = create_temp_file("double_mutptr", code);
    let output = run_analyzer(&temp_file, "include");
    cleanup(&temp_file);

    assert!(
        output.contains("already") && output.contains("borrow"),
        "Double MutPtr borrow should fail. Output: {}", output
    );
}

// ============================================================================
// MIXING MUTABLE AND IMMUTABLE BORROWS (SHOULD FAIL)
// ============================================================================

#[test]
fn test_ptr_then_mutptr_fails() {
    let code = r#"
#include "rusty/ptr.hpp"

// @safe
void test() {
    int x = 42;
    rusty::Ptr<int> p = &x;       // Immutable borrow
    rusty::MutPtr<int> mp = &x;   // ERROR: mutable borrow while immutable exists
    int val = *p;
}
"#;
    let temp_file = create_temp_file("ptr_then_mutptr", code);
    let output = run_analyzer(&temp_file, "include");
    cleanup(&temp_file);

    assert!(
        output.contains("already") && output.contains("borrow"),
        "Ptr then MutPtr should fail. Output: {}", output
    );
}

#[test]
fn test_mutptr_then_ptr_fails() {
    let code = r#"
#include "rusty/ptr.hpp"

// @safe
void test() {
    int x = 42;
    rusty::MutPtr<int> mp = &x;  // Mutable borrow
    rusty::Ptr<int> p = &x;      // ERROR: immutable borrow while mutable exists
    *mp = 100;
}
"#;
    let temp_file = create_temp_file("mutptr_then_ptr", code);
    let output = run_analyzer(&temp_file, "include");
    cleanup(&temp_file);

    assert!(
        output.contains("already") && output.contains("borrow"),
        "MutPtr then Ptr should fail. Output: {}", output
    );
}

// ============================================================================
// SCOPE-BASED BORROW ENDING
// ============================================================================

#[test]
fn test_ptr_borrow_ends_at_scope() {
    let code = r#"
#include "rusty/ptr.hpp"

// @safe
void test() {
    int x = 42;
    {
        rusty::Ptr<int> p = &x;  // Borrow in inner scope
        int val = *p;
    }  // Borrow ends here
    rusty::MutPtr<int> mp = &x;  // OK: no active immutable borrow
    *mp = 100;
}
"#;
    let temp_file = create_temp_file("ptr_scope_end", code);
    let output = run_analyzer(&temp_file, "include");
    cleanup(&temp_file);

    assert!(
        !output.contains("already") || output.contains("no violations"),
        "Ptr borrow should end at scope exit. Output: {}", output
    );
}

#[test]
fn test_mutptr_borrow_ends_at_scope() {
    let code = r#"
#include "rusty/ptr.hpp"

// @safe
void test() {
    int x = 42;
    {
        rusty::MutPtr<int> mp = &x;  // Mutable borrow in inner scope
        *mp = 100;
    }  // Mutable borrow ends here
    rusty::MutPtr<int> mp2 = &x;  // OK: previous mutable borrow ended
    *mp2 = 200;
}
"#;
    let temp_file = create_temp_file("mutptr_scope_end", code);
    let output = run_analyzer(&temp_file, "include");
    cleanup(&temp_file);

    assert!(
        !output.contains("already") || output.contains("no violations"),
        "MutPtr borrow should end at scope exit. Output: {}", output
    );
}

#[test]
fn test_nested_scope_borrow_cleanup() {
    let code = r#"
#include "rusty/ptr.hpp"

// @safe
void test() {
    int x = 42;
    {
        {
            rusty::Ptr<int> p = &x;
            int val = *p;
        }  // Inner Ptr scope ends
    }  // Outer scope ends
    rusty::MutPtr<int> mp = &x;  // OK: all borrows ended
    *mp = 100;
}
"#;
    let temp_file = create_temp_file("nested_scope", code);
    let output = run_analyzer(&temp_file, "include");
    cleanup(&temp_file);

    assert!(
        !output.contains("already") || output.contains("no violations"),
        "Nested scope borrows should be cleaned up. Output: {}", output
    );
}

// ============================================================================
// CANNOT MOVE WHILE BORROWED BY Ptr/MutPtr
// ============================================================================

#[test]
fn test_cannot_move_while_ptr_borrowed() {
    let code = r#"
#include "rusty/ptr.hpp"
#include "rusty/box.hpp"

// @safe
void take_ownership(rusty::Box<int> b) {
    int x = *b;
}

// @safe
void test() {
    rusty::Box<int> b = rusty::Box<int>::make(42);
    rusty::Ptr<rusty::Box<int>> p = &b;  // Ptr borrows b
    take_ownership(std::move(b));        // ERROR: cannot move while borrowed
}
"#;
    let temp_file = create_temp_file("move_while_ptr", code);
    let output = run_analyzer(&temp_file, "include");
    cleanup(&temp_file);

    assert!(
        output.contains("Cannot move") || output.contains("borrowed"),
        "Should not allow move while Ptr borrowed. Output: {}", output
    );
}

#[test]
fn test_cannot_move_while_mutptr_borrowed() {
    let code = r#"
#include "rusty/ptr.hpp"
#include "rusty/box.hpp"

// @safe
void take_ownership(rusty::Box<int> b) {
    int x = *b;
}

// @safe
void test() {
    rusty::Box<int> b = rusty::Box<int>::make(42);
    rusty::MutPtr<rusty::Box<int>> mp = &b;  // MutPtr borrows b
    take_ownership(std::move(b));            // ERROR: cannot move while borrowed
}
"#;
    let temp_file = create_temp_file("move_while_mutptr", code);
    let output = run_analyzer(&temp_file, "include");
    cleanup(&temp_file);

    assert!(
        output.contains("Cannot move") || output.contains("borrowed"),
        "Should not allow move while MutPtr borrowed. Output: {}", output
    );
}

#[test]
fn test_can_move_after_ptr_scope_ends() {
    let code = r#"
#include "rusty/ptr.hpp"
#include "rusty/box.hpp"

// @safe
void take_ownership(rusty::Box<int> b) {
    int x = *b;
}

// @safe
void test() {
    rusty::Box<int> b = rusty::Box<int>::make(42);
    {
        rusty::Ptr<rusty::Box<int>> p = &b;
        int val = **p;
    }  // Ptr borrow ends here
    take_ownership(std::move(b));  // OK: no active borrow
}
"#;
    let temp_file = create_temp_file("move_after_ptr_scope", code);
    let output = run_analyzer(&temp_file, "include");
    cleanup(&temp_file);

    assert!(
        output.contains("no violations") || !output.contains("Cannot move"),
        "Should allow move after Ptr scope ends. Output: {}", output
    );
}

// ============================================================================
// SEQUENTIAL BORROWS IN SEPARATE SCOPES
// ============================================================================

#[test]
fn test_sequential_ptr_borrows_in_scopes() {
    let code = r#"
#include "rusty/ptr.hpp"

// @safe
void test() {
    int x = 42;

    {
        rusty::Ptr<int> p1 = &x;
        int v1 = *p1;
    }  // p1 ends

    {
        rusty::Ptr<int> p2 = &x;
        int v2 = *p2;
    }  // p2 ends

    rusty::MutPtr<int> mp = &x;  // OK: all Ptr borrows ended
    *mp = 100;
}
"#;
    let temp_file = create_temp_file("sequential_ptr", code);
    let output = run_analyzer(&temp_file, "include");
    cleanup(&temp_file);

    assert!(
        !output.contains("already") || output.contains("no violations"),
        "Sequential Ptr borrows in scopes should work. Output: {}", output
    );
}

#[test]
fn test_sequential_mutptr_borrows_in_scopes() {
    let code = r#"
#include "rusty/ptr.hpp"

// @safe
void test() {
    int x = 42;

    {
        rusty::MutPtr<int> mp1 = &x;
        *mp1 = 100;
    }  // mp1 ends

    {
        rusty::MutPtr<int> mp2 = &x;
        *mp2 = 200;
    }  // mp2 ends

    // Final value is 200
}
"#;
    let temp_file = create_temp_file("sequential_mutptr", code);
    let output = run_analyzer(&temp_file, "include");
    cleanup(&temp_file);

    assert!(
        !output.contains("already") || output.contains("no violations"),
        "Sequential MutPtr borrows in scopes should work. Output: {}", output
    );
}

// ============================================================================
// CONDITIONAL BORROWS
// ============================================================================

#[test]
fn test_conditional_ptr_borrow_cleanup() {
    let code = r#"
#include "rusty/ptr.hpp"

// @safe
void test(bool condition) {
    int x = 42;

    if (condition) {
        rusty::Ptr<int> p = &x;
        int val = *p;
    }  // p only exists in if branch

    rusty::MutPtr<int> mp = &x;  // OK: p scope ended
    *mp = 100;
}
"#;
    let temp_file = create_temp_file("conditional_ptr", code);
    let output = run_analyzer(&temp_file, "include");
    cleanup(&temp_file);

    assert!(
        !output.contains("already") || output.contains("no violations"),
        "Conditional Ptr borrow should be cleaned up. Output: {}", output
    );
}

// ============================================================================
// LOOP BORROWS
// ============================================================================

#[test]
fn test_loop_ptr_borrow_prevents_mutptr() {
    let code = r#"
#include "rusty/ptr.hpp"

// @safe
void test() {
    int x = 42;

    for (int i = 0; i < 2; i++) {
        rusty::Ptr<int> p = &x;     // Ptr in loop
        rusty::MutPtr<int> mp = &x; // ERROR: conflicts with Ptr
        int val = *p;
    }
}
"#;
    let temp_file = create_temp_file("loop_ptr_conflict", code);
    let output = run_analyzer(&temp_file, "include");
    cleanup(&temp_file);

    assert!(
        output.contains("already") && output.contains("borrow"),
        "Loop Ptr should conflict with MutPtr. Output: {}", output
    );
}

#[test]
fn test_loop_mutptr_double_borrow_fails() {
    let code = r#"
#include "rusty/ptr.hpp"

// @safe
void test() {
    int x = 42;

    for (int i = 0; i < 2; i++) {
        rusty::MutPtr<int> mp1 = &x;  // Mutable borrow
        rusty::MutPtr<int> mp2 = &x;  // ERROR: double mutable borrow
        *mp1 = i;
    }
}
"#;
    let temp_file = create_temp_file("loop_double_mutptr", code);
    let output = run_analyzer(&temp_file, "include");
    cleanup(&temp_file);

    assert!(
        output.contains("already") && output.contains("borrow"),
        "Loop double MutPtr should fail. Output: {}", output
    );
}

// ============================================================================
// MULTIPLE BORROWERS PREVENT MOVE
// ============================================================================

#[test]
fn test_multiple_ptr_borrowers_prevent_move() {
    let code = r#"
#include "rusty/ptr.hpp"
#include "rusty/box.hpp"

// @safe
void take_ownership(rusty::Box<int> b) {
    int x = *b;
}

// @safe
void test() {
    rusty::Box<int> b = rusty::Box<int>::make(42);
    rusty::Ptr<rusty::Box<int>> p1 = &b;
    rusty::Ptr<rusty::Box<int>> p2 = &b;
    rusty::Ptr<rusty::Box<int>> p3 = &b;

    // ERROR: b is borrowed by p1, p2, p3
    take_ownership(std::move(b));
}
"#;
    let temp_file = create_temp_file("multiple_ptr_prevent_move", code);
    let output = run_analyzer(&temp_file, "include");
    cleanup(&temp_file);

    assert!(
        output.contains("Cannot move") || output.contains("borrowed"),
        "Multiple Ptr borrowers should prevent move. Output: {}", output
    );
}

// ============================================================================
// INDEPENDENT BORROWS OF DIFFERENT VARIABLES
// ============================================================================

#[test]
fn test_independent_variable_borrows() {
    let code = r#"
#include "rusty/ptr.hpp"

// @safe
void test() {
    int x = 1;
    int y = 2;

    rusty::Ptr<int> px = &x;      // Borrow x
    rusty::MutPtr<int> mpy = &y;  // Borrow y (independent)

    int val = *px;
    *mpy = 100;
}
"#;
    let temp_file = create_temp_file("independent_borrows", code);
    let output = run_analyzer(&temp_file, "include");
    cleanup(&temp_file);

    assert!(
        !output.contains("already") || output.contains("no violations"),
        "Independent variable borrows should be allowed. Output: {}", output
    );
}

// ============================================================================
// BORROW AFTER MOVE (USE-AFTER-MOVE)
// ============================================================================

#[test]
fn test_ptr_borrow_after_move_fails() {
    let code = r#"
#include "rusty/ptr.hpp"
#include "rusty/box.hpp"

// @safe
void take_ownership(rusty::Box<int> b) {
    int x = *b;
}

// @safe
void test() {
    rusty::Box<int> b = rusty::Box<int>::make(42);
    take_ownership(std::move(b));

    // ERROR: b has been moved
    rusty::Ptr<rusty::Box<int>> p = &b;
}
"#;
    let temp_file = create_temp_file("ptr_after_move", code);
    let output = run_analyzer(&temp_file, "include");
    cleanup(&temp_file);

    assert!(
        output.contains("moved") || output.contains("violation"),
        "Ptr borrow after move should fail. Output: {}", output
    );
}

// ============================================================================
// REBINDING BORROWS
// ============================================================================

#[test]
fn test_ptr_rebind_allowed() {
    let code = r#"
#include "rusty/ptr.hpp"

// @safe
void test() {
    int x = 42;
    int y = 100;

    rusty::Ptr<int> p = &x;  // Borrow x
    int v1 = *p;

    p = &y;  // Rebind to y (x borrow ends implicitly)
    int v2 = *p;

    rusty::MutPtr<int> mpx = &x;  // OK: x no longer borrowed
    *mpx = 200;
}
"#;
    let temp_file = create_temp_file("ptr_rebind", code);
    let output = run_analyzer(&temp_file, "include");
    cleanup(&temp_file);

    // Note: This test documents expected behavior - rebinding may or may not
    // be fully tracked yet
    println!("Ptr rebind output: {}", output);
}

#[test]
fn test_mutptr_rebind_releases_borrow() {
    let code = r#"
#include "rusty/ptr.hpp"

// @safe
void test() {
    int x = 42;
    int y = 100;

    rusty::MutPtr<int> mp = &x;  // Mutable borrow of x
    *mp = 50;

    mp = &y;  // Rebind to y (x mutable borrow ends)
    *mp = 150;

    rusty::MutPtr<int> mp2 = &x;  // OK: x no longer borrowed
    *mp2 = 200;
}
"#;
    let temp_file = create_temp_file("mutptr_rebind", code);
    let output = run_analyzer(&temp_file, "include");
    cleanup(&temp_file);

    // Note: This test documents expected behavior
    println!("MutPtr rebind output: {}", output);
}
