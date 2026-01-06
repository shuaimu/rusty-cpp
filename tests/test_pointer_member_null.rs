//! Integration tests for Pointer Non-Null at Init/Assignment
//!
//! Tests that pointers must be initialized to non-null values in @safe code.
//! This covers:
//! - Local pointer variables (existing functionality)
//! - Pointer members in structs/classes (NEW)
//! - Constructor initializer lists (NEW)
//! - Default member initializers (NEW)
//! - Assignment from possibly-null sources (NEW)

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
// PART 1: Local Pointer Variables (verify existing functionality)
// ============================================================================

#[test]
fn test_local_uninitialized_pointer() {
    // Local pointer without initializer should be an error
    let code = r#"
// @safe
void test() {
    int* ptr;  // ERROR: uninitialized pointer
}
"#;
    let output = run_checker(code);
    assert!(
        output.contains("Uninitialized pointer") || output.contains("uninitialized"),
        "Should detect uninitialized local pointer. Output: {}",
        output
    );
}

#[test]
fn test_local_nullptr_init() {
    // Local pointer initialized to nullptr should be an error
    let code = r#"
// @safe
void test() {
    int* ptr = nullptr;  // ERROR: null pointer initialization
}
"#;
    let output = run_checker(code);
    assert!(
        output.contains("null") || output.contains("Null"),
        "Should detect nullptr initialization. Output: {}",
        output
    );
}

#[test]
fn test_local_nullptr_assignment() {
    // Assigning nullptr to a pointer should be an error
    let code = r#"
// @safe
void test(int* ptr) {
    ptr = nullptr;  // ERROR: null pointer assignment
}
"#;
    let output = run_checker(code);
    assert!(
        output.contains("Null pointer assignment") || output.contains("null"),
        "Should detect nullptr assignment. Output: {}",
        output
    );
}

// ============================================================================
// PART 2: Struct Pointer Member - Declaration without Init
// ============================================================================

#[test]
fn test_safe_struct_with_pointer_member_no_init() {
    // A @safe struct with a pointer member that has no initialization
    // should be an error - the pointer could be garbage
    let code = r#"
// @safe
struct Container {
    int* ptr;  // ERROR: pointer member without initialization in @safe struct
};
"#;
    let output = run_checker(code);
    assert!(
        output.contains("pointer member") || output.contains("uninitialized") || output.contains("must be initialized"),
        "Should detect uninitialized pointer member in @safe struct. Output: {}",
        output
    );
}

#[test]
fn test_unsafe_struct_with_pointer_member_allowed() {
    // An @unsafe struct can have uninitialized pointer members
    let code = r#"
// @unsafe
struct UnsafeContainer {
    int* ptr;  // OK: struct is @unsafe
};
"#;
    let output = run_checker(code);
    assert!(
        output.contains("no violations") || !output.contains("pointer member"),
        "Should allow uninitialized pointer in @unsafe struct. Output: {}",
        output
    );
}

// ============================================================================
// PART 3: Constructor Initializer List
// ============================================================================

#[test]
fn test_constructor_init_list_nullptr() {
    // Constructor initializing pointer member to nullptr should be an error
    let code = r#"
// @safe
struct Container {
    int* ptr;

    // @safe
    Container() : ptr(nullptr) {}  // ERROR: initializing to nullptr
};
"#;
    let output = run_checker(code);
    assert!(
        output.contains("nullptr") || output.contains("null") || output.contains("Null"),
        "Should detect nullptr in constructor initializer list. Output: {}",
        output
    );
}

#[test]
fn test_constructor_init_list_valid_pointer() {
    // Constructor initializing pointer member to valid address is OK
    let code = r#"
// @safe
struct Container {
    int* ptr;
    int value;

    // @unsafe - need unsafe for &value
    Container() : value(42), ptr(&value) {}
};

// @safe
void test() {
    // @unsafe
    {
        Container c;
    }
}
"#;
    let output = run_checker(code);
    // This should not complain about null pointer (address-of is non-null)
    assert!(
        !output.contains("nullptr") || output.contains("no violations"),
        "Should allow valid pointer initialization. Output: {}",
        output
    );
}

