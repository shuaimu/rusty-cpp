use std::path::Path;
use std::process::Command;
use tempfile::{NamedTempFile, TempDir};
use std::io::Write;
use std::fs;
use std::env;

fn run_analyzer(cpp_file: &Path) -> (bool, String) {
    // Set Z3 header path based on platform
    let z3_header = if cfg!(target_os = "macos") {
        "/opt/homebrew/include/z3.h"
    } else {
        "/usr/include/z3.h"
    };
    
    let mut cmd = Command::new("cargo");
    cmd.args(&["run", "--quiet", "--", cpp_file.to_str().unwrap()])
        .env("Z3_SYS_Z3_HEADER", z3_header);
    
    // Set library paths based on platform
    if cfg!(target_os = "macos") {
        cmd.env("DYLD_LIBRARY_PATH", "/opt/homebrew/Cellar/llvm/19.1.7/lib");
    } else {
        cmd.env("LD_LIBRARY_PATH", "/usr/lib/llvm-14/lib");
    }
    
    let output = cmd.output()
        .expect("Failed to execute analyzer");
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let full_output = format!("{}{}", stdout, stderr);
    
    (output.status.success(), full_output)
}

fn create_temp_cpp_file(content: &str) -> NamedTempFile {
    let mut file = NamedTempFile::with_suffix(".cpp").unwrap();
    file.write_all(content.as_bytes()).unwrap();
    file.flush().unwrap();
    file
}

#[test]
fn test_valid_cpp_code_passes() {
    let code = r#"
    void valid_function() {
        int x = 42;
        int y = x + 1;
    }
    
    int main() {
        valid_function();
        return 0;
    }
    "#;
    
    let temp_file = create_temp_cpp_file(code);
    let (success, output) = run_analyzer(temp_file.path());
    
    assert!(success, "Valid code should pass analysis. Output: {}", output);
    assert!(output.contains("no violations found"));
}

#[test]
fn test_simple_function_analysis() {
    let code = r#"
    int add(int a, int b) {
        return a + b;
    }
    "#;
    
    let temp_file = create_temp_cpp_file(code);
    let (success, output) = run_analyzer(temp_file.path());
    
    assert!(success, "Simple function should pass. Output: {}", output);
}

#[test]
fn test_multiple_functions() {
    let code = r#"
    void func1() {
        int x = 1;
    }
    
    void func2() {
        int y = 2;
    }
    
    void func3() {
        func1();
        func2();
    }
    "#;
    
    let temp_file = create_temp_cpp_file(code);
    let (success, output) = run_analyzer(temp_file.path());
    
    assert!(success, "Multiple functions should be handled. Output: {}", output);
}

// These tests would fail with proper implementation but pass with current skeleton
#[test]
#[ignore] // TODO: Requires dereference assignment tracking (*ptr = value)
fn test_use_after_move_with_unique_ptr() {
    let code = r#"
    #include <memory>

    // @safe
    void bad_function() {
        std::unique_ptr<int> ptr1(new int(42));
        std::unique_ptr<int> ptr2 = std::move(ptr1);
        *ptr1 = 10;  // Should be detected as use-after-move (requires dereference tracking)
    }
    "#;
    
    let temp_file = create_temp_cpp_file(code);
    let (success, output) = run_analyzer(temp_file.path());
    
    assert!(!success, "Use-after-move should fail analysis");
    assert!(output.contains("use") && output.contains("move"));
}

#[test]
fn test_multiple_mutable_borrows() {
    let code = r#"// @safe
    void bad_borrows() {
        int value = 42;
        int& ref1 = value;
        int& ref2 = value;  // Should be detected as multiple mutable borrows
        ref1 = 10;
        ref2 = 20;
    }
    "#;
    
    let temp_file = create_temp_cpp_file(code);
    let (success, output) = run_analyzer(temp_file.path());
    
    assert!(output.contains("violation"), "Should detect violations");
    assert!(output.contains("Cannot create mutable reference") && output.contains("already mutably borrowed"), 
            "Should detect multiple mutable borrows. Output: {}", output);
}

#[test]
fn test_dangling_reference_lifetime() {
    let code = r#"
    // @safe
    int& get_dangling() {
        int local = 42;
        return local;  // Should be detected as returning reference to local
    }
    "#;
    
    let temp_file = create_temp_cpp_file(code);
    let (success, output) = run_analyzer(temp_file.path());
    
    assert!(!success, "Dangling reference should fail");
    assert!(output.contains("lifetime") || output.contains("dangling"));
}

#[test]
fn test_const_references_allowed() {
    let code = r#"// @safe
    void test_function() {
        int value = 42;
        const int& ref1 = value;
        const int& ref2 = value;
        const int& ref3 = value;  // Multiple const refs should be OK
    }
    "#;
    
    let temp_file = create_temp_cpp_file(code);
    let (success, output) = run_analyzer(temp_file.path());
    
    assert!(success, "Multiple const references should be allowed. Output: {}", output);
    assert!(output.contains("no violations found"));
}

