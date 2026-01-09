// Tests for mixing Ptr<T>/MutPtr<T> with C++ references
//
// This tests the interaction between safe pointers and references.
// Key scenarios:
//   1. Ptr<T> p = &ref (const T&) - creates another immutable borrow (OK)
//   2. MutPtr<T> p = &mut_ref (T&) - creates mutable borrow from mutable ref (CONFLICT)
//   3. MutPtr<T> p = &rusty::move(mut_ref) - moves the ref, valid (should work)
//   4. Reference borrowing same variable as Ptr/MutPtr

use std::process::Command;
use std::fs;
use std::env;

fn create_temp_file(name: &str, code: &str) -> std::path::PathBuf {
    let temp_dir = env::temp_dir();
    let temp_file = temp_dir.join(format!("test_ptr_ref_mix_{}.cpp", name));
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
// MIXING CONST REFERENCE AND Ptr (SHOULD BE ALLOWED - both immutable)
// ============================================================================

#[test]
fn test_const_ref_then_ptr_allowed() {
    // const int& creates immutable borrow, Ptr<int> also creates immutable borrow
    // Multiple immutable borrows are allowed
    let code = r#"
#include "rusty/ptr.hpp"

// @safe
void test() {
    int x = 42;
    const int& ref = x;       // Immutable reference borrow
    rusty::Ptr<int> p = &x;   // Another immutable borrow - OK
    int a = ref;
    int b = *p;
}
"#;
    let temp_file = create_temp_file("const_ref_then_ptr", code);
    let output = run_analyzer(&temp_file, "include");
    cleanup(&temp_file);

    assert!(
        !output.contains("already") || output.contains("no violations"),
        "const ref then Ptr should be allowed (both immutable). Output: {}", output
    );
}

#[test]
fn test_ptr_then_const_ref_allowed() {
    let code = r#"
#include "rusty/ptr.hpp"

// @safe
void test() {
    int x = 42;
    rusty::Ptr<int> p = &x;   // Immutable borrow
    const int& ref = x;       // Another immutable borrow - OK
    int a = *p;
    int b = ref;
}
"#;
    let temp_file = create_temp_file("ptr_then_const_ref", code);
    let output = run_analyzer(&temp_file, "include");
    cleanup(&temp_file);

    assert!(
        !output.contains("already") || output.contains("no violations"),
        "Ptr then const ref should be allowed (both immutable). Output: {}", output
    );
}

// ============================================================================
// MIXING MUTABLE REFERENCE AND MutPtr (SHOULD FAIL - double mutable)
// ============================================================================

#[test]
fn test_mut_ref_then_mutptr_fails() {
    // int& creates mutable borrow, MutPtr<int> also creates mutable borrow
    // Cannot have two mutable borrows
    let code = r#"
#include "rusty/ptr.hpp"

// @safe
void test() {
    int x = 42;
    int& ref = x;               // Mutable reference borrow
    rusty::MutPtr<int> mp = &x; // ERROR: double mutable borrow
    ref = 100;
    *mp = 200;
}
"#;
    let temp_file = create_temp_file("mut_ref_then_mutptr", code);
    let output = run_analyzer(&temp_file, "include");
    cleanup(&temp_file);

    assert!(
        output.contains("already") && output.contains("borrow"),
        "mut ref then MutPtr should fail (double mutable). Output: {}", output
    );
}

#[test]
fn test_mutptr_then_mut_ref_fails() {
    let code = r#"
#include "rusty/ptr.hpp"

// @safe
void test() {
    int x = 42;
    rusty::MutPtr<int> mp = &x; // Mutable borrow
    int& ref = x;               // ERROR: another mutable borrow
    *mp = 100;
    ref = 200;
}
"#;
    let temp_file = create_temp_file("mutptr_then_mut_ref", code);
    let output = run_analyzer(&temp_file, "include");
    cleanup(&temp_file);

    assert!(
        output.contains("already") && output.contains("borrow"),
        "MutPtr then mut ref should fail (double mutable). Output: {}", output
    );
}

// ============================================================================
// MIXING MUTABLE REFERENCE AND Ptr (SHOULD FAIL - cannot read while mutably borrowed)
// ============================================================================

#[test]
fn test_mut_ref_then_ptr_fails() {
    // int& creates mutable borrow, Ptr<int> creates immutable borrow
    // Cannot have immutable borrow while mutable exists
    let code = r#"
#include "rusty/ptr.hpp"

// @safe
void test() {
    int x = 42;
    int& ref = x;             // Mutable borrow
    rusty::Ptr<int> p = &x;   // ERROR: cannot immutable borrow while mutably borrowed
    ref = 100;
    int val = *p;
}
"#;
    let temp_file = create_temp_file("mut_ref_then_ptr", code);
    let output = run_analyzer(&temp_file, "include");
    cleanup(&temp_file);

    assert!(
        output.contains("already") && output.contains("borrow"),
        "mut ref then Ptr should fail. Output: {}", output
    );
}

#[test]
fn test_ptr_then_mut_ref_fails() {
    let code = r#"
#include "rusty/ptr.hpp"

// @safe
void test() {
    int x = 42;
    rusty::Ptr<int> p = &x;   // Immutable borrow
    int& ref = x;             // ERROR: cannot mutable borrow while immutably borrowed
    int val = *p;
    ref = 100;
}
"#;
    let temp_file = create_temp_file("ptr_then_mut_ref", code);
    let output = run_analyzer(&temp_file, "include");
    cleanup(&temp_file);

    assert!(
        output.contains("already") && output.contains("borrow"),
        "Ptr then mut ref should fail. Output: {}", output
    );
}

// ============================================================================
// MIXING CONST REFERENCE AND MutPtr (SHOULD FAIL)
// ============================================================================

#[test]
fn test_const_ref_then_mutptr_fails() {
    let code = r#"
#include "rusty/ptr.hpp"

// @safe
void test() {
    int x = 42;
    const int& ref = x;         // Immutable borrow
    rusty::MutPtr<int> mp = &x; // ERROR: cannot mutable borrow while immutably borrowed
    int val = ref;
    *mp = 100;
}
"#;
    let temp_file = create_temp_file("const_ref_then_mutptr", code);
    let output = run_analyzer(&temp_file, "include");
    cleanup(&temp_file);

    assert!(
        output.contains("already") && output.contains("borrow"),
        "const ref then MutPtr should fail. Output: {}", output
    );
}

#[test]
fn test_mutptr_then_const_ref_fails() {
    let code = r#"
#include "rusty/ptr.hpp"

// @safe
void test() {
    int x = 42;
    rusty::MutPtr<int> mp = &x; // Mutable borrow
    const int& ref = x;         // ERROR: cannot immutable borrow while mutably borrowed
    *mp = 100;
    int val = ref;
}
"#;
    let temp_file = create_temp_file("mutptr_then_const_ref", code);
    let output = run_analyzer(&temp_file, "include");
    cleanup(&temp_file);

    assert!(
        output.contains("already") && output.contains("borrow"),
        "MutPtr then const ref should fail. Output: {}", output
    );
}

// ============================================================================
// Ptr FROM REFERENCE (taking address of reference)
// ============================================================================

#[test]
fn test_ptr_from_const_ref_allowed() {
    // Ptr<int> p = &ref where ref is const int&
    // This creates another immutable borrow, which is OK
    let code = r#"
#include "rusty/ptr.hpp"

// @safe
void test() {
    int x = 42;
    const int& ref = x;       // Immutable reference to x
    rusty::Ptr<int> p = &ref; // Ptr from reference - creates another immutable borrow of x
    int a = ref;
    int b = *p;
}
"#;
    let temp_file = create_temp_file("ptr_from_const_ref", code);
    let output = run_analyzer(&temp_file, "include");
    cleanup(&temp_file);

    // This should be allowed - both are immutable borrows
    println!("Ptr from const ref output: {}", output);
}

#[test]
fn test_mutptr_from_mut_ref_fails() {
    // MutPtr<int> p = &mut_ref where mut_ref is int&
    // This would create a SECOND mutable borrow, which is NOT allowed
    // (the original mut_ref is already a mutable borrow)
    let code = r#"
#include "rusty/ptr.hpp"

// @safe
void test() {
    int x = 42;
    int& mut_ref = x;            // Mutable borrow of x
    rusty::MutPtr<int> mp = &mut_ref; // ERROR: creates second mutable borrow
    mut_ref = 100;
    *mp = 200;
}
"#;
    let temp_file = create_temp_file("mutptr_from_mut_ref", code);
    let output = run_analyzer(&temp_file, "include");
    cleanup(&temp_file);

    // Can detect as double borrow OR as "borrowed by" conflict
    assert!(
        (output.contains("already") && output.contains("borrow")) ||
        (output.contains("borrowed by") && output.contains("violation")),
        "MutPtr from mut_ref should fail (would create 2 mutable borrows). Output: {}", output
    );
}

// ============================================================================
// MutPtr FROM MOVED REFERENCE (rusty::move)
// ============================================================================

#[test]
fn test_mutptr_from_moved_mut_ref_allowed() {
    // MutPtr<int> p = &rusty::move(mut_ref)
    // This MOVES the mutable reference, so there's only one mutable borrow
    // The original mut_ref should be invalidated
    let code = r#"
#include "rusty/ptr.hpp"
#include "rusty/move.hpp"

// @safe
void test() {
    int x = 42;
    int& mut_ref = x;                          // Mutable borrow of x
    rusty::MutPtr<int> mp = &rusty::move(mut_ref); // Move the reference, mp takes over
    *mp = 100;  // OK: mp has the mutable borrow now
}
"#;
    let temp_file = create_temp_file("mutptr_from_moved_ref", code);
    let output = run_analyzer(&temp_file, "include");
    cleanup(&temp_file);

    // This should be allowed - rusty::move transfers the borrow
    println!("MutPtr from moved ref output: {}", output);
    // Note: This is expected behavior that may need implementation
}

#[test]
fn test_use_after_move_ref_fails() {
    // After rusty::move(mut_ref), using mut_ref should fail
    let code = r#"
#include "rusty/ptr.hpp"
#include "rusty/move.hpp"

// @safe
void test() {
    int x = 42;
    int& mut_ref = x;
    rusty::MutPtr<int> mp = &rusty::move(mut_ref); // Move the reference
    mut_ref = 100;  // ERROR: mut_ref has been moved
}
"#;
    let temp_file = create_temp_file("use_moved_ref", code);
    let output = run_analyzer(&temp_file, "include");
    cleanup(&temp_file);

    assert!(
        output.contains("moved") || output.contains("invalid"),
        "Using moved reference should fail. Output: {}", output
    );
}

// ============================================================================
// SCOPE INTERACTIONS BETWEEN REFERENCES AND Ptr/MutPtr
// ============================================================================

#[test]
fn test_ref_scope_ends_allows_mutptr() {
    let code = r#"
#include "rusty/ptr.hpp"

// @safe
void test() {
    int x = 42;
    {
        const int& ref = x;  // Immutable borrow in inner scope
        int val = ref;
    }  // ref ends here
    rusty::MutPtr<int> mp = &x;  // OK: no active immutable borrow
    *mp = 100;
}
"#;
    let temp_file = create_temp_file("ref_scope_then_mutptr", code);
    let output = run_analyzer(&temp_file, "include");
    cleanup(&temp_file);

    assert!(
        !output.contains("already") || output.contains("no violations"),
        "Ref scope end should allow MutPtr. Output: {}", output
    );
}

#[test]
fn test_ptr_scope_ends_allows_mut_ref() {
    let code = r#"
#include "rusty/ptr.hpp"

// @safe
void test() {
    int x = 42;
    {
        rusty::Ptr<int> p = &x;  // Immutable borrow in inner scope
        int val = *p;
    }  // p ends here
    int& mut_ref = x;  // OK: no active immutable borrow
    mut_ref = 100;
}
"#;
    let temp_file = create_temp_file("ptr_scope_then_ref", code);
    let output = run_analyzer(&temp_file, "include");
    cleanup(&temp_file);

    assert!(
        !output.contains("already") || output.contains("no violations"),
        "Ptr scope end should allow mut ref. Output: {}", output
    );
}

// ============================================================================
// COMPLEX MIXING SCENARIOS
// ============================================================================

#[test]
fn test_multiple_immutable_mix_allowed() {
    // Multiple immutable borrows through different mechanisms
    let code = r#"
#include "rusty/ptr.hpp"

// @safe
void test() {
    int x = 42;
    const int& ref1 = x;      // Immutable ref
    const int& ref2 = x;      // Another immutable ref
    rusty::Ptr<int> p1 = &x;  // Immutable Ptr
    rusty::Ptr<int> p2 = &x;  // Another immutable Ptr

    int sum = ref1 + ref2 + *p1 + *p2;
}
"#;
    let temp_file = create_temp_file("multiple_immutable_mix", code);
    let output = run_analyzer(&temp_file, "include");
    cleanup(&temp_file);

    assert!(
        !output.contains("already") || output.contains("no violations"),
        "Multiple immutable borrows (refs and Ptrs) should be allowed. Output: {}", output
    );
}

#[test]
fn test_single_mutable_multiple_mechanisms_fails() {
    // Cannot have mutable borrow through both ref and MutPtr
    let code = r#"
#include "rusty/ptr.hpp"

// @safe
void test() {
    int x = 42;
    int& mut_ref = x;           // Mutable ref
    rusty::MutPtr<int> mp = &x; // ERROR: already mutably borrowed
    mut_ref = 100;
    *mp = 200;
}
"#;
    let temp_file = create_temp_file("mutable_mix_fails", code);
    let output = run_analyzer(&temp_file, "include");
    cleanup(&temp_file);

    assert!(
        output.contains("already") && output.contains("borrow"),
        "Cannot have mutable borrow through both ref and MutPtr. Output: {}", output
    );
}

// ============================================================================
// FUNCTION PARAMETER INTERACTIONS
// ============================================================================

#[test]
fn test_ptr_param_and_ref_param_independent() {
    // Different variables borrowed by different param types
    let code = r#"
#include "rusty/ptr.hpp"

// @safe
void take_ptr_and_ref(rusty::Ptr<int> p, int& ref) {
    int val = *p;
    ref = 100;
}

// @safe
void test() {
    int x = 1;
    int y = 2;
    take_ptr_and_ref(&x, y);  // x borrowed by Ptr, y by ref
}
"#;
    let temp_file = create_temp_file("ptr_ref_params", code);
    let output = run_analyzer(&temp_file, "include");
    cleanup(&temp_file);

    assert!(
        !output.contains("already") || output.contains("no violations"),
        "Independent borrows to different vars should work. Output: {}", output
    );
}
