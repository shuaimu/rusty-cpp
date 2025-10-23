use std::fs;
use std::process::Command;
use tempfile::TempDir;

fn compile_and_check(source: &str) -> Result<Vec<String>, String> {
    let dir = TempDir::new().unwrap();
    let file_path = dir.path().join("test.cpp");
    fs::write(&file_path, source).unwrap();
    
    let output = Command::new("cargo")
        .args(&["run", "--", file_path.to_str().unwrap()])
        
        
        .output()
        .map_err(|e| format!("Failed to run checker: {}", e))?;
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    
    // Extract violations from output
    let mut violations = Vec::new();
    for line in stdout.lines().chain(stderr.lines()) {
        if line.contains("unsafe") || line.contains("violation") {
            violations.push(line.to_string());
        }
    }
    
    Ok(violations)
}

#[test]
fn test_unsafe_function_call_in_safe_context() {
    let source = r#"
// @safe
namespace safe_namespace {
    void unmarked_function() {
        // This function has no safety annotation
    }
    
    // @safe
    void safe_function() {
        unmarked_function(); // Should be an error
    }
}
"#;
    
    let violations = compile_and_check(source).unwrap();
    assert!(violations.iter().any(|v| v.contains("unmarked_function")));
    assert!(violations.iter().any(|v| v.contains("unsafe")));
}

#[test]
fn test_explicitly_unsafe_function_call() {
    let source = r#"
// @safe
namespace safe_namespace {
    // @unsafe
    void unsafe_operation() {
        // Explicitly unsafe
    }
    
    // @safe
    void safe_function() {
        unsafe_operation(); // Should be an error
    }
}
"#;
    
    let violations = compile_and_check(source).unwrap();
    assert!(violations.iter().any(|v| v.contains("unsafe_operation")));
}

#[test]
fn test_safe_calling_safe_is_allowed() {
    let source = r#"
// @safe
namespace safe_namespace {
    // @safe
    void helper() {
        // Safe helper function
    }
    
    // @safe
    void caller() {
        helper(); // Should be allowed
    }
}
"#;
    
    let violations = compile_and_check(source).unwrap();
    // Should not have any unsafe propagation errors for this case
    assert!(!violations.iter().any(|v| v.contains("helper") && v.contains("unsafe")));
}

#[test]
fn test_unsafe_function_can_call_anything() {
    let source = r#"
namespace default_namespace {
    void unmarked_function() {
        // No annotation
    }
    
    // @unsafe
    void unsafe_caller() {
        unmarked_function(); // Should be allowed in unsafe context
    }
}
"#;
    
    let violations = compile_and_check(source).unwrap();
    // Should not have errors for unsafe functions calling other functions
    assert!(!violations.iter().any(|v| v.contains("unmarked_function") && v.contains("requires unsafe")));
}

#[test]
fn test_nested_unsafe_calls() {
    let source = r#"
// @safe
namespace safe_namespace {
    void level3() {
        // Unmarked function
    }
    
    void level2() {
        level3(); // Unmarked calling unmarked
    }
    
    // @safe
    void level1() {
        level2(); // Safe calling unmarked - should error
    }
}
"#;
    
    let violations = compile_and_check(source).unwrap();
    assert!(violations.iter().any(|v| v.contains("level2")));
}

#[test]
fn test_standard_library_functions() {
    let source = r#"
extern "C" int printf(const char*, ...);

// @safe
namespace safe_namespace {
    // @safe
    void test_function() {
        printf("Hello\n"); // printf is whitelisted as safe
        int x = 10;
        int y = x; // move should be safe
    }
}
"#;
    
    let violations = compile_and_check(source).unwrap();
    // printf and move should not trigger unsafe propagation errors
    assert!(!violations.iter().any(|v| v.contains("printf") && v.contains("unsafe")));
}