#[test]
fn test_mixed_const_and_mutable_refs() {
    let code = r#"// @safe
    void test_function() {
        int value = 42;
        const int& const_ref = value;
        int& mut_ref = value;  // Should fail - can't have mutable with const
    }
    "#;
    
    let temp_file = create_temp_cpp_file(code);
    let (_success, output) = run_analyzer(temp_file.path());
    
    assert!(output.contains("violation"), "Should detect mixed reference violation");
    assert!(output.contains("Cannot create mutable reference") && output.contains("already immutably borrowed"),
            "Should detect mixed borrows. Output: {}", output);
}

#[test]
fn test_mutable_then_const_refs() {
    let code = r#"// @safe
    void test_function() {
        int value = 42;
        int& mut_ref = value;
        const int& const_ref = value;  // Should fail - can't have const with mutable
    }
    "#;
    
    let temp_file = create_temp_cpp_file(code);
    let (_success, output) = run_analyzer(temp_file.path());
    
    assert!(output.contains("violation"), "Should detect mixed reference violation");
    assert!(output.contains("Cannot create immutable reference") && output.contains("already mutably borrowed"),
            "Should detect mixed borrows. Output: {}", output);
}

#[test]
fn test_reference_to_reference() {
    let code = r#"
    void test_function() {
        int value = 42;
        int& ref1 = value;
        int& ref2 = ref1;  // Should work with our current implementation
    }
    "#;
    
    let temp_file = create_temp_cpp_file(code);
    let (_success, output) = run_analyzer(temp_file.path());
    
    // This will likely show violations with current implementation
    // but documents expected behavior
    println!("Reference to reference output: {}", output);
}

#[test]
fn test_const_ref_assignment_detection() {
    let code = r#"
    void test_function() {
        int value = 42;
        const int& const_ref = value;
        // We can't detect this at parse time, but documenting expected behavior
        // const_ref = 100;  // Would be a compile error in C++
    }
    "#;
    
    let temp_file = create_temp_cpp_file(code);
    let (success, output) = run_analyzer(temp_file.path());
    
    assert!(success, "Valid const reference should pass. Output: {}", output);
}

#[test]
fn test_simulated_move_semantics() {
    // Test move semantics without std::unique_ptr
    let code = r#"
    struct Resource {
        int* data;
    };
    
    void test_move() {
        Resource r1;
        r1.data = new int(42);
        
        // Simulate move - in real implementation we'd track this
        Resource r2 = r1;  // This is a copy in C++, but we could track as move
        
        // Use after "move" - would need special annotations
        *r1.data = 10;  // Currently won't be detected without annotations
    }
    "#;
    
    let temp_file = create_temp_cpp_file(code);
    let (_success, output) = run_analyzer(temp_file.path());
    
    // Document current behavior - this passes because we don't track raw pointers yet
    println!("Move semantics test output: {}", output);
}

#[test] 
fn test_reference_invalidation() {
    // Test that we detect when references become invalid
    let code = r#"// @safe
    void test_invalidation() {
        int x = 10;
        int& ref1 = x;
        int& ref2 = x;  // Should fail - multiple mutable refs
        
        ref1 = 20;
        ref2 = 30;
    }
    "#;
    
    let temp_file = create_temp_cpp_file(code);
    let (_success, output) = run_analyzer(temp_file.path());
    
    assert!(output.contains("violation"), "Should detect reference violations");
    assert!(output.contains("already mutably borrowed"), "Should detect multiple mutable refs");
}

#[test]
fn test_const_after_mutable() {
    // Test Rust's rule: can't have const ref while mutable ref exists
    let code = r#"// @safe
    void test() {
        int value = 100;
        int& mut_ref = value;      // Mutable borrow
        const int& const_ref = value;  // Should fail
        
        mut_ref = 200;
        int x = const_ref;  // Would read stale value
    }
    "#;
    
    let temp_file = create_temp_cpp_file(code);
    let (_success, output) = run_analyzer(temp_file.path());
    
    assert!(output.contains("violation"), "Should detect violation");
    assert!(output.contains("Cannot create immutable reference") && 
            output.contains("already mutably borrowed"),
            "Should enforce Rust's borrowing rules. Output: {}", output);
}

#[test]
fn test_complex_reference_pattern() {
    // Test a more complex borrowing pattern
    let code = r#"// @safe
    void complex_test() {
        int a = 1;
        int b = 2;
        
        const int& ref_a1 = a;  // OK - immutable borrow of a
        const int& ref_a2 = a;  // OK - multiple immutable borrows
        
        int& mut_b = b;         // OK - mutable borrow of b
        const int& ref_b = b;   // ERROR - can't have both
    }
    "#;
    
    let temp_file = create_temp_cpp_file(code);
    let (_success, output) = run_analyzer(temp_file.path());
    
    // Should only error on 'b', not 'a'
    assert!(output.contains("violation"), "Should detect violation");
    assert!(output.contains("'b'") && output.contains("already mutably borrowed"),
            "Should only error on variable b. Output: {}", output);
}

