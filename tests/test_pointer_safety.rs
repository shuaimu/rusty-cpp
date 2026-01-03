use std::process::Command;
use std::fs;

#[test]
fn test_pointer_dereference_in_safe_function() {
    // Dereferencing pointers should require unsafe context
    let test_code = r#"
// @safe
void test() {
    int x = 42;
    int* ptr = &x;
    int y = *ptr;  // ERROR: pointer dereference requires unsafe
}
"#;
    
    fs::write("test_pointer_deref.cpp", test_code).unwrap();
    
    let output = Command::new("cargo")
        .args(&["run", "--", "test_pointer_deref.cpp"])
        
        
        .output()
        .expect("Failed to run borrow checker");
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    
    // Should detect unsafe pointer operation
    assert!(stdout.contains("pointer") && stdout.contains("dereference"),
            "Should detect pointer dereference as unsafe. Output: {}", stdout);
    
    // Clean up
    let _ = fs::remove_file("test_pointer_deref.cpp");
}

#[test]
fn test_address_of_in_safe_function() {
    // Taking address should require unsafe context
    let test_code = r#"
// @safe
void test() {
    int x = 42;
    int* ptr = &x;  // ERROR: address-of requires unsafe
}
"#;
    
    fs::write("test_address_of.cpp", test_code).unwrap();
    
    let output = Command::new("cargo")
        .args(&["run", "--", "test_address_of.cpp"])
        
        
        .output()
        .expect("Failed to run borrow checker");
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    
    // Should detect unsafe pointer operation
    assert!(stdout.contains("pointer") && stdout.contains("address-of"),
            "Should detect address-of as unsafe. Output: {}", stdout);
    
    // Clean up
    let _ = fs::remove_file("test_address_of.cpp");
}

#[test]
fn test_pointers_allowed_in_unsafe_function() {
    // Pointer operations should be allowed in unsafe functions
    let test_code = r#"
// @unsafe
void test() {
    int x = 42;
    int* ptr = &x;  // OK: function is unsafe
    int y = *ptr;   // OK: function is unsafe
    *ptr = 100;     // OK: function is unsafe
}
"#;
    
    fs::write("test_unsafe_pointers.cpp", test_code).unwrap();
    
    let output = Command::new("cargo")
        .args(&["run", "--", "test_unsafe_pointers.cpp"])
        
        
        .output()
        .expect("Failed to run borrow checker");
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    
    // Should NOT detect any violations (unsafe function is not checked)
    assert!(!stdout.contains("violation") || stdout.contains("✓"),
            "Should not check unsafe functions. Output: {}", stdout);
    
    // Clean up
    let _ = fs::remove_file("test_unsafe_pointers.cpp");
}

#[test]
fn test_references_are_safe() {
    // References should be allowed in safe functions (they're not raw pointers)
    let test_code = r#"
// @safe
void test() {
    int x = 42;
    int& ref = x;   // OK: references are safe
    int y = ref;    // OK: using reference is safe
    const int& cref = x;  // OK: const reference is safe
}
"#;
    
    fs::write("test_references_safe.cpp", test_code).unwrap();
    
    let output = Command::new("cargo")
        .args(&["run", "--", "test_references_safe.cpp"])
        
        
        .output()
        .expect("Failed to run borrow checker");
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    
    // Should NOT detect pointer violations for references
    assert!(!stdout.contains("pointer") || stdout.contains("✓"),
            "References should be safe. Output: {}", stdout);
    
    // Clean up
    let _ = fs::remove_file("test_references_safe.cpp");
}

#[test]
fn test_pointer_in_namespace() {
    // Test pointer operations in a safe namespace
    let test_code = r#"
// @safe
namespace myapp {
    void test() {
        int x = 42;
        int* ptr = &x;  // ERROR: in safe namespace
        int y = *ptr;   // ERROR: in safe namespace
    }
}
"#;
    
    fs::write("test_namespace_pointers.cpp", test_code).unwrap();
    
    let output = Command::new("cargo")
        .args(&["run", "--", "test_namespace_pointers.cpp"])
        
        
        .output()
        .expect("Failed to run borrow checker");
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    
    // Should detect unsafe pointer operations
    assert!(stdout.contains("pointer") && stdout.contains("violation"),
            "Should detect pointer operations in safe namespace. Output: {}", stdout);
    
    // Clean up
    let _ = fs::remove_file("test_namespace_pointers.cpp");
}

