/// Test external annotations with qualified function names (std::, namespace::, etc.)
/// This tests the bug where external annotations don't match when function names
/// are called with namespace qualification (e.g., std::dynamic_pointer_cast)

use std::io::Write;
use std::path::Path;
use std::process::Command;
use tempfile::TempDir;

fn run_analyzer_with_compile_commands(cpp_file: &Path, compile_commands_dir: &Path) -> (bool, String) {
    let z3_header = if cfg!(target_os = "macos") {
        "/opt/homebrew/include/z3.h"
    } else {
        "/usr/include/z3.h"
    };

    let mut cmd = Command::new("cargo");
    cmd.args(&[
        "run", "--quiet", "--",
        cpp_file.to_str().unwrap(),
        "--compile-commands",
        compile_commands_dir.join("compile_commands.json").to_str().unwrap()
    ])
        .env("Z3_SYS_Z3_HEADER", z3_header);

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

#[test]
fn test_qualified_std_function_call() {
    // Test that std::dynamic_pointer_cast matches external annotation for "dynamic_pointer_cast"
    let source_content = r#"
#include <memory>

class Base {};
class Derived : public Base {};

// @external: {
//   dynamic_pointer_cast: [unsafe, template<T, U>(const std::shared_ptr<U>&) -> std::shared_ptr<T>]
// }

// @safe
void test_function() {
    std::shared_ptr<Base> base = std::make_shared<Derived>();
    // This calls std::dynamic_pointer_cast (with std:: prefix)
    // But annotation only says "dynamic_pointer_cast" (without std::)
    auto derived = std::dynamic_pointer_cast<Derived>(base);
}
"#;

    let temp_dir = TempDir::new().unwrap();
    let source_path = temp_dir.path().join("test.cpp");
    std::fs::write(&source_path, source_content).unwrap();

    // Create compile_commands.json
    let compile_commands = format!(r#"[
  {{
    "directory": "{}",
    "file": "{}",
    "command": "g++ -std=c++17 -c {}"
  }}
]"#, temp_dir.path().display(), source_path.display(), source_path.display());

    let compile_commands_path = temp_dir.path().join("compile_commands.json");
    std::fs::write(&compile_commands_path, compile_commands).unwrap();

    let (success, output) = run_analyzer_with_compile_commands(&source_path, temp_dir.path());

    println!("=== Test: std::dynamic_pointer_cast with annotation 'dynamic_pointer_cast' ===");
    println!("{}", output);

    // This test documents the CURRENT behavior (which may be buggy)
    // If the test fails, it means the bug is fixed!
    if output.contains("undeclared") {
        println!("BUG CONFIRMED: External annotation 'dynamic_pointer_cast' doesn't match call 'std::dynamic_pointer_cast'");
    } else if success {
        println!("BUG FIXED: External annotation now matches despite different qualification!");
    } else {
        println!("Unexpected failure: {}", output);
    }
}

#[test]
fn test_qualified_std_annotation_exact_match() {
    // Test that std::dynamic_pointer_cast matches annotation for "std::dynamic_pointer_cast"
    let source_content = r#"
#include <memory>

class Base {};
class Derived : public Base {};

// @external: {
//   std::dynamic_pointer_cast: [unsafe, template<T, U>(const std::shared_ptr<U>&) -> std::shared_ptr<T>]
// }

// @safe
void test_function() {
    std::shared_ptr<Base> base = std::make_shared<Derived>();
    auto derived = std::dynamic_pointer_cast<Derived>(base);
}
"#;

    let temp_dir = TempDir::new().unwrap();
    let source_path = temp_dir.path().join("test.cpp");
    std::fs::write(&source_path, source_content).unwrap();

    // Create compile_commands.json
    let compile_commands = format!(r#"[
  {{
    "directory": "{}",
    "file": "{}",
    "command": "g++ -std=c++17 -c {}"
  }}
]"#, temp_dir.path().display(), source_path.display(), source_path.display());

    let compile_commands_path = temp_dir.path().join("compile_commands.json");
    std::fs::write(&compile_commands_path, compile_commands).unwrap();

    let (success, output) = run_analyzer_with_compile_commands(&source_path, temp_dir.path());

    println!("=== Test: std::dynamic_pointer_cast with annotation 'std::dynamic_pointer_cast' ===");
    println!("{}", output);

    // With fully qualified annotation, it should work
    assert!(
        success,
        "Should succeed with exact qualified match. Output: {}",
        output
    );
}

#[test]
fn test_unqualified_call_with_qualified_annotation() {
    // Test unqualified call (via using directive) with qualified annotation
    let source_content = r#"
#include <memory>
using std::dynamic_pointer_cast;
using std::shared_ptr;
using std::make_shared;

class Base {};
class Derived : public Base {};

// @external: {
//   std::dynamic_pointer_cast: [unsafe, template<T, U>(const shared_ptr<U>&) -> shared_ptr<T>]
// }

// @safe
void test_function() {
    shared_ptr<Base> base = make_shared<Derived>();
    // Unqualified call (no std:: prefix)
    auto derived = dynamic_pointer_cast<Derived>(base);
}
"#;

    let temp_dir = TempDir::new().unwrap();
    let source_path = temp_dir.path().join("test.cpp");
    std::fs::write(&source_path, source_content).unwrap();

    // Create compile_commands.json
    let compile_commands = format!(r#"[
  {{
    "directory": "{}",
    "file": "{}",
    "command": "g++ -std=c++17 -c {}"
  }}
]"#, temp_dir.path().display(), source_path.display(), source_path.display());

    let compile_commands_path = temp_dir.path().join("compile_commands.json");
    std::fs::write(&compile_commands_path, compile_commands).unwrap();

    let (success, output) = run_analyzer_with_compile_commands(&source_path, temp_dir.path());

    println!("=== Test: unqualified call with qualified annotation ===");
    println!("{}", output);

    // This may fail if libclang returns the canonical name (std::...) but annotation only has unqualified
    if output.contains("undeclared") {
        println!("BUG: Annotation 'std::dynamic_pointer_cast' doesn't match unqualified call");
    }
}

#[test]
fn test_pattern_matching_with_wildcard() {
    // Test that *::dynamic_pointer_cast pattern works
    let source_content = r#"
#include <memory>

class Base {};
class Derived : public Base {};

// @external_whitelist: [
//   "*::dynamic_pointer_cast"
// ]

// @external: {
//   "*::dynamic_pointer_cast": [unsafe, template<T, U>(const std::shared_ptr<U>&) -> std::shared_ptr<T>]
// }

// @safe
void test_function() {
    std::shared_ptr<Base> base = std::make_shared<Derived>();
    auto derived = std::dynamic_pointer_cast<Derived>(base);
}
"#;

    let temp_dir = TempDir::new().unwrap();
    let source_path = temp_dir.path().join("test.cpp");
    std::fs::write(&source_path, source_content).unwrap();

    // Create compile_commands.json
    let compile_commands = format!(r#"[
  {{
    "directory": "{}",
    "file": "{}",
    "command": "g++ -std=c++17 -c {}"
  }}
]"#, temp_dir.path().display(), source_path.display(), source_path.display());

    let compile_commands_path = temp_dir.path().join("compile_commands.json");
    std::fs::write(&compile_commands_path, compile_commands).unwrap();

    let (success, output) = run_analyzer_with_compile_commands(&source_path, temp_dir.path());

    println!("=== Test: pattern matching with *::dynamic_pointer_cast ===");
    println!("{}", output);

    // Pattern matching should work regardless of qualification
    assert!(
        success || !output.contains("dynamic_pointer_cast") && !output.contains("undeclared"),
        "Pattern *::dynamic_pointer_cast should match any qualified call. Output: {}",
        output
    );
}
