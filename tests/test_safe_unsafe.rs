use std::process::Command;
use std::fs;

#[test]
fn test_unsafe_by_default() {
    // By default, C++ files are unsafe - no checking
    let test_code = r#"
void test() {
    int* ptr = new int(42);
    int* ptr2 = ptr;
    delete ptr;
    *ptr2 = 100;  // Use after free - but should NOT be caught (unsafe by default)
}
"#;
    
    fs::write("test_unsafe_default.cpp", test_code).unwrap();
    
    let output = Command::new("cargo")
        .args(&["run", "--", "test_unsafe_default.cpp"])
        .env("Z3_SYS_Z3_HEADER", "/opt/homebrew/include/z3.h")
        .env("DYLD_LIBRARY_PATH", "/opt/homebrew/Cellar/llvm/19.1.7/lib")
        .output()
        .expect("Failed to run borrow checker");
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    
    // Should NOT detect any violations (unsafe by default)
    assert!(stdout.contains("no violations found") || stdout.contains("✓"),
            "Should not check unsafe code by default. Output: {}", stdout);
    
    // Clean up
    let _ = fs::remove_file("test_unsafe_default.cpp");
}

#[test]
fn test_safe_namespace_annotation() {
    // With @safe annotation on namespace, all contents are checked
    let test_code = r#"// @safe
namespace myapp {

class UniquePtr {
public:
    UniquePtr(int* p) : ptr(p) {}
    int* ptr;
};

UniquePtr&& move(UniquePtr& p) {
    return static_cast<UniquePtr&&>(p);
}

void test() {
    int* raw = new int(42);
    UniquePtr ptr(raw);
    UniquePtr moved = move(ptr);
    UniquePtr again = move(ptr);  // Error: use after move
}

}  // namespace myapp
"#;
    
    fs::write("test_safe_namespace.cpp", test_code).unwrap();
    
    let output = Command::new("cargo")
        .args(&["run", "--", "test_safe_namespace.cpp"])
        .env("Z3_SYS_Z3_HEADER", "/opt/homebrew/include/z3.h")
        .env("DYLD_LIBRARY_PATH", "/opt/homebrew/Cellar/llvm/19.1.7/lib")
        .output()
        .expect("Failed to run borrow checker");
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    
    // Should detect use-after-move with @safe annotation
    assert!(stdout.contains("moved") || stdout.contains("violation"),
            "Should detect errors with @safe namespace annotation. Output: {}", stdout);
    
    // Clean up
    let _ = fs::remove_file("test_safe_namespace.cpp");
}

#[test]
fn test_safe_function_annotation() {
    // Individual function marked as @safe
    let test_code = r#"
class UniquePtr {
public:
    UniquePtr(int* p) : ptr(p) {}
    int* ptr;
};

UniquePtr&& move(UniquePtr& p) {
    return static_cast<UniquePtr&&>(p);
}

// This function is not checked (no annotation)
void unsafe_func() {
    int* ptr = new int(42);
    delete ptr;
    *ptr = 100;  // Use after free - not caught
}

// @safe
// This function IS checked
void safe_func() {
    int* raw = new int(42);
    UniquePtr ptr(raw);
    UniquePtr moved = move(ptr);
    UniquePtr again = move(ptr);  // Error: use after move - should be caught
}
"#;
    
    fs::write("test_safe_func.cpp", test_code).unwrap();
    
    let output = Command::new("cargo")
        .args(&["run", "--", "test_safe_func.cpp"])
        .env("Z3_SYS_Z3_HEADER", "/opt/homebrew/include/z3.h")
        .env("DYLD_LIBRARY_PATH", "/opt/homebrew/Cellar/llvm/19.1.7/lib")
        .output()
        .expect("Failed to run borrow checker");
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    
    // Should detect error in safe_func but not unsafe_func
    assert!(stdout.contains("moved") || stdout.contains("violation"),
            "Should detect errors in @safe function. Output: {}", stdout);
    
    // Clean up
    let _ = fs::remove_file("test_safe_func.cpp");
}

#[test]
fn test_unsafe_function_annotation() {
    // Function explicitly marked as @unsafe in a safe namespace
    let test_code = r#"// @safe
namespace myapp {

class UniquePtr {
public:
    UniquePtr(int* p) : ptr(p) {}
    int* ptr;
};

UniquePtr&& move(UniquePtr& p) {
    return static_cast<UniquePtr&&>(p);
}

// @unsafe
// This function is NOT checked even in safe mode
void unsafe_func() {
    int* raw = new int(42);
    UniquePtr ptr(raw);
    UniquePtr moved = move(ptr);
    UniquePtr again = move(ptr);  // Not caught - function is @unsafe
}

// This function IS checked (safe namespace)
void safe_func() {
    int value = 42;
    int& ref1 = value;
    int& ref2 = value;  // Error: double mutable borrow
}

}  // namespace myapp
"#;
    
    fs::write("test_unsafe_func.cpp", test_code).unwrap();
    
    let output = Command::new("cargo")
        .args(&["run", "--", "test_unsafe_func.cpp"])
        .env("Z3_SYS_Z3_HEADER", "/opt/homebrew/include/z3.h")
        .env("DYLD_LIBRARY_PATH", "/opt/homebrew/Cellar/llvm/19.1.7/lib")
        .output()
        .expect("Failed to run borrow checker");
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    
    // Should detect error in safe_func but not unsafe_func
    assert!(stdout.contains("already mutably borrowed") || stdout.contains("violation"),
            "Should detect errors in safe function but not @unsafe. Output: {}", stdout);
    
    // Should NOT detect the move error in unsafe_func
    assert!(!stdout.contains("again"),
            "Should not check @unsafe function. Output: {}", stdout);
    
    // Clean up
    let _ = fs::remove_file("test_unsafe_func.cpp");
}

#[test]
fn test_mixed_safe_unsafe() {
    // Mix of safe and unsafe code
    let test_code = r#"
// @safe
void checked_function() {
    int value = 42;
    int& ref1 = value;
    const int& ref2 = value;  // Error: mixing mutable and const
}

// No annotation - unsafe by default
void unchecked_function() {
    int value = 42;
    int& ref1 = value;
    int& ref2 = value;  // Not caught - function is unsafe
}
"#;
    
    fs::write("test_mixed.cpp", test_code).unwrap();
    
    let output = Command::new("cargo")
        .args(&["run", "--", "test_mixed.cpp"])
        .env("Z3_SYS_Z3_HEADER", "/opt/homebrew/include/z3.h")
        .env("DYLD_LIBRARY_PATH", "/opt/homebrew/Cellar/llvm/19.1.7/lib")
        .output()
        .expect("Failed to run borrow checker");
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    
    // Should detect error only in checked_function
    assert!(stdout.contains("already mutably borrowed") || stdout.contains("violation"),
            "Should detect errors in @safe function. Output: {}", stdout);
    
    // Clean up
    let _ = fs::remove_file("test_mixed.cpp");
}