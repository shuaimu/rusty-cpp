use std::io::Write;
use std::process::Command;
use tempfile::NamedTempFile;

fn run_analyzer(cpp_file: &std::path::Path) -> (bool, String) {
    let mut cmd = Command::new("cargo");
    cmd.args(&["run", "--quiet", "--", cpp_file.to_str().unwrap()]);

    if cfg!(target_os = "macos") {
        cmd.env("Z3_SYS_Z3_HEADER", "/opt/homebrew/include/z3.h");
        cmd.env("DYLD_LIBRARY_PATH", "/opt/homebrew/Cellar/llvm/19.1.7/lib");
    } else {
        cmd.env("Z3_SYS_Z3_HEADER", "/usr/include/z3.h");
        cmd.env("LD_LIBRARY_PATH", "/usr/lib/llvm-14/lib");
    }

    let output = cmd.output().expect("Failed to execute analyzer");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let full_output = format!("{}{}", stdout, stderr);

    (output.status.success(), full_output)
}

// ============================================================================
// SAFE NAMESPACE TESTS
// ============================================================================

#[test]
fn test_unsafe_class_in_safe_namespace() {
    // Class @unsafe should override namespace @safe
    let code = r#"
// @safe
namespace myapp {

// @unsafe
class UnsafeClass {
public:
    void use_raw_pointers() {
        int* ptr = nullptr;
        *ptr = 42;  // OK in unsafe class, even though namespace is safe
    }
};

} // namespace myapp
"#;

    let mut temp_file = NamedTempFile::new().unwrap();
    write!(temp_file, "{}", code).unwrap();

    let (success, output) = run_analyzer(temp_file.path());

    assert!(
        success,
        "Class @unsafe should override namespace @safe. Output: {}",
        output
    );
}

#[test]
fn test_safe_class_in_safe_namespace() {
    // Both safe - should enforce safety rules
    let code = r#"
// @safe
namespace myapp {

// @safe (redundant but explicit)
class SafeClass {
public:
    void safe_method() {
        int x = 42;
    }
};

} // namespace myapp
"#;

    let mut temp_file = NamedTempFile::new().unwrap();
    write!(temp_file, "{}", code).unwrap();

    let (success, _output) = run_analyzer(temp_file.path());

    assert!(success, "Safe class in safe namespace should work");
}

#[test]
fn test_unsafe_function_in_safe_class_in_safe_namespace() {
    // Function @unsafe should override both class @safe and namespace @safe
    let code = r#"
// @safe
namespace myapp {

// @safe
class SafeClass {
public:
    // @unsafe
    void unsafe_method() {
        int* ptr = nullptr;
        *ptr = 42;  // OK in unsafe function
    }

    // @safe (inherits from class)
    void safe_method() {
        int x = 42;
    }
};

} // namespace myapp
"#;

    let mut temp_file = NamedTempFile::new().unwrap();
    write!(temp_file, "{}", code).unwrap();

    let (success, output) = run_analyzer(temp_file.path());

    assert!(
        success,
        "Function @unsafe should override class @safe and namespace @safe. Output: {}",
        output
    );
}

#[test]
fn test_safe_function_in_unsafe_class_in_safe_namespace() {
    // Function @safe should override class @unsafe (namespace @safe is overridden by class)
    let code = r#"
// @safe
namespace myapp {

// @unsafe
class UnsafeClass {
public:
    // @safe
    void safe_method() {
        int x = 42;
        // Cannot use raw pointers here - this is a safe function
    }

    // @unsafe (inherits from class)
    void unsafe_method() {
        int* ptr = nullptr;
        *ptr = 42;
    }
};

} // namespace myapp
"#;

    let mut temp_file = NamedTempFile::new().unwrap();
    write!(temp_file, "{}", code).unwrap();

    let (success, output) = run_analyzer(temp_file.path());

    assert!(
        success,
        "Function @safe should override class @unsafe. Output: {}",
        output
    );
}

// ============================================================================
// UNSAFE NAMESPACE TESTS
// ============================================================================

#[test]
fn test_safe_class_in_unsafe_namespace() {
    // Class @safe should override namespace @unsafe
    let code = r#"
// @unsafe
namespace myapp {

// @safe
class SafeClass {
public:
    void safe_method() {
        int x = 42;
        // Cannot use raw pointers here
    }
};

// Undeclared class - inherits @unsafe from namespace
class UnsafeClass {
public:
    void unsafe_method() {
        int* ptr = nullptr;
        *ptr = 42;  // OK - inherited unsafe
    }
};

} // namespace myapp
"#;

    let mut temp_file = NamedTempFile::new().unwrap();
    write!(temp_file, "{}", code).unwrap();

    let (success, output) = run_analyzer(temp_file.path());

    assert!(
        success,
        "Class @safe should override namespace @unsafe. Output: {}",
        output
    );
}