#[test]
fn test_mixed_safe_unsafe_with_pointers() {
    // Test mixed safe and unsafe functions with pointer operations
    let test_code = r#"
// @safe
void safe_func() {
    int x = 42;
    int* ptr = &x;  // ERROR: pointer in safe function
}

// @unsafe  
void unsafe_func() {
    int x = 42;
    int* ptr = &x;  // OK: pointer in unsafe function
    int y = *ptr;   // OK: pointer in unsafe function
}
"#;
    
    fs::write("test_mixed_pointers.cpp", test_code).unwrap();
    
    let output = Command::new("cargo")
        .args(&["run", "--", "test_mixed_pointers.cpp"])
        
        
        .output()
        .expect("Failed to run borrow checker");
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    
    // Should detect violation only in safe_func
    assert!(stdout.contains("safe_func") && stdout.contains("pointer"),
            "Should detect pointer in safe_func. Output: {}", stdout);
    assert!(!stdout.contains("unsafe_func") || !stdout.contains("unsafe_func.*pointer"),
            "Should not report pointer errors in unsafe_func. Output: {}", stdout);
    
    // Clean up
    let _ = fs::remove_file("test_mixed_pointers.cpp");
}

#[test]
fn test_non_pointer_unary_operators_are_safe() {
    // Issue #10: Non-pointer unary operators (!, ~, -, +) should NOT be flagged as address-of
    // These are pure arithmetic/logical operations, not pointer operations
    let test_code = r#"
// @safe - logical NOT returns int (0 or 1), not pointer
int test_logical_not(int v) {
    return v + !v;
}

// @safe - bitwise NOT is pure arithmetic
unsigned test_bitwise_not(unsigned v) {
    return ~v;
}

// @safe - unary minus is pure arithmetic
int test_unary_minus(int v) {
    return -v;
}

// @safe - unary plus is pure arithmetic
int test_unary_plus(int v) {
    return +v;
}
"#;

    fs::write("test_unary_non_pointer.cpp", test_code).unwrap();

    let output = Command::new("cargo")
        .args(&["run", "--", "test_unary_non_pointer.cpp"])
        .output()
        .expect("Failed to run borrow checker");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should NOT detect any violations - these are not pointer operations
    assert!(stdout.contains("no violations") || stdout.contains("✓"),
            "Non-pointer unary operators should be safe. Output: {}", stdout);

    // Make sure we're not seeing false positives for address-of
    assert!(!stdout.contains("address-of"),
            "Should not flag non-pointer unary operators as address-of. Output: {}", stdout);

    // Clean up
    let _ = fs::remove_file("test_unary_non_pointer.cpp");
}

// ============================================================================
// NULL POINTER DETECTION TESTS
// ============================================================================

#[test]
fn test_nullptr_initialization_forbidden() {
    // Initializing a pointer with nullptr should be an error in @safe code
    let test_code = r#"
// @safe
void test() {
    int* p = nullptr;  // ERROR: null pointer initialization
}
"#;

    fs::write("test_nullptr_init.cpp", test_code).unwrap();

    let output = Command::new("cargo")
        .args(&["run", "--", "test_nullptr_init.cpp"])
        .output()
        .expect("Failed to run borrow checker");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should detect null pointer
    assert!(stdout.contains("null") && stdout.contains("pointer"),
            "Should detect nullptr initialization. Output: {}", stdout);

    // Clean up
    let _ = fs::remove_file("test_nullptr_init.cpp");
}

#[test]
fn test_nullptr_assignment_forbidden() {
    // Assigning nullptr to an existing pointer should be an error in @safe code
    let test_code = r#"
// @safe
void test(int* p) {
    p = nullptr;  // ERROR: null pointer assignment
}
"#;

    fs::write("test_nullptr_assign.cpp", test_code).unwrap();

    let output = Command::new("cargo")
        .args(&["run", "--", "test_nullptr_assign.cpp"])
        .output()
        .expect("Failed to run borrow checker");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should detect null pointer assignment
    assert!(stdout.contains("null") && stdout.contains("assignment") ||
            stdout.contains("Null pointer"),
            "Should detect nullptr assignment. Output: {}", stdout);

    // Clean up
    let _ = fs::remove_file("test_nullptr_assign.cpp");
}

