use std::process::Command;
use std::fs;

#[test]
fn test_scope_tracking_eliminates_false_positives() {
    // This code SHOULD NOT have any errors with scope tracking
    let test_code = r#"
void test() {
    int value = 42;
    
    // First scope
    {
        int& ref1 = value;
        ref1 = 100;
    }  // ref1 out of scope
    
    // Second scope - should be OK
    {
        int& ref2 = value;
        ref2 = 200;
    }  // ref2 out of scope
}
"#;
    
    fs::write("test_scopes.cpp", test_code).unwrap();
    
    let output = Command::new("cargo")
        .args(&["run", "--", "test_scopes.cpp"])
        .env("Z3_SYS_Z3_HEADER", "/opt/homebrew/include/z3.h")
        .output()
        .expect("Failed to run borrow checker");
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    
    // Should find NO violations
    assert!(stdout.contains("no violations found") || 
            stdout.contains("✓"),
            "Should not report false positives for scoped references. Output: {}", stdout);
    
    // Clean up
    let _ = fs::remove_file("test_scopes.cpp");
}

#[test]
fn test_scope_tracking_still_catches_real_errors() {
    // This code SHOULD have an error
    let test_code = r#"
void test() {
    int value = 42;
    int& ref1 = value;
    int& ref2 = value;  // ERROR: double mutable borrow
}
"#;
    
    fs::write("test_double_borrow.cpp", test_code).unwrap();
    
    let output = Command::new("cargo")
        .args(&["run", "--", "test_double_borrow.cpp"])
        .env("Z3_SYS_Z3_HEADER", "/opt/homebrew/include/z3.h")
        .output()
        .expect("Failed to run borrow checker");
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    
    // Should find the violation
    assert!(stdout.contains("violation") || stdout.contains("already mutably borrowed"),
            "Should still catch real double borrows. Output: {}", stdout);
    
    // Clean up
    let _ = fs::remove_file("test_double_borrow.cpp");
}

#[test]
fn test_nested_scope_tracking() {
    let test_code = r#"
void test() {
    int value = 42;
    
    {
        const int& cref1 = value;
        {
            const int& cref2 = value;  // Nested const refs - OK
            int sum = cref1 + cref2;
        }
    }
    
    // All const refs gone
    int& mref = value;  // Should be OK
    mref = 100;
}
"#;
    
    fs::write("test_nested.cpp", test_code).unwrap();
    
    let output = Command::new("cargo")
        .args(&["run", "--", "test_nested.cpp"])
        .env("Z3_SYS_Z3_HEADER", "/opt/homebrew/include/z3.h")
        .output()
        .expect("Failed to run borrow checker");
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    
    // Should find NO violations
    assert!(stdout.contains("no violations found") || 
            stdout.contains("✓") ||
            !stdout.contains("violation"),
            "Nested scopes should work correctly. Output: {}", stdout);
    
    // Clean up
    let _ = fs::remove_file("test_nested.cpp");
}