#[test]
fn test_safe_function_in_safe_class_in_unsafe_namespace() {
    // Function @safe inherits from class @safe, which overrides namespace @unsafe
    let code = r#"
// @unsafe
namespace myapp {

// @safe
class SafeClass {
public:
    // @safe (redundant, inherits from class)
    void safe_method() {
        int x = 42;
    }
};

} // namespace myapp
"#;

    let mut temp_file = NamedTempFile::new().unwrap();
    write!(temp_file, "{}", code).unwrap();

    let (success, _output) = run_analyzer(temp_file.path());

    assert!(success, "Safe function in safe class should work");
}

#[test]
fn test_unsafe_function_in_safe_class_in_unsafe_namespace() {
    // Function @unsafe should override class @safe
    let code = r#"
// @unsafe
namespace myapp {

// @safe
class SafeClass {
public:
    // @unsafe
    void unsafe_method() {
        int* ptr = nullptr;
        *ptr = 42;  // OK - function is unsafe
    }

    void safe_method() {
        int x = 42;  // OK - inherits safe from class
    }
};

} // namespace myapp
"#;

    let mut temp_file = NamedTempFile::new().unwrap();
    write!(temp_file, "{}", code).unwrap();

    let (success, output) = run_analyzer(temp_file.path());

    assert!(
        success,
        "Function @unsafe should override class @safe. Output: {}",
        output
    );
}

#[test]
fn test_safe_function_in_unsafe_class_in_unsafe_namespace() {
    // Function @safe should override both class @unsafe and namespace @unsafe
    let code = r#"
// @unsafe
namespace myapp {

// @unsafe (redundant, inherits from namespace)
class UnsafeClass {
public:
    // @safe
    void safe_method() {
        int x = 42;
        // Cannot use raw pointers
    }

    void unsafe_method() {
        int* ptr = nullptr;
        *ptr = 42;  // OK - inherits unsafe
    }
};

} // namespace myapp
"#;

    let mut temp_file = NamedTempFile::new().unwrap();
    write!(temp_file, "{}", code).unwrap();

    let (success, output) = run_analyzer(temp_file.path());

    assert!(
        success,
        "Function @safe should override class @unsafe and namespace @unsafe. Output: {}",
        output
    );
}

// ============================================================================
// UNDECLARED NAMESPACE TESTS
// ============================================================================

#[test]
fn test_safe_class_in_undeclared_namespace() {
    // Class @safe should work in undeclared namespace
    let code = r#"
namespace myapp {

// @safe
class SafeClass {
public:
    void safe_method() {
        int x = 42;
    }
};

} // namespace myapp
"#;

    let mut temp_file = NamedTempFile::new().unwrap();
    write!(temp_file, "{}", code).unwrap();

    let (success, _output) = run_analyzer(temp_file.path());

    assert!(success, "Safe class in undeclared namespace should work");
}

#[test]
fn test_unsafe_class_in_undeclared_namespace() {
    // Class @unsafe should work in undeclared namespace
    let code = r#"
namespace myapp {

// @unsafe
class UnsafeClass {
public:
    void unsafe_method() {
        int* ptr = nullptr;
        *ptr = 42;
    }
};

} // namespace myapp
"#;

    let mut temp_file = NamedTempFile::new().unwrap();
    write!(temp_file, "{}", code).unwrap();

    let (success, _output) = run_analyzer(temp_file.path());

    assert!(success, "Unsafe class in undeclared namespace should work");
}

#[test]
fn test_mixed_functions_in_safe_class_in_undeclared_namespace() {
    // Mix of safe and unsafe functions in safe class
    let code = r#"
namespace myapp {

// @safe
class MixedClass {
public:
    // @safe (inherits from class)
    void safe_method() {
        int x = 42;
    }

    // @unsafe (overrides class)
    void unsafe_method() {
        int* ptr = nullptr;
        *ptr = 42;
    }
};

} // namespace myapp
"#;

    let mut temp_file = NamedTempFile::new().unwrap();
    write!(temp_file, "{}", code).unwrap();

    let (success, output) = run_analyzer(temp_file.path());

    assert!(
        success,
        "Mixed safe/unsafe functions should work with proper annotations. Output: {}",
        output
    );
}

// ============================================================================
// COMPLEX HIERARCHY TESTS
// ============================================================================

#[test]
fn test_three_level_override_safe_unsafe_safe() {
    // Safe namespace -> Unsafe class -> Safe function
    let code = r#"
// @safe
namespace myapp {

// @unsafe
class UnsafeClass {
public:
    // @safe
    void safe_method() {
        int x = 42;
        // This is safe despite being in unsafe class in safe namespace
    }
};

} // namespace myapp
"#;

    let mut temp_file = NamedTempFile::new().unwrap();
    write!(temp_file, "{}", code).unwrap();

    let (success, output) = run_analyzer(temp_file.path());

    assert!(
        success,
        "Three-level override (safe->unsafe->safe) should work. Output: {}",
        output
    );
}