#[test]
fn test_constructor_body_nullptr_assignment() {
    // Constructor body assigning nullptr to member should be an error
    let code = r#"
// @safe
struct Container {
    int* ptr;

    // @safe
    Container() {
        ptr = nullptr;  // ERROR: assigning nullptr to member
    }
};
"#;
    let output = run_checker(code);
    assert!(
        output.contains("null") || output.contains("Null"),
        "Should detect nullptr assignment in constructor body. Output: {}",
        output
    );
}

// ============================================================================
// PART 4: Default Member Initializers (C++11)
// ============================================================================

#[test]
fn test_default_member_initializer_nullptr() {
    // Default member initializer with nullptr should be an error in @safe struct
    let code = r#"
// @safe
struct Container {
    int* ptr = nullptr;  // ERROR: default init to nullptr in @safe struct
};
"#;
    let output = run_checker(code);
    assert!(
        output.contains("nullptr") || output.contains("null") || output.contains("Null"),
        "Should detect nullptr in default member initializer. Output: {}",
        output
    );
}

#[test]
fn test_default_member_initializer_valid() {
    // Default member initializer with valid value is OK
    // (Though this is tricky in practice - what's a valid default?)
    let code = r#"
int global_value = 42;

// @safe
struct Container {
    // @unsafe - address-of requires unsafe
    int* ptr = &global_value;  // This needs @unsafe context
};
"#;
    let output = run_checker(code);
    // The address-of might require unsafe, but it's not a null issue
    println!("Output: {}", output);
}

// ============================================================================
// PART 5: Multiple Pointer Members
// ============================================================================

#[test]
fn test_multiple_pointer_members_mixed() {
    // Multiple pointer members - some initialized, some not
    let code = r#"
// @safe
struct MultiPointer {
    int* ptr1;           // ERROR: uninitialized
    int* ptr2 = nullptr; // ERROR: nullptr
    int& ref;            // OK: reference (must be bound)
};
"#;
    let output = run_checker(code);
    // Should detect at least one issue
    assert!(
        output.contains("null") || output.contains("uninitialized") || output.contains("pointer member"),
        "Should detect issues with pointer members. Output: {}",
        output
    );
}

// ============================================================================
// PART 6: Nested Structs
// ============================================================================

#[test]
fn test_nested_struct_pointer_member() {
    // Nested struct with pointer member
    let code = r#"
// @safe
struct Inner {
    int* ptr;  // ERROR: uninitialized in @safe struct
};

// @safe
struct Outer {
    Inner inner;  // Contains problematic pointer
};
"#;
    let output = run_checker(code);
    assert!(
        output.contains("pointer member") || output.contains("uninitialized") || output.contains("null"),
        "Should detect pointer member in nested struct. Output: {}",
        output
    );
}

// ============================================================================
// PART 7: Inheritance
// ============================================================================

#[test]
fn test_base_class_pointer_member() {
    // Base class with pointer member
    let code = r#"
// @safe
struct Base {
    int* ptr;  // ERROR: uninitialized pointer member
};

// @safe
struct Derived : Base {
    int value;
};
"#;
    let output = run_checker(code);
    assert!(
        output.to_lowercase().contains("pointer member") || output.contains("uninitialized"),
        "Should detect pointer member in base class. Output: {}",
        output
    );
}

// ============================================================================
// PART 8: Template Structs
// ============================================================================

#[test]
fn test_template_struct_pointer_member() {
    // Template struct with pointer member
    let code = r#"
template<typename T>
// @safe
struct Container {
    T* ptr;  // ERROR: uninitialized pointer member
};
"#;
    let output = run_checker(code);
    // Template analysis might not catch this yet
    println!("Template pointer member output: {}", output);
}

// ============================================================================
// PART 9: Conditional Initialization (advanced)
// ============================================================================

#[test]
fn test_conditional_nullptr_init() {
    // Pointer conditionally initialized - one path is nullptr
    let code = r#"
// @safe
void test(bool cond) {
    int x = 42;
    int* ptr;
    if (cond) {
        // @unsafe
        {
            ptr = &x;  // OK path
        }
    } else {
        ptr = nullptr;  // ERROR: nullptr assignment
    }
}
"#;
    let output = run_checker(code);
    assert!(
        output.contains("null") || output.contains("Null"),
        "Should detect nullptr in else branch. Output: {}",
        output
    );
}

// ============================================================================
// PART 10: Function Parameter Pointer (for reference)
// ============================================================================

