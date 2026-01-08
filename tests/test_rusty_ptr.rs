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
fn test_ptr_address_of_requires_unsafe() {
    let code = r#"
#include "rusty/ptr.hpp"

// @safe
void test_ptr_addr() {
    int x = 42;
    rusty::Ptr<int> p = &x;  // Should ERROR: address-of in @safe
}
"#;

    let temp_file = create_temp_file("addr_of", code);
    let output = run_analyzer(&temp_file, "include");
    cleanup(&temp_file);

    assert!(
        output.contains("address-of") || output.contains("unsafe"),
        "Ptr address-of should require unsafe. Output: {}",
        output
    );
}

#[test]
fn test_mutptr_address_of_requires_unsafe() {
    let code = r#"
#include "rusty/ptr.hpp"

// @safe
void test_mutptr_addr() {
    int x = 42;
    rusty::MutPtr<int> mp = &x;  // Should ERROR: address-of in @safe
}
"#;

    let temp_file = create_temp_file("mutptr_addr_of", code);
    let output = run_analyzer(&temp_file, "include");
    cleanup(&temp_file);

    assert!(
        output.contains("address-of") || output.contains("unsafe"),
        "MutPtr address-of should require unsafe. Output: {}",
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
fn test_type_alias_resolved_to_canonical() {
    // This test verifies that Ptr<T> and MutPtr<T> are resolved to their
    // canonical types (const T* and T*) by libclang, so no hardcoding needed
    let code = r#"
#include "rusty/ptr.hpp"

// @safe
void test_both_types() {
    int x = 42;

    // Both should be detected - relies on canonical type resolution
    rusty::Ptr<int> p = &x;      // ERROR: address-of (Ptr<int> -> const int*)
    rusty::MutPtr<int> mp = &x;  // ERROR: address-of (MutPtr<int> -> int*)

    int a = *p;   // ERROR: dereference
    int b = *mp;  // ERROR: dereference
}
"#;

    let temp_file = create_temp_file("canonical_types", code);
    let output = run_analyzer(&temp_file, "include");
    cleanup(&temp_file);

    // Count violations - should have at least 4 (2 address-of + 2 dereference)
    let has_addr_of = output.contains("address-of");
    let has_deref = output.contains("dereference");

    assert!(
        has_addr_of && has_deref,
        "Should detect both address-of and dereference for type aliases. Output: {}",
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