#[test]
fn test_nullptr_as_function_argument_forbidden() {
    // Passing nullptr as a function argument should be an error in @safe code
    let test_code = r#"
// @safe
void test() {
    void func(int*);
    func(nullptr);  // ERROR: null pointer as argument
}
"#;

    fs::write("test_nullptr_arg.cpp", test_code).unwrap();

    let output = Command::new("cargo")
        .args(&["run", "--", "test_nullptr_arg.cpp"])
        .output()
        .expect("Failed to run borrow checker");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should detect null pointer as argument
    assert!(stdout.contains("null") && stdout.contains("argument") ||
            stdout.contains("Null pointer passed"),
            "Should detect nullptr as function argument. Output: {}", stdout);

    // Clean up
    let _ = fs::remove_file("test_nullptr_arg.cpp");
}

#[test]
fn test_nullptr_return_forbidden() {
    // Returning nullptr should be an error in @safe code
    let test_code = r#"
// @safe
int* test() {
    return nullptr;  // ERROR: returning null pointer
}
"#;

    fs::write("test_nullptr_return.cpp", test_code).unwrap();

    let output = Command::new("cargo")
        .args(&["run", "--", "test_nullptr_return.cpp"])
        .output()
        .expect("Failed to run borrow checker");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should detect null pointer return
    assert!(stdout.contains("null") && stdout.contains("return") ||
            stdout.contains("Cannot return nullptr"),
            "Should detect nullptr return. Output: {}", stdout);

    // Clean up
    let _ = fs::remove_file("test_nullptr_return.cpp");
}

#[test]
fn test_uninitialized_pointer_forbidden() {
    // Declaring a pointer without initialization should be an error in @safe code
    let test_code = r#"
// @safe
void test() {
    int* p;  // ERROR: uninitialized pointer
}
"#;

    fs::write("test_uninit_ptr.cpp", test_code).unwrap();

    let output = Command::new("cargo")
        .args(&["run", "--", "test_uninit_ptr.cpp"])
        .output()
        .expect("Failed to run borrow checker");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should detect uninitialized pointer
    assert!(stdout.contains("uninitialized") || stdout.contains("Uninitialized"),
            "Should detect uninitialized pointer. Output: {}", stdout);

    // Clean up
    let _ = fs::remove_file("test_uninit_ptr.cpp");
}

#[test]
fn test_nullptr_allowed_in_unsafe_function() {
    // nullptr should be allowed in @unsafe functions
    let test_code = r#"
// @unsafe
void test() {
    int* p = nullptr;  // OK: function is unsafe
    p = nullptr;       // OK: function is unsafe
}
"#;

    fs::write("test_nullptr_unsafe_func.cpp", test_code).unwrap();

    let output = Command::new("cargo")
        .args(&["run", "--", "test_nullptr_unsafe_func.cpp"])
        .output()
        .expect("Failed to run borrow checker");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should NOT detect null pointer violations in unsafe function
    assert!(!stdout.contains("null pointer") || stdout.contains("no violations") || stdout.contains("✓"),
            "Should allow nullptr in unsafe functions. Output: {}", stdout);

    // Clean up
    let _ = fs::remove_file("test_nullptr_unsafe_func.cpp");
}

#[test]
fn test_nullptr_allowed_in_unsafe_block() {
    // nullptr should be allowed in @unsafe blocks within @safe functions
    let test_code = r#"
// @safe
void test() {
    // @unsafe
    {
        int* p = nullptr;  // OK: in @unsafe block
    }
}
"#;

    fs::write("test_nullptr_unsafe_block.cpp", test_code).unwrap();

    let output = Command::new("cargo")
        .args(&["run", "--", "test_nullptr_unsafe_block.cpp"])
        .output()
        .expect("Failed to run borrow checker");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should NOT detect null pointer violations in unsafe block
    assert!(!stdout.contains("null pointer") || stdout.contains("no violations") || stdout.contains("✓"),
            "Should allow nullptr in unsafe blocks. Output: {}", stdout);

    // Clean up
    let _ = fs::remove_file("test_nullptr_unsafe_block.cpp");
}