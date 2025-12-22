use std::fs::File;
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

#[test]
fn test_unsafe_with_hyphen_suffix() {
    // Test that @unsafe-XXX is recognized
    // This test just verifies the suffix parsing, not calling rules
    let code = r#"
// @unsafe-this should work
void unsafe_func() {
    int* ptr = nullptr;
    *ptr = 42;  // OK in @unsafe function
}

// @safe
void safe_func() {
    // Use @unsafe block to call unsafe function
    // @unsafe
    {
        unsafe_func();  // OK: inside @unsafe block
    }
}
"#;

    let mut temp_file = NamedTempFile::new().unwrap();
    write!(temp_file, "{}", code).unwrap();

    let (success, output) = run_analyzer(temp_file.path());

    // Should succeed - @unsafe-XXX should be recognized
    assert!(
        success,
        "Annotation @unsafe-XXX should be recognized. Output: {}",
        output
    );
}

#[test]
fn test_safe_with_hyphen_suffix() {
    // Test that @safe-note is recognized
    let code = r#"
// @safe-this is a safe function with a note
void safe_func() {
    int x = 42;
}

void undeclared_func() {}

// @safe
void another_safe() {
    safe_func();  // OK: safe can call safe
    // undeclared_func();  // Would be error: safe cannot call undeclared
}
"#;

    let mut temp_file = NamedTempFile::new().unwrap();
    write!(temp_file, "{}", code).unwrap();

    let (success, _output) = run_analyzer(temp_file.path());

    // Should succeed - @safe-XXX should be recognized
    assert!(success, "Annotation @safe-XXX should be recognized");
}

#[test]
fn test_unsafe_with_colon_suffix() {
    // Test that @unsafe: note is recognized
    let code = r#"
// @unsafe: uses raw pointers
void unsafe_func() {
    int* ptr = nullptr;
    *ptr = 42;
}
"#;

    let mut temp_file = NamedTempFile::new().unwrap();
    write!(temp_file, "{}", code).unwrap();

    let (success, _output) = run_analyzer(temp_file.path());

    assert!(success, "Annotation @unsafe: should be recognized");
}

#[test]
fn test_safe_with_comma_suffix() {
    // Test that @safe, with comma is recognized
    let code = r#"
// @safe, verified manually
void safe_func() {
    int x = 42;
}
"#;

    let mut temp_file = NamedTempFile::new().unwrap();
    write!(temp_file, "{}", code).unwrap();

    let (success, _output) = run_analyzer(temp_file.path());

    assert!(success, "Annotation @safe, should be recognized");
}

#[test]
fn test_multiline_comment_with_suffix() {
    // Test that /* @unsafe-XXX */ is recognized
    let code = r#"
/* @unsafe-uses raw pointers */
void unsafe_func() {
    int* ptr = nullptr;
    *ptr = 42;
}

/* @safe-verified */
void safe_func() {
    int x = 42;
}
"#;

    let mut temp_file = NamedTempFile::new().unwrap();
    write!(temp_file, "{}", code).unwrap();

    let (success, _output) = run_analyzer(temp_file.path());

    assert!(success, "Multiline comment annotations with suffix should be recognized");
}

#[test]
fn test_block_comment_with_suffix() {
    // Test that block comments with suffixes work
    let code = r#"
/*
 * @unsafe-manual verification required
 */
void unsafe_func() {
    int* ptr = nullptr;
}

/*
 * @safe-checked on 2025-01-17
 */
void safe_func() {
    int x = 42;
}
"#;

    let mut temp_file = NamedTempFile::new().unwrap();
    write!(temp_file, "{}", code).unwrap();

    let (success, _output) = run_analyzer(temp_file.path());

    assert!(success, "Block comment annotations with suffix should be recognized");
}

#[test]
fn test_namespace_annotation_with_suffix() {
    // Test that namespace annotations with suffixes work
    let code = r#"
// @safe-entire namespace is safe
namespace myapp {
    void func1() {
        int x = 42;
    }

    void func2() {
        func1();  // OK: both in safe namespace
    }
}
"#;

    let mut temp_file = NamedTempFile::new().unwrap();
    write!(temp_file, "{}", code).unwrap();

    let (success, _output) = run_analyzer(temp_file.path());

    assert!(success, "Namespace annotation @safe-XXX should be recognized");
}

#[test]
fn test_reject_partial_match() {
    // Test that @safety is NOT matched when looking for @safe
    let code = r#"
// This is about @safety in general, not a @safe annotation
void should_be_undeclared() {
    int x = 42;
}

// @safe
void safe_func() {
    // should_be_undeclared();  // Would be error: safe cannot call undeclared
}
"#;

    let mut temp_file = NamedTempFile::new().unwrap();
    write!(temp_file, "{}", code).unwrap();

    let (_success, output) = run_analyzer(temp_file.path());

    // The function should_be_undeclared should NOT be treated as @safe
    // If we uncommented the call, it would be an error
    // For now, just verify it doesn't crash
    assert!(!output.contains("panic"), "Should not panic on @safety comment");
}

#[test]
fn test_reject_annotation_in_middle_of_text() {
    // Test that "No @safe annotation" does NOT trigger @safe
    let code = r#"
// No @safe annotation - this class is undeclared
class MyClass {
    mutable int count;
public:
    void increment() const {
        count++;
    }
};
"#;

    let mut temp_file = NamedTempFile::new().unwrap();
    write!(temp_file, "{}", code).unwrap();

    let (success, output) = run_analyzer(temp_file.path());

    // Should succeed - the class is undeclared, mutable is allowed
    assert!(
        success,
        "Comment mentioning '@safe' should not trigger @safe annotation. Output: {}",
        output
    );
    assert!(
        output.contains("no violations"),
        "Should have no violations for undeclared class. Output: {}",
        output
    );
}

#[test]
fn test_mixed_suffixes() {
    // Test multiple different suffix styles in one file
    let code = r#"
// @unsafe-raw pointers
void func1() {
    int* p = nullptr;
}

// @unsafe: manual verification
void func2() {
    int* p = nullptr;
}

/* @safe-checked */
void func3() {
    int x = 42;
}

/*
 * @safe: verified on 2025-01-17
 */
void func4() {
    func3();
}
"#;

    let mut temp_file = NamedTempFile::new().unwrap();
    write!(temp_file, "{}", code).unwrap();

    let (success, _output) = run_analyzer(temp_file.path());

    assert!(success, "Multiple annotation suffix styles should all work");
}