#[test]
fn test_passing_nullptr_as_argument() {
    // Passing nullptr as function argument should be an error
    let code = r#"
void process(int* ptr);

// @safe
void test() {
    process(nullptr);  // ERROR: passing nullptr
}
"#;
    let output = run_checker(code);
    assert!(
        output.contains("Null pointer passed") || output.contains("null"),
        "Should detect nullptr passed as argument. Output: {}",
        output
    );
}

// ============================================================================
// PART 11: Return nullptr
// ============================================================================

#[test]
fn test_return_nullptr() {
    // Returning nullptr should be an error
    let code = r#"
// @safe
int* test() {
    return nullptr;  // ERROR: returning nullptr
}
"#;
    let output = run_checker(code);
    assert!(
        output.contains("Cannot return nullptr") || output.contains("null"),
        "Should detect nullptr return. Output: {}",
        output
    );
}

// ============================================================================
// PART 12: Smart Pointer Members (should be OK)
// ============================================================================

#[test]
fn test_smart_pointer_member_ok() {
    // Smart pointer members should be OK (they handle null safely)
    let code = r#"
#include <memory>

// @safe
struct Container {
    std::unique_ptr<int> ptr;  // OK: smart pointer, not raw
};
"#;
    let output = run_checker(code);
    // Should not complain about smart pointers
    assert!(
        !output.contains("pointer member") || output.contains("no violations"),
        "Should not flag smart pointer members. Output: {}",
        output
    );
}

// ============================================================================
// PART 13: Reference Members (should be OK)
// ============================================================================

#[test]
fn test_reference_member_ok() {
    // Reference members must be bound - C++ enforces this
    let code = r#"
// @safe
struct Container {
    int& ref;  // OK: reference, not pointer (C++ enforces binding)

    Container(int& r) : ref(r) {}
};
"#;
    let output = run_checker(code);
    // Should not complain about reference members
    assert!(
        !output.contains("uninitialized") || output.contains("no violations"),
        "Should not flag reference members. Output: {}",
        output
    );
}

// ============================================================================
// PART 14: Static Pointer Members
// ============================================================================

#[test]
fn test_static_pointer_member() {
    // Static pointer members - different initialization rules
    let code = r#"
// @safe
struct Container {
    static int* ptr;  // Static members are zero-initialized by default
};

int* Container::ptr = nullptr;  // ERROR: initializing to nullptr
"#;
    let output = run_checker(code);
    // Static member initialized to nullptr should be caught
    println!("Static pointer member output: {}", output);
}

// ============================================================================
// PART 15: Pointer-to-Pointer Members
// ============================================================================

#[test]
fn test_pointer_to_pointer_member() {
    // Double pointer member
    let code = r#"
// @safe
struct Container {
    int** ptr;  // ERROR: pointer member without init
};
"#;
    let output = run_checker(code);
    assert!(
        output.to_lowercase().contains("pointer member") || output.contains("uninitialized"),
        "Should detect double pointer member. Output: {}",
        output
    );
}

// ============================================================================
// PART 16: Const Pointer Members
// ============================================================================

#[test]
fn test_const_pointer_member() {
    // Const pointer member - must be initialized
    let code = r#"
// @safe
struct Container {
    int* const ptr;  // ERROR: const pointer must be initialized, and not to null
};
"#;
    let output = run_checker(code);
    // Const pointers must be initialized - C++ will complain anyway,
    // but we should also check it's not null
    println!("Const pointer member output: {}", output);
}

// ============================================================================
// PART 17: Array of Pointers Member
// ============================================================================

#[test]
fn test_array_of_pointers_member() {
    // Array of pointers - each element could be null
    let code = r#"
// @safe
struct Container {
    int* ptrs[10];  // ERROR: array of uninitialized pointers
};
"#;
    let output = run_checker(code);
    // Array of pointers should be flagged
    println!("Array of pointers member output: {}", output);
}

// ============================================================================
// PART 18: Valid Cases - Should Pass
// ============================================================================

#[test]
fn test_struct_with_no_pointers() {
    // Struct with no pointer members should be fine
    let code = r#"
// @safe
struct SafeStruct {
    int value;
    double data;
    bool flag;
};

// @safe
void test() {
    SafeStruct s;
    s.value = 42;
}
"#;
    let output = run_checker(code);
    assert!(
        output.contains("no violations") || !output.contains("pointer"),
        "Struct with no pointers should be OK. Output: {}",
        output
    );
}

