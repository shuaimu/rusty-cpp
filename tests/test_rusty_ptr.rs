// Tests for rusty::Ptr<T> and rusty::MutPtr<T> type aliases
//
// These types provide Rust-like pointer semantics:
//   Ptr<T>    - const T* (like *const T) - immutable pointee by default
//   MutPtr<T> - T*       (like *mut T)   - explicit mutable pointee
//
// In @safe code, both require @unsafe for dereference and address-of operations.

use std::process::Command;
use std::fs;
use std::env;

fn create_temp_file(name: &str, code: &str) -> std::path::PathBuf {
    let temp_dir = env::temp_dir();
    let temp_file = temp_dir.join(format!("test_rusty_ptr_{}.cpp", name));
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

// =============================================================================
// Basic Ptr<T> and MutPtr<T> tests
// =============================================================================

#[test]
fn test_ptr_address_of_allowed_in_safe() {
    // Address-of IS allowed when assigning to Ptr<T> because Ptr provides safe semantics
    let code = r#"
#include "rusty/ptr.hpp"

// @safe
void test_ptr_addr() {
    int x = 42;
    rusty::Ptr<int> p = &x;  // OK: address-of allowed when assigning to Ptr<T>
}
"#;

    let temp_file = create_temp_file("addr_of", code);
    let output = run_analyzer(&temp_file, "include");
    cleanup(&temp_file);

    assert!(
        !output.contains("address-of"),
        "Ptr address-of should be allowed in @safe code. Output: {}",
        output
    );
}

#[test]
fn test_mutptr_address_of_allowed_in_safe() {
    // Address-of IS allowed when assigning to MutPtr<T> because MutPtr provides safe semantics
    let code = r#"
#include "rusty/ptr.hpp"

// @safe
void test_mutptr_addr() {
    int x = 42;
    rusty::MutPtr<int> mp = &x;  // OK: address-of allowed when assigning to MutPtr<T>
}
"#;

    let temp_file = create_temp_file("mutptr_addr_of", code);
    let output = run_analyzer(&temp_file, "include");
    cleanup(&temp_file);

    assert!(
        !output.contains("address-of"),
        "MutPtr address-of should be allowed in @safe code. Output: {}",
        output
    );
}

#[test]
fn test_ptr_dereference_requires_unsafe() {
    let code = r#"
#include "rusty/ptr.hpp"

// @unsafe
rusty::Ptr<int> get_ptr();

// @safe
void test_ptr_deref() {
    rusty::Ptr<int> p = get_ptr();
    int val = *p;  // Should ERROR: dereference in @safe
}
"#;

    let temp_file = create_temp_file("ptr_deref", code);
    let output = run_analyzer(&temp_file, "include");
    cleanup(&temp_file);

    assert!(
        output.contains("dereference") || output.contains("unsafe"),
        "Ptr dereference should require unsafe. Output: {}",
        output
    );
}

#[test]
fn test_mutptr_dereference_requires_unsafe() {
    let code = r#"
#include "rusty/ptr.hpp"

// @unsafe
rusty::MutPtr<int> get_mut_ptr();

// @safe
void test_mutptr_deref() {
    rusty::MutPtr<int> mp = get_mut_ptr();
    *mp = 10;  // Should ERROR: dereference in @safe
}
"#;

    let temp_file = create_temp_file("mutptr_deref", code);
    let output = run_analyzer(&temp_file, "include");
    cleanup(&temp_file);

    assert!(
        output.contains("dereference") || output.contains("unsafe"),
        "MutPtr dereference should require unsafe. Output: {}",
        output
    );
}

// =============================================================================
// Unsafe block allows Ptr/MutPtr operations
// =============================================================================

#[test]
fn test_ptr_operations_allowed_in_unsafe_block() {
    let code = r#"
#include "rusty/ptr.hpp"

// @safe
void test_ptr_unsafe_block() {
    int x = 42;

    // @unsafe
    {
        rusty::Ptr<int> p = &x;  // OK in unsafe block
        int val = *p;            // OK in unsafe block
    }
}
"#;

    let temp_file = create_temp_file("ptr_unsafe_block", code);
    let output = run_analyzer(&temp_file, "include");
    cleanup(&temp_file);

    // Should not have address-of or dereference errors
    assert!(
        !output.contains("address-of") && !output.contains("dereference"),
        "Ptr operations should be allowed in unsafe block. Output: {}",
        output
    );
}

#[test]
fn test_mutptr_operations_allowed_in_unsafe_block() {
    let code = r#"
#include "rusty/ptr.hpp"

// @safe
void test_mutptr_unsafe_block() {
    int x = 42;

    // @unsafe
    {
        rusty::MutPtr<int> mp = &x;  // OK in unsafe block
        *mp = 100;                    // OK in unsafe block
    }
}
"#;

    let temp_file = create_temp_file("mutptr_unsafe_block", code);
    let output = run_analyzer(&temp_file, "include");
    cleanup(&temp_file);

    // Should not have address-of or dereference errors
    assert!(
        !output.contains("address-of") && !output.contains("dereference"),
        "MutPtr operations should be allowed in unsafe block. Output: {}",
        output
    );
}

// =============================================================================
// Type alias resolution - no hardcoding needed
// =============================================================================

#[test]
fn test_ptr_and_mutptr_borrow_conflict() {
    // This test verifies that Ptr<T> and MutPtr<T> obey borrow rules.
    // - Ptr<int> p = &x creates an immutable borrow of x
    // - MutPtr<int> mp = &x creates a mutable borrow of x
    // Having both at the same time violates borrow rules!
    let code = r#"
#include "rusty/ptr.hpp"

// @safe
void test_both_types() {
    int x = 42;

    // Address-of is SAFE when assigning to safe pointer types
    rusty::Ptr<int> p = &x;      // OK: immutable borrow of x
    rusty::MutPtr<int> mp = &x;  // ERROR: mutable borrow conflicts with existing immutable borrow

    // Dereferencing safe pointers (Ptr/MutPtr) is also SAFE
    int a = *p;   // OK: safe pointer dereference
    int b = *mp;  // OK: safe pointer dereference
}
"#;

    let temp_file = create_temp_file("canonical_types", code);
    let output = run_analyzer(&temp_file, "include");
    cleanup(&temp_file);

    // Should detect borrow conflict: can't have mutable borrow while immutable borrow exists
    let has_borrow_conflict = output.contains("already") && output.contains("borrow");

    assert!(
        has_borrow_conflict,
        "Should detect borrow conflict when Ptr (immutable) and MutPtr (mutable) both borrow same variable. Output: {}",
        output
    );
}

// =============================================================================
// Ptr vs MutPtr semantic difference
// =============================================================================

#[test]
fn test_ptr_is_const_pointer() {
    // Ptr<T> = const T* means you cannot modify through it
    // This is enforced by the C++ type system, not the analyzer
    // The analyzer just checks that pointer ops are in unsafe blocks
    let code = r#"
#include "rusty/ptr.hpp"

// @unsafe
void test_ptr_constness() {
    int x = 42;
    rusty::Ptr<int> p = &x;
    // *p = 100;  // Would be compile error: cannot assign to const
    int val = *p;  // OK: reading is allowed
}
"#;

    let temp_file = create_temp_file("ptr_const", code);
    let output = run_analyzer(&temp_file, "include");
    cleanup(&temp_file);

    // Should have no violations in @unsafe code
    assert!(
        output.contains("no violations") || !output.contains("violation"),
        "Unsafe function should have no analyzer violations. Output: {}",
        output
    );
}

#[test]
fn test_mutptr_allows_mutation() {
    // MutPtr<T> = T* allows modification through it
    let code = r#"
#include "rusty/ptr.hpp"

// @unsafe
void test_mutptr_mutation() {
    int x = 42;
    rusty::MutPtr<int> mp = &x;
    *mp = 100;  // OK in @unsafe: MutPtr allows mutation
    int val = *mp;
}
"#;

    let temp_file = create_temp_file("mutptr_mutation", code);
    let output = run_analyzer(&temp_file, "include");
    cleanup(&temp_file);

    // Should have no violations in @unsafe code
    assert!(
        output.contains("no violations") || !output.contains("violation"),
        "Unsafe function should have no analyzer violations. Output: {}",
        output
    );
}

// =============================================================================
// Borrow rule tests for Ptr<T> and MutPtr<T>
// =============================================================================

#[test]
fn test_ptr_single_immutable_borrow_allowed() {
    // A single immutable borrow via Ptr<T> should be allowed
    let code = r#"
#include "rusty/ptr.hpp"

// @safe
void test_single_ptr() {
    int x = 42;
    rusty::Ptr<int> p = &x;  // OK: single immutable borrow
    int a = *p;              // OK: use the borrow
}
"#;

    let temp_file = create_temp_file("single_ptr", code);
    let output = run_analyzer(&temp_file, "include");
    cleanup(&temp_file);

    assert!(
        !output.contains("borrow") || output.contains("no violations"),
        "Single Ptr borrow should be allowed. Output: {}",
        output
    );
}

#[test]
fn test_mutptr_single_mutable_borrow_allowed() {
    // A single mutable borrow via MutPtr<T> should be allowed
    let code = r#"
#include "rusty/ptr.hpp"

// @safe
void test_single_mutptr() {
    int x = 42;
    rusty::MutPtr<int> mp = &x;  // OK: single mutable borrow
    *mp = 100;                   // OK: mutate through the borrow
}
"#;

    let temp_file = create_temp_file("single_mutptr", code);
    let output = run_analyzer(&temp_file, "include");
    cleanup(&temp_file);

    assert!(
        !output.contains("borrow") || output.contains("no violations"),
        "Single MutPtr borrow should be allowed. Output: {}",
        output
    );
}

#[test]
fn test_multiple_ptr_immutable_borrows_allowed() {
    // Multiple immutable borrows via Ptr<T> should be allowed (like Rust's &T)
    let code = r#"
#include "rusty/ptr.hpp"

// @safe
void test_multiple_ptrs() {
    int x = 42;
    rusty::Ptr<int> p1 = &x;  // OK: immutable borrow
    rusty::Ptr<int> p2 = &x;  // OK: another immutable borrow is allowed
    int a = *p1;
    int b = *p2;
}
"#;

    let temp_file = create_temp_file("multiple_ptrs", code);
    let output = run_analyzer(&temp_file, "include");
    cleanup(&temp_file);

    assert!(
        !output.contains("already") || output.contains("no violations"),
        "Multiple Ptr (immutable) borrows should be allowed. Output: {}",
        output
    );
}

#[test]
fn test_mutptr_then_ptr_conflict() {
    // MutPtr then Ptr should also conflict (mutable borrow exists)
    let code = r#"
#include "rusty/ptr.hpp"

// @safe
void test_mutptr_then_ptr() {
    int x = 42;
    rusty::MutPtr<int> mp = &x;  // Mutable borrow
    rusty::Ptr<int> p = &x;      // ERROR: can't immutable borrow while mutably borrowed
    int a = *p;
}
"#;

    let temp_file = create_temp_file("mutptr_then_ptr", code);
    let output = run_analyzer(&temp_file, "include");
    cleanup(&temp_file);

    // Should detect borrow conflict
    let has_borrow_conflict = output.contains("already") && output.contains("borrow");

    assert!(
        has_borrow_conflict,
        "Should detect borrow conflict when Ptr tries to borrow while MutPtr has mutable borrow. Output: {}",
        output
    );
}

#[test]
fn test_two_mutptr_conflict() {
    // Two MutPtr to same variable should conflict
    let code = r#"
#include "rusty/ptr.hpp"

// @safe
void test_two_mutptrs() {
    int x = 42;
    rusty::MutPtr<int> mp1 = &x;  // Mutable borrow
    rusty::MutPtr<int> mp2 = &x;  // ERROR: can't have two mutable borrows
    *mp1 = 1;
    *mp2 = 2;
}
"#;

    let temp_file = create_temp_file("two_mutptrs", code);
    let output = run_analyzer(&temp_file, "include");
    cleanup(&temp_file);

    // Should detect borrow conflict
    let has_borrow_conflict = output.contains("already") && output.contains("borrow");

    assert!(
        has_borrow_conflict,
        "Should detect borrow conflict when two MutPtrs try to mutably borrow same variable. Output: {}",
        output
    );
}
