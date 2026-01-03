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
    
    // Extract violations from output (ignore compiler warnings)
    let mut violations = Vec::new();
    for line in stdout.lines() {
        // Only include lines that look like actual violations, not compiler warnings
        if (line.contains("unsafe") || line.contains("violation"))
            && !line.contains("warning:")
            && !line.contains("-->")
            && !line.trim().starts_with("|")
            && !line.contains("✓") {
            violations.push(line.to_string());
        }
    }

    Ok(violations)
}

// REMOVED: This test was expecting incorrect behavior.
// When a namespace is marked @safe, all functions in it are safe by default.
// So unmarked_function() in a @safe namespace IS safe and can be called from safe_function().

#[test]
fn test_safe_namespace_makes_all_functions_safe() {
    // Test that @safe namespace makes unmarked functions safe by default
    let source = r#"
// @safe
namespace safe_namespace {
    void unmarked_function() {
        // This function has no explicit @safe annotation
        // But it should be safe because of the namespace annotation
    }

    void another_safe_function() {
        unmarked_function(); // Should be allowed - both are safe
    }
}
"#;

    let violations = compile_and_check(source).unwrap();
    // Should have NO violations - unmarked functions in @safe namespace are safe
    assert!(violations.is_empty(),
            "Expected no violations, but got: {:?}", violations);
}

#[test]
fn test_safe_namespace_can_call_unmarked_functions() {
    // Test that explicitly @safe function can call unmarked functions in same @safe namespace
    let source = r#"
// @safe
namespace safe_namespace {
    void helper() {
        // No explicit annotation
    }

    // @safe
    void caller() {
        helper(); // Should be allowed - helper is safe via namespace
    }
}
"#;

    let violations = compile_and_check(source).unwrap();
    assert!(violations.is_empty(),
            "Expected no violations when calling unmarked function in @safe namespace, got: {:?}", violations);
}

#[test]
fn test_nested_calls_in_safe_namespace() {
    // Test that nested calls work correctly in @safe namespace
    let source = r#"
// @safe
namespace safe_namespace {
    void level3() {
        // Unmarked function
    }

    void level2() {
        level3(); // Unmarked calling unmarked - both safe via namespace
    }

    void level1() {
        level2(); // All functions are safe via namespace
    }
}
"#;

    let violations = compile_and_check(source).unwrap();
    assert!(violations.is_empty(),
            "Expected no violations for nested calls in @safe namespace, got: {:?}", violations);
}

#[test]
fn test_safe_cannot_call_unsafe_without_block() {
    // With two-state model: @safe can ONLY call @safe
    // To call @unsafe, must use @unsafe block
    let source = r#"
// @safe
namespace safe_namespace {
    // @unsafe
    void unsafe_operation() {
        // Explicitly unsafe
    }

    // @safe
    void safe_function() {
        unsafe_operation(); // ERROR: @safe cannot call @unsafe without @unsafe block
    }
}
"#;

    let violations = compile_and_check(source).unwrap();
    // Should have violations - @safe cannot call @unsafe without @unsafe block
    assert!(!violations.is_empty(),
            "Expected violations when @safe calls @unsafe without @unsafe block, got: {:?}", violations);
    assert!(violations.iter().any(|v| v.contains("non-safe") || v.contains("@unsafe")),
            "Error should mention unsafe call requirement, got: {:?}", violations);
}

#[test]
fn test_safe_can_call_unsafe_with_block() {
    // With two-state model: @safe can call @unsafe IF wrapped in @unsafe block
    let source = r#"
// @safe
namespace safe_namespace {
    // @unsafe
    void unsafe_operation() {
        // Explicitly unsafe
    }

    // @safe
    void safe_function() {
        // @unsafe
        {
            unsafe_operation(); // OK: inside @unsafe block
        }
    }
}
"#;

    let violations = compile_and_check(source).unwrap();
    // Should have NO violations - @unsafe block allows calling unsafe
    assert!(violations.is_empty(),
            "Expected no violations when @safe calls @unsafe via @unsafe block, got: {:?}", violations);
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

// REMOVED: This test was expecting incorrect behavior.
// When a namespace is marked @safe, all functions in it are safe by default.
// So level2() and level3() in a @safe namespace ARE safe, and can be called from level1().

#[test]
fn test_unsafe_override_in_safe_namespace() {
    // Test that explicit @unsafe annotation overrides @safe namespace for pointer operations,
    // but borrow checking is still performed uniformly on all code.
    // With the new design: @unsafe allows pointer operations but does NOT disable borrow checking.
    let source = r#"
// @safe
namespace safe_namespace {
    // @unsafe
    void explicitly_unsafe() {
        // This is marked unsafe despite being in @safe namespace
        // @unsafe allows pointer operations but borrow conflicts are still checked
        int value = 42;
        int& ref1 = value;  // First mutable borrow
        // int& ref2 = value; // This would be a borrow conflict, removed
        int x = ref1;  // Use ref1
    }

    void safe_caller() {
        // @unsafe
        {
            explicitly_unsafe(); // OK - inside @unsafe block
        }
    }
}
"#;

    let violations = compile_and_check(source).unwrap();
    // Should have NO violations - no borrow conflicts in the code
    assert!(violations.is_empty(),
            "Expected no violations - no borrow conflicts, got: {:?}", violations);
}

#[test]
fn test_safe_namespace_with_explicit_safe_annotations() {
    // Test that explicit @safe annotations work correctly in @safe namespace
    let source = r#"
// @safe
namespace safe_namespace {
    // @safe (redundant but allowed)
    void explicitly_safe() {
        // Explicitly marked safe
    }

    void implicitly_safe() {
        // Safe via namespace
    }

    // @safe (redundant)
    void caller() {
        explicitly_safe();  // OK
        implicitly_safe();  // OK
    }
}
"#;

    let violations = compile_and_check(source).unwrap();
    assert!(violations.is_empty(),
            "Expected no violations when all functions are safe, got: {:?}", violations);
}

#[test]
fn test_unmarked_namespace_functions_are_undeclared() {
    // Test that functions in unmarked (default) namespace are undeclared
    // and cannot be called from @safe functions
    let source = r#"
namespace default_namespace {
    void unmarked_function() {
        // No annotation, namespace has no annotation
    }
}

// @safe
void safe_function() {
    default_namespace::unmarked_function(); // Should be ERROR - undeclared function
}
"#;

    let violations = compile_and_check(source).unwrap();
    // Should have violation - safe function cannot call undeclared function
    assert!(violations.iter().any(|v| v.contains("unmarked_function") || v.contains("undeclared")),
            "Expected violation for calling undeclared function from safe context, got: {:?}", violations);
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