#[test]
fn test_three_level_override_unsafe_safe_unsafe() {
    // Unsafe namespace -> Safe class -> Unsafe function
    let code = r#"
// @unsafe
namespace myapp {

// @safe
class SafeClass {
public:
    // @unsafe
    void unsafe_method() {
        int* ptr = nullptr;
        *ptr = 42;
        // This is unsafe despite being in safe class in unsafe namespace
    }
};

} // namespace myapp
"#;

    let mut temp_file = NamedTempFile::new().unwrap();
    write!(temp_file, "{}", code).unwrap();

    let (success, output) = run_analyzer(temp_file.path());

    assert!(
        success,
        "Three-level override (unsafe->safe->unsafe) should work. Output: {}",
        output
    );
}

#[test]
fn test_multiple_classes_with_different_annotations() {
    // Multiple classes with different annotations in same namespace
    let code = r#"
// @safe
namespace myapp {

// @safe
class SafeClass {
public:
    void method1() { int x = 42; }
};

// @unsafe
class UnsafeClass {
public:
    void method2() {
        int* ptr = nullptr;
        *ptr = 42;
    }
};

// No annotation - undeclared
class UndeclaredClass {
public:
    void method3() { int y = 10; }
};

} // namespace myapp
"#;

    let mut temp_file = NamedTempFile::new().unwrap();
    write!(temp_file, "{}", code).unwrap();

    let (success, output) = run_analyzer(temp_file.path());

    assert!(
        success,
        "Multiple classes with different annotations should coexist. Output: {}",
        output
    );
}

#[test]
fn test_safe_cannot_call_unsafe_without_block() {
    // Test that safe functions CANNOT call unsafe functions directly - must use @unsafe block
    let code = r#"
// @safe
namespace myapp {

// @unsafe
class UnsafeHelper {
public:
    void do_unsafe_work() {
        int* ptr = nullptr;
    }
};

// @safe
class SafeClass {
public:
    // @safe
    void safe_method() {
        UnsafeHelper helper;
        helper.do_unsafe_work();  // ERROR: safe cannot call unsafe directly
    }
};

} // namespace myapp
"#;

    let mut temp_file = NamedTempFile::new().unwrap();
    write!(temp_file, "{}", code).unwrap();

    let (success, output) = run_analyzer(temp_file.path());

    // With the new two-state model, safe functions cannot call unsafe functions directly
    assert!(
        !success,
        "Safe functions should NOT be able to call unsafe functions directly. Output: {}",
        output
    );
    assert!(
        output.contains("@unsafe"),
        "Error should mention @unsafe block requirement. Output: {}",
        output
    );
}

#[test]
fn test_safe_can_call_unsafe_with_block() {
    // Test that safe functions CAN call unsafe functions inside @unsafe block
    let code = r#"
// @safe
namespace myapp {

// @unsafe
class UnsafeHelper {
public:
    void do_unsafe_work() {
        int* ptr = nullptr;
    }
};

// @safe
class SafeClass {
public:
    // @safe
    void safe_method() {
        // @unsafe
        {
            UnsafeHelper helper;
            helper.do_unsafe_work();  // OK: inside @unsafe block
        }
    }
};

} // namespace myapp
"#;

    let mut temp_file = NamedTempFile::new().unwrap();
    write!(temp_file, "{}", code).unwrap();

    let (success, output) = run_analyzer(temp_file.path());

    assert!(
        success,
        "Safe functions should be able to call unsafe functions inside @unsafe block. Output: {}",
        output
    );
}

#[test]
fn test_mutable_field_respects_class_annotation_not_namespace() {
    // Mutable fields should be checked based on class annotation, not namespace
    let code = r#"
// @safe
namespace myapp {

// @unsafe - mutable should be allowed
class UnsafeClass {
    mutable int count;
public:
    void increment() const {
        count++;  // OK - class is unsafe
    }
};

// @safe - mutable should be error
class SafeClass {
    mutable int count;  // ERROR - class is safe
public:
    void increment() const {
        count++;
    }
};

} // namespace myapp
"#;

    let mut temp_file = NamedTempFile::new().unwrap();
    write!(temp_file, "{}", code).unwrap();

    let (success, output) = run_analyzer(temp_file.path());

    // Should fail because SafeClass has mutable field
    assert!(
        !success,
        "Mutable field in safe class should be error. Output: {}",
        output
    );
    assert!(
        output.contains("Mutable field"),
        "Should report mutable field error. Output: {}",
        output
    );
}
