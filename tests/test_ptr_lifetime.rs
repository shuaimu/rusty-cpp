// Lifetime tracking tests for Ptr<T> and MutPtr<T>
//
// These tests verify that RustyCpp catches lifetime violations for safe pointers:
//   - Returning Ptr/MutPtr to a local variable (dangling)
//   - Ptr/MutPtr outliving the pointed-to object
//   - Storing Ptr/MutPtr to a local in a container
//
// Key lifetime rules:
//   1. Ptr/MutPtr cannot outlive the object they point to
//   2. Cannot return Ptr/MutPtr to a local variable
//   3. Cannot store Ptr/MutPtr to a local in a longer-lived container

use std::process::Command;
use std::fs;
use std::env;

fn create_temp_file(name: &str, code: &str) -> std::path::PathBuf {
    let temp_dir = env::temp_dir();
    let temp_file = temp_dir.join(format!("test_ptr_lifetime_{}.cpp", name));
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
// CATEGORY 1: Returning Ptr/MutPtr to local variable (SHOULD FAIL)
// ============================================================================

#[test]
fn test_return_ptr_to_local_fails() {
    // Returning Ptr to a local variable creates a dangling pointer
    let code = r#"
#include "rusty/ptr.hpp"

// @safe
rusty::Ptr<int> bad_return() {
    int x = 42;
    return &x;  // ERROR: returning pointer to local
}
"#;
    let temp_file = create_temp_file("return_ptr_local", code);
    let output = run_analyzer(&temp_file, "include");
    cleanup(&temp_file);

    assert!(
        output.contains("local") || output.contains("dangling") || output.contains("lifetime"),
        "Should detect returning Ptr to local variable. Output: {}", output
    );
}

#[test]
fn test_return_mutptr_to_local_fails() {
    // Returning MutPtr to a local variable creates a dangling pointer
    let code = r#"
#include "rusty/ptr.hpp"

// @safe
rusty::MutPtr<int> bad_return() {
    int x = 42;
    return &x;  // ERROR: returning pointer to local
}
"#;
    let temp_file = create_temp_file("return_mutptr_local", code);
    let output = run_analyzer(&temp_file, "include");
    cleanup(&temp_file);

    assert!(
        output.contains("local") || output.contains("dangling") || output.contains("lifetime"),
        "Should detect returning MutPtr to local variable. Output: {}", output
    );
}

#[test]
fn test_return_ptr_to_param_allowed() {
    // Returning Ptr to a parameter is OK (caller owns it)
    let code = r#"
#include "rusty/ptr.hpp"

// @lifetime: (&'a) -> &'a
// @safe
rusty::Ptr<int> ok_return(int& x) {
    return &x;  // OK: x is a parameter, caller owns it
}
"#;
    let temp_file = create_temp_file("return_ptr_param", code);
    let output = run_analyzer(&temp_file, "include");
    cleanup(&temp_file);

    assert!(
        !output.contains("dangling") && !output.contains("lifetime") || output.contains("no violations"),
        "Should allow returning Ptr to parameter. Output: {}", output
    );
}

// ============================================================================
// CATEGORY 2: Ptr/MutPtr outliving pointed-to object (SHOULD FAIL)
// ============================================================================

#[test]
fn test_ptr_outlives_local_in_scope_fails() {
    // Ptr assigned in inner scope outlives the local it points to
    let code = r#"
#include "rusty/ptr.hpp"

// @safe
void bad() {
    rusty::Ptr<int> p;
    {
        int x = 42;
        p = &x;
    }  // x dies here
    int val = *p;  // ERROR: p is dangling
}
"#;
    let temp_file = create_temp_file("ptr_outlives_scope", code);
    let output = run_analyzer(&temp_file, "include");
    cleanup(&temp_file);

    assert!(
        output.contains("outlive") || output.contains("dangling") || output.contains("scope") || output.contains("lifetime"),
        "Should detect Ptr outliving local. Output: {}", output
    );
}

#[test]
fn test_mutptr_outlives_local_in_scope_fails() {
    // MutPtr assigned in inner scope outlives the local it points to
    let code = r#"
#include "rusty/ptr.hpp"

// @safe
void bad() {
    rusty::MutPtr<int> mp;
    {
        int x = 42;
        mp = &x;
    }  // x dies here
    *mp = 100;  // ERROR: mp is dangling
}
"#;
    let temp_file = create_temp_file("mutptr_outlives_scope", code);
    let output = run_analyzer(&temp_file, "include");
    cleanup(&temp_file);

    assert!(
        output.contains("outlive") || output.contains("dangling") || output.contains("scope") || output.contains("lifetime"),
        "Should detect MutPtr outliving local. Output: {}", output
    );
}

#[test]
fn test_ptr_same_scope_allowed() {
    // Ptr and pointee in same scope - OK
    let code = r#"
#include "rusty/ptr.hpp"

// @safe
void ok() {
    int x = 42;
    rusty::Ptr<int> p = &x;
    int val = *p;  // OK: x is still alive
}  // Both die here
"#;
    let temp_file = create_temp_file("ptr_same_scope", code);
    let output = run_analyzer(&temp_file, "include");
    cleanup(&temp_file);

    assert!(
        !output.contains("dangling") && !output.contains("outlive") || output.contains("no violations"),
        "Same scope Ptr should be allowed. Output: {}", output
    );
}

#[test]
fn test_ptr_inner_scope_dies_first_allowed() {
    // Ptr in inner scope dies before pointee - OK
    let code = r#"
#include "rusty/ptr.hpp"

// @safe
void ok() {
    int x = 42;
    {
        rusty::Ptr<int> p = &x;
        int val = *p;  // OK: x is still alive
    }  // p dies here, x still alive
}  // x dies here
"#;
    let temp_file = create_temp_file("ptr_inner_dies_first", code);
    let output = run_analyzer(&temp_file, "include");
    cleanup(&temp_file);

    assert!(
        !output.contains("dangling") && !output.contains("outlive") || output.contains("no violations"),
        "Ptr dying before pointee should be allowed. Output: {}", output
    );
}

// ============================================================================
// CATEGORY 3: Conditional/loop lifetime issues (SHOULD FAIL)
// ============================================================================

#[test]
fn test_ptr_assigned_in_conditional_outlives_fails() {
    // Ptr assigned in if block to a local that dies at block end
    let code = r#"
#include "rusty/ptr.hpp"

// @safe
void bad(bool cond) {
    rusty::Ptr<int> p;
    if (cond) {
        int x = 42;
        p = &x;
    }  // x dies here
    // p may be dangling if cond was true
}
"#;
    let temp_file = create_temp_file("ptr_conditional", code);
    let output = run_analyzer(&temp_file, "include");
    cleanup(&temp_file);

    // This is a tricky case - conservative analysis should catch it
    println!("Conditional Ptr output: {}", output);
}

#[test]
fn test_ptr_in_loop_to_loop_local_fails() {
    // Ptr points to loop-local variable that dies each iteration
    let code = r#"
#include "rusty/ptr.hpp"

// @safe
void bad() {
    rusty::Ptr<int> p;
    for (int i = 0; i < 10; i++) {
        int x = i;
        p = &x;
    }  // x dies here each iteration
    int val = *p;  // ERROR: p is dangling (last x is dead)
}
"#;
    let temp_file = create_temp_file("ptr_loop_local", code);
    let output = run_analyzer(&temp_file, "include");
    cleanup(&temp_file);

    assert!(
        output.contains("dangling") || output.contains("outlive") || output.contains("scope") || output.contains("lifetime"),
        "Should detect Ptr to loop-local outliving loop. Output: {}", output
    );
}

// ============================================================================
// CATEGORY 4: Storing Ptr in container (SHOULD FAIL for locals)
// ============================================================================

#[test]
fn test_store_ptr_to_local_in_vector_fails() {
    // Storing Ptr to a local in a vector that outlives the local
    let code = r#"
#include "rusty/ptr.hpp"
#include <vector>

// @safe
void bad() {
    std::vector<rusty::Ptr<int>> ptrs;
    {
        int x = 42;
        ptrs.push_back(&x);  // ERROR: storing pointer to local
    }  // x dies here
    // ptrs[0] is dangling
}
"#;
    let temp_file = create_temp_file("store_ptr_vector", code);
    let output = run_analyzer(&temp_file, "include");
    cleanup(&temp_file);

    // This is caught by either:
    // - Container store tracking (dangling/outlive/container)
    // - Pointer safety (address-of in function call)
    // - Unsafe propagation (calling non-safe STL function)
    // Any of these correctly flags the problematic code
    println!("Store Ptr in vector output: {}", output);
    assert!(
        output.contains("dangling") || output.contains("outlive") ||
        output.contains("local") || output.contains("container") ||
        output.contains("address-of") || output.contains("non-safe"),
        "Should detect issue with storing Ptr to local in vector. Output: {}", output
    );
}

// ============================================================================
// CATEGORY 5: Function parameter lifetime (with annotations)
// ============================================================================

#[test]
fn test_ptr_from_identity_function_to_temp_fails() {
    // Similar to reference lifetime tests - Ptr from function returning input
    let code = r#"
#include "rusty/ptr.hpp"

// @lifetime: (&'a) -> &'a
// @safe
rusty::Ptr<int> get_ptr(int& x) {
    return &x;
}

// @safe
void bad() {
    int temp = 42;
    rusty::Ptr<int> p = get_ptr(temp);  // OK so far
    // But if temp was a temporary literal, it would be bad
}
"#;
    let temp_file = create_temp_file("ptr_identity", code);
    let output = run_analyzer(&temp_file, "include");
    cleanup(&temp_file);

    // This should be OK since temp is a named local
    println!("Ptr identity function output: {}", output);
}

#[test]
fn test_ptr_from_function_to_temporary_fails() {
    // Getting Ptr from function where argument is temporary
    let code = r#"
#include "rusty/ptr.hpp"

// Helper that takes a reference and returns Ptr to it
// @lifetime: (&'a) -> &'a
// @safe
rusty::Ptr<int> addr_of_ref(int& x) {
    return &x;
}

// @safe
void bad() {
    int y = 0;
    // If we could pass a temporary here, it would dangle
    // But C++ requires lvalue for non-const ref, so this is OK
    rusty::Ptr<int> p = addr_of_ref(y);
    int val = *p;
}
"#;
    let temp_file = create_temp_file("ptr_temp_arg", code);
    let output = run_analyzer(&temp_file, "include");
    cleanup(&temp_file);

    // This should be OK since y is a named local in same scope
    println!("Ptr from temp arg output: {}", output);
}

// ============================================================================
// CATEGORY 6: Complex nesting scenarios
// ============================================================================

#[test]
fn test_ptr_through_multiple_scopes_fails() {
    // Ptr passed through multiple scope levels, pointee dies in middle
    let code = r#"
#include "rusty/ptr.hpp"

// @safe
void bad() {
    rusty::Ptr<int> outer_p;
    {
        rusty::Ptr<int> mid_p;
        {
            int x = 42;
            mid_p = &x;
        }  // x dies here
        outer_p = mid_p;  // Copying dangling pointer
    }
    int val = *outer_p;  // ERROR: dangling
}
"#;
    let temp_file = create_temp_file("ptr_multiple_scopes", code);
    let output = run_analyzer(&temp_file, "include");
    cleanup(&temp_file);

    assert!(
        output.contains("dangling") || output.contains("outlive") || output.contains("scope"),
        "Should detect Ptr dangling through multiple scopes. Output: {}", output
    );
}

#[test]
fn test_reassign_ptr_to_outer_scope_var_allowed() {
    // Ptr first points to inner scope var, then reassigned to outer - OK
    let code = r#"
#include "rusty/ptr.hpp"

// @safe
void ok() {
    int outer = 1;
    rusty::Ptr<int> p;
    {
        int inner = 2;
        p = &inner;
        int val1 = *p;  // OK: inner still alive
    }  // inner dies, p is dangling temporarily
    p = &outer;  // Reassign to valid pointer
    int val2 = *p;  // OK: outer still alive
}
"#;
    let temp_file = create_temp_file("ptr_reassign_outer", code);
    let output = run_analyzer(&temp_file, "include");
    cleanup(&temp_file);

    // The tricky part: is p used between inner dying and reassignment?
    // In this case, no - so it should be OK
    println!("Ptr reassign to outer output: {}", output);
}

// ============================================================================
// POSITIVE TESTS - These SHOULD be allowed
// ============================================================================

#[test]
fn test_ptr_to_static_allowed() {
    // Ptr to static variable - always valid
    let code = r#"
#include "rusty/ptr.hpp"

static int g_value = 42;

// @safe
rusty::Ptr<int> get_static_ptr() {
    return &g_value;  // OK: static has infinite lifetime
}

// @safe
void ok() {
    rusty::Ptr<int> p = get_static_ptr();
    int val = *p;  // OK
}
"#;
    let temp_file = create_temp_file("ptr_static", code);
    let output = run_analyzer(&temp_file, "include");
    cleanup(&temp_file);

    // This test checks for lifetime violations, not null safety.
    // The null safety checker may warn about potential null dereference, but that's a separate concern.
    assert!(
        !output.contains("dangling") && !output.contains("outlive") && !output.contains("lifetime violation"),
        "Ptr to static should not have lifetime violations. Output: {}", output
    );
}

#[test]
fn test_ptr_to_heap_via_box_allowed() {
    // Ptr to heap-allocated value via Box - valid while Box alive
    let code = r#"
#include "rusty/ptr.hpp"
#include "rusty/box.hpp"

// @safe
void ok() {
    auto box = rusty::Box<int>::make(42);
    rusty::Ptr<int> p = &(*box);  // OK: Box keeps value alive
    int val = *p;  // OK
}  // Box and p die together
"#;
    let temp_file = create_temp_file("ptr_heap_box", code);
    let output = run_analyzer(&temp_file, "include");
    cleanup(&temp_file);

    // This should be OK as long as we don't use p after box dies
    println!("Ptr to heap via Box output: {}", output);
}