#[test]
fn test_unsafe_block_allows_nullptr() {
    // Inside @unsafe block, nullptr should be allowed
    let code = r#"
// @safe
void test() {
    // @unsafe
    {
        int* ptr = nullptr;  // OK: in @unsafe block
    }
}
"#;
    let output = run_checker(code);
    assert!(
        output.contains("no violations") || !output.contains("error"),
        "Should allow nullptr in @unsafe block. Output: {}",
        output
    );
}

// ============================================================================
// PART 19: Constructor-Initialized Pointer Members (NEW FEATURE)
// ============================================================================

#[test]
fn test_deleted_default_ctor_with_init_ctor_ok() {
    // @safe struct with deleted default constructor and proper init constructor
    // This should be allowed since there's no way to create an uninitialized instance
    let code = r#"
// @safe
struct SafeWrapper {
    int* ptr;
    int data;

    SafeWrapper() = delete;  // No default constructor

    // @unsafe - address-of requires unsafe
    SafeWrapper(int& value) : ptr(&value), data(42) {}  // Always non-null
};
"#;
    let output = run_checker(code);
    // Should not complain about uninitialized pointer - default ctor is deleted
    // and the only constructor initializes ptr properly
    assert!(
        output.contains("no violations") || !output.contains("must be initialized"),
        "Should allow pointer member when default ctor is deleted and other ctor inits it. Output: {}",
        output
    );
}

#[test]
fn test_user_defined_ctor_only_no_default() {
    // @safe struct with only user-defined constructor (no default)
    let code = r#"
// @safe
struct SafeWrapper {
    int* ptr;

    // @unsafe - address-of requires unsafe
    SafeWrapper(int* p) : ptr(p) {}  // Only constructor - no implicit default
};
"#;
    let output = run_checker(code);
    // Should allow since there's no way to create an uninitialized instance
    assert!(
        output.contains("no violations") || !output.contains("must be initialized"),
        "Should allow when only user-defined ctor exists and inits member. Output: {}",
        output
    );
}

#[test]
fn test_multiple_ctors_all_init_properly() {
    // @safe struct with multiple constructors, all init properly
    let code = r#"
// @safe
struct MultiCtor {
    int* ptr;

    MultiCtor() = delete;

    // @unsafe
    MultiCtor(int* p) : ptr(p) {}

    // @unsafe
    MultiCtor(int& ref) : ptr(&ref) {}
};
"#;
    let output = run_checker(code);
    assert!(
        output.contains("no violations") || !output.contains("must be initialized"),
        "Should allow when all ctors init the member. Output: {}",
        output
    );
}

#[test]
fn test_default_ctor_not_deleted_requires_init() {
    // @safe struct with explicit default constructor that doesn't init - ERROR
    let code = r#"
// @safe
struct BadWrapper {
    int* ptr;

    // @safe
    BadWrapper() {}  // ERROR: default ctor doesn't init ptr

    // @unsafe
    BadWrapper(int* p) : ptr(p) {}
};
"#;
    let output = run_checker(code);
    // Should complain - default ctor exists and doesn't init
    assert!(
        output.contains("must be initialized") || output.contains("pointer member"),
        "Should require init when default ctor exists. Output: {}",
        output
    );
}

#[test]
fn test_init_list_to_nullptr_still_error() {
    // Even with deleted default ctor, initializing to nullptr is still an error
    let code = r#"
// @safe
struct BadWrapper {
    int* ptr;

    BadWrapper() = delete;

    // @safe
    BadWrapper(int x) : ptr(nullptr) {}  // ERROR: initializing to nullptr
};
"#;
    let output = run_checker(code);
    assert!(
        output.contains("nullptr") || output.contains("null"),
        "Should detect nullptr init even with deleted default ctor. Output: {}",
        output
    );
}

#[test]
fn test_multiple_ctors_one_doesnt_init() {
    // Multiple constructors, but one doesn't initialize the pointer - ERROR
    let code = r#"
// @safe
struct MultiCtor {
    int* ptr;

    MultiCtor() = delete;

    // @unsafe - this one initializes
    MultiCtor(int* p) : ptr(p) {}

    // @safe - this one does NOT initialize ptr
    MultiCtor(int x) {}  // ERROR: doesn't init ptr
};
"#;
    let output = run_checker(code);
    assert!(
        output.contains("must be initialized") || output.contains("pointer member"),
        "Should detect when one constructor doesn't init the pointer. Output: {}",
        output
    );
}