#[test]
fn test_cross_file_header_parsing() {
    // Create a temporary directory for our test files
    let temp_dir = TempDir::new().unwrap();
    
    // Create a header file with lifetime annotations
    let header_content = r#"
#ifndef TEST_H
#define TEST_H

// @lifetime: &'a
const int& getRef();

// @lifetime: (&'a) -> &'a
const int& identity(const int& x);

// @lifetime: owned
int getValue();

#endif
"#;
    
    let header_path = temp_dir.path().join("test.h");
    fs::write(&header_path, header_content).unwrap();
    
    // Create a C++ file that includes the header
    let cpp_content = r#"
#include "test.h"

void test_function() {
    const int& ref = getRef();
    int value = getValue();
    const int& ref2 = identity(ref);
}
"#;
    
    let cpp_path = temp_dir.path().join("test.cpp");
    fs::write(&cpp_path, cpp_content).unwrap();
    
    // Run the analyzer
    let (success, output) = run_analyzer(&cpp_path);
    
    // Should succeed - no borrow violations
    assert!(success, "Should successfully analyze file with header. Output: {}", output);
    assert!(output.contains("no violations found"), 
            "Should find no violations. Output: {}", output);
}

#[test]
fn test_lifetime_annotation_parsing() {
    // Create a temporary directory
    let temp_dir = TempDir::new().unwrap();
    
    // Create header with various lifetime annotations
    let header_content = r#"
#ifndef LIFETIME_H
#define LIFETIME_H

// @lifetime: &'a
const char* getString();

// @lifetime: (&'a, &'b) -> &'a where 'a: 'b
const char* selectFirst(const char* a, const char* b);

// @lifetime: &'a mut
char* getMutableString();

// @lifetime: owned
char* copyString(const char* src);

#endif
"#;
    
    let header_path = temp_dir.path().join("lifetime.h");
    fs::write(&header_path, header_content).unwrap();
    
    // Create C++ file using the header
    let cpp_content = r#"
#include "lifetime.h"

void test_lifetimes() {
    const char* s1 = getString();
    const char* s2 = getString();
    
    const char* selected = selectFirst(s1, s2);
    
    char* owned = copyString(s1);
    
    char* mut_str = getMutableString();
    *mut_str = 'x';
}
"#;
    
    let cpp_path = temp_dir.path().join("test_lifetime.cpp");
    fs::write(&cpp_path, cpp_content).unwrap();
    
    let (success, output) = run_analyzer(&cpp_path);
    
    // This should pass as there are no borrow violations
    assert!(success, "Should handle lifetime annotations. Output: {}", output);
}

#[test]
fn test_env_include_paths() {
    // Create a temporary directory structure
    let temp_dir = TempDir::new().unwrap();
    let include_dir = temp_dir.path().join("include");
    fs::create_dir(&include_dir).unwrap();
    
    // Create header with annotations
    let header_content = r#"
#ifndef TEST_ENV_H
#define TEST_ENV_H

// @lifetime: &'a
const int& getEnvRef();

// @lifetime: owned
int getEnvValue();

#endif
"#;
    
    let header_path = include_dir.join("test_env.h");
    fs::write(&header_path, header_content).unwrap();
    
    // Create C++ file that uses the header
    let cpp_content = r#"
#include <test_env.h>

void test_env() {
    const int& ref = getEnvRef();
    int value = getEnvValue();
}
"#;
    
    let cpp_path = temp_dir.path().join("test_env.cpp");
    fs::write(&cpp_path, cpp_content).unwrap();
    
    // Run analyzer with environment variable set
    // Set Z3 header path based on platform
    let z3_header = if cfg!(target_os = "macos") {
        "/opt/homebrew/include/z3.h"
    } else {
        "/usr/include/z3.h"
    };
    
    let mut cmd = Command::new("cargo");
    cmd.args(&["run", "--quiet", "--", cpp_path.to_str().unwrap()])
        .env("CPLUS_INCLUDE_PATH", include_dir.to_str().unwrap())
        .env("Z3_SYS_Z3_HEADER", z3_header);
    
    // Set library paths based on platform
    if cfg!(target_os = "macos") {
        cmd.env("DYLD_LIBRARY_PATH", "/opt/homebrew/Cellar/llvm/19.1.7/lib");
    } else {
        cmd.env("LD_LIBRARY_PATH", "/usr/lib/llvm-14/lib");
    }
    
    let output = cmd.output()
        .expect("Failed to execute analyzer");
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let full_output = format!("{}{}", stdout, stderr);
    
    assert!(output.status.success(), 
            "Should successfully use environment include paths. Output: {}", full_output);
    assert!(full_output.contains("Found 1 include path(s) from environment"),
            "Should report finding environment paths. Output: {}", full_output);
}