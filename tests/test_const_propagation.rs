//! Tests for const propagation through pointer members
//!
//! In @safe code, const should propagate through pointer members.
//! If you have `const Outer*`, accessing `ptr` member should give const semantics.

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
// PART 1: Basic const pointer parameter -> pointer member -> non-const method
// ============================================================================

#[test]
fn test_const_ptr_param_to_ptr_member_nonconst_call() {
    // Calling non-const method through pointer member of const object - ERROR
    let code = r#"
struct Inner {
    int value;
    void mutate() { value = 42; }
};

struct Outer {
    Inner* ptr;
};

// @safe
void bad(const Outer* outer) {
    outer->ptr->mutate();  // ERROR: outer is const, should propagate
}
"#;
    let output = run_checker(code);
    assert!(
        output.contains("const") || output.contains("Const") || output.contains("mutate"),
        "Should detect non-const method call through const path. Output: {}",
        output
    );
}

#[test]
fn test_const_ref_param_to_ptr_member_nonconst_call() {
    // Same but with const reference parameter
    let code = r#"
struct Inner {
    int value;
    void mutate() { value = 42; }
};

struct Outer {
    Inner* ptr;
};

// @safe
void bad(const Outer& outer) {
    outer.ptr->mutate();  // ERROR: outer is const ref
}
"#;
    let output = run_checker(code);
    assert!(
        output.contains("const") || output.contains("Const") || output.contains("mutate"),
        "Should detect non-const method call through const ref path. Output: {}",
        output
    );
}

// ============================================================================
// PART 2: const method 'this' -> pointer member -> non-const method
// ============================================================================

#[test]
fn test_const_method_this_to_ptr_member_nonconst_call() {
    // In const method, 'this' is const, should propagate to pointer members
    let code = r#"
struct Inner {
    int value;
    void mutate() { value = 42; }
};

struct Outer {
    Inner* ptr;

    // @safe
    void const_method() const {
        ptr->mutate();  // ERROR: 'this' is const in const method
    }
};
"#;
    let output = run_checker(code);
    assert!(
        output.contains("const") || output.contains("Const") || output.contains("mutate"),
        "Should detect non-const call through const 'this'. Output: {}",
        output
    );
}

// ============================================================================
// PART 3: Assignment through const path
// ============================================================================

#[test]
fn test_const_ptr_param_assignment_through_ptr_member() {
    // Assigning through pointer member of const object - ERROR
    let code = r#"
struct Inner {
    int value;
};

struct Outer {
    Inner* ptr;
};

// @safe
void bad(const Outer* outer) {
    outer->ptr->value = 42;  // ERROR: assigning through const path
}
"#;
    let output = run_checker(code);
    assert!(
        output.contains("const") || output.contains("Const") || output.contains("assign"),
        "Should detect assignment through const path. Output: {}",
        output
    );
}

// ============================================================================
// PART 4: Nested pointer access
// ============================================================================

#[test]
fn test_nested_ptr_const_propagation() {
    // Const should propagate through multiple levels of pointer indirection
    let code = r#"
struct C {
    int value;
    void mutate() { value = 42; }
};

struct B {
    C* c_ptr;
};

struct A {
    B* b_ptr;
};

// @safe
void bad(const A* a) {
    a->b_ptr->c_ptr->mutate();  // ERROR: const propagates through chain
}
"#;
    let output = run_checker(code);
    assert!(
        output.contains("const") || output.contains("Const") || output.contains("mutate"),
        "Should detect non-const call through nested const path. Output: {}",
        output
    );
}

// ============================================================================
// PART 5: OK cases - non-const access
// ============================================================================

#[test]
fn test_nonconst_ptr_param_ok() {
    // Non-const pointer parameter - mutations OK
    let code = r#"
struct Inner {
    int value;
    void mutate() { value = 42; }
};

struct Outer {
    Inner* ptr;
};

// @safe
void ok(Outer* outer) {
    outer->ptr->mutate();  // OK: outer is non-const
}
"#;
    let output = run_checker(code);
    assert!(
        output.contains("no violations") || !output.contains("const"),
        "Should allow mutation through non-const path. Output: {}",
        output
    );
}

#[test]
fn test_nonconst_method_ok() {
    // Non-const method - mutations OK
    let code = r#"
struct Inner {
    int value;
    void mutate() { value = 42; }
};

struct Outer {
    Inner* ptr;

    // @safe
    void non_const_method() {
        ptr->mutate();  // OK: 'this' is non-const
    }
};
"#;
    let output = run_checker(code);
    assert!(
        output.contains("no violations") || !output.contains("const propagation"),
        "Should allow mutation in non-const method. Output: {}",
        output
    );
}

// ============================================================================
// PART 6: OK cases - const method calls and reads
// ============================================================================

#[test]
fn test_const_ptr_param_const_method_ok() {
    // Calling const method through const path - OK
    let code = r#"
struct Inner {
    int value;
    int get_value() const { return value; }
};

struct Outer {
    Inner* ptr;
};

// @safe
void ok(const Outer* outer) {
    int x = outer->ptr->get_value();  // OK: get_value is const
}
"#;
    let output = run_checker(code);
    assert!(
        output.contains("no violations") || !output.contains("ERROR"),
        "Should allow const method call through const path. Output: {}",
        output
    );
}

#[test]
fn test_const_ptr_param_read_ok() {
    // Reading through const path - OK
    let code = r#"
struct Inner {
    int value;
};

struct Outer {
    Inner* ptr;
};

// @safe
void ok(const Outer* outer) {
    int x = outer->ptr->value;  // OK: reading is fine
}
"#;
    let output = run_checker(code);
    assert!(
        output.contains("no violations") || !output.contains("const"),
        "Should allow read through const path. Output: {}",
        output
    );
}

// ============================================================================
// PART 7: @unsafe block should bypass checks
// ============================================================================

#[test]
fn test_unsafe_block_bypasses_const_propagation() {
    // In @unsafe block, const propagation checks are skipped
    let code = r#"
struct Inner {
    int value;
    void mutate() { value = 42; }
};

struct Outer {
    Inner* ptr;
};

// @safe
void ok(const Outer* outer) {
    // @unsafe
    {
        outer->ptr->mutate();  // OK: in @unsafe block
    }
}
"#;
    let output = run_checker(code);
    assert!(
        output.contains("no violations") || !output.contains("ERROR"),
        "Should allow mutation in @unsafe block. Output: {}",
        output
    );
}

// ============================================================================
// PART 8: Edge cases
// ============================================================================

#[test]
fn test_local_const_ptr_propagation() {
    // Local const pointer should also propagate
    let code = r#"
struct Inner {
    int value;
    void mutate() { value = 42; }
};

struct Outer {
    Inner* ptr;
};

// @safe
void bad(Outer* outer) {
    const Outer* const_outer = outer;
    const_outer->ptr->mutate();  // ERROR: const_outer is const
}
"#;
    let output = run_checker(code);
    assert!(
        output.contains("const") || output.contains("Const") || output.contains("mutate"),
        "Should detect non-const call through local const pointer. Output: {}",
        output
    );
}

#[test]
fn test_const_propagation_with_arrow_and_dot() {
    // Both -> and . should propagate const
    let code = r#"
struct Inner {
    int value;
    void mutate() { value = 42; }
};

struct Middle {
    Inner inner;  // Not a pointer, but embedded
    Inner* ptr;   // Pointer member
};

struct Outer {
    Middle* mid;
};

// @safe
void bad(const Outer* outer) {
    outer->mid->ptr->mutate();  // ERROR: const propagates
}
"#;
    let output = run_checker(code);
    assert!(
        output.contains("const") || output.contains("Const") || output.contains("mutate"),
        "Should propagate const through mixed access. Output: {}",
        output
    );
}

// ============================================================================
// PART 9: Multiple pointer members
// ============================================================================

#[test]
fn test_multiple_ptr_members_const_propagation() {
    // All pointer members should have const propagated
    let code = r#"
struct Inner {
    int value;
    void mutate() { value = 42; }
    int get() const { return value; }
};

struct Outer {
    Inner* ptr1;
    Inner* ptr2;
};

// @safe
void bad(const Outer* outer) {
    outer->ptr1->mutate();  // ERROR
}

// @safe
void ok(const Outer* outer) {
    int a = outer->ptr1->get();  // OK: const method
    int b = outer->ptr2->get();  // OK: const method
}
"#;
    let output = run_checker(code);
    // Should have at least one error for the bad function
    assert!(
        output.contains("const") || output.contains("Const") || output.contains("mutate"),
        "Should detect const violation in bad(). Output: {}",
        output
    );
}

// ============================================================================
// PART 10: Reference members (similar rules should apply)
// ============================================================================

#[test]
fn test_reference_member_const_propagation() {
    // Reference members should also have const propagated (if implemented)
    let code = r#"
struct Inner {
    int value;
    void mutate() { value = 42; }
};

struct Outer {
    Inner& ref;
    Outer(Inner& r) : ref(r) {}
};

// @safe
void maybe_bad(const Outer* outer) {
    outer->ref.mutate();  // Should this be ERROR? References are tricky
}
"#;
    let output = run_checker(code);
    // Note: This test documents behavior - references might be handled differently
    println!("Reference member const propagation output: {}", output);
}

// ============================================================================
// PART 11: Return value from const method
// ============================================================================

#[test]
fn test_return_ptr_from_const_method() {
    // If a const method returns a pointer to member, what can we do with it?
    let code = r#"
struct Inner {
    int value;
    void mutate() { value = 42; }
};

struct Outer {
    Inner* ptr;

    // Returns non-const pointer - but called on const object!
    Inner* get_ptr() const { return ptr; }
};

// @safe
void tricky(const Outer* outer) {
    Inner* p = outer->get_ptr();
    p->mutate();  // Is this an error? The pointer "escaped" const context
}
"#;
    let output = run_checker(code);
    // This is a tricky case - document the behavior
    println!("Return pointer from const method output: {}", output);
}
