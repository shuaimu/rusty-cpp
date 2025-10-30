/// Test that external annotations in C++ comments are properly parsed
/// This test verifies the exact format used in coordinator.cc

use std::process::Command;
use std::path::Path;
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
fn test_comment_block_format() {
    // This matches the EXACT format from coordinator.cc
    let source_content = r#"
#include <memory>

class Base {};
class Derived : public Base {};

// External annotations for std library template functions
// @external: {
//   std::dynamic_pointer_cast: [unsafe, template<T, U>(const std::shared_ptr<U>& ptr) -> std::shared_ptr<T>]
//   dynamic_pointer_cast: [unsafe, template<T, U>(const std::shared_ptr<U>& ptr) -> std::shared_ptr<T>]
// }

// @unsafe - calling unsafe function dynamic_pointer_cast
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

    println!("=== Test: Comment block format (exact from coordinator.cc) ===");
    println!("{}", output);

    if output.contains("undeclared") && output.contains("dynamic_pointer_cast") {
        println!("BUG CONFIRMED: External annotations in comment block not being parsed!");
        println!("The annotation format matches coordinator.cc exactly, but it's not working.");
    } else if success {
        println!("SUCCESS: Comment block annotations are being parsed correctly");
    } else {
        println!("Other error (not related to external annotations): {}", output);
    }

    // For now, this test documents the bug
    // When fixed, this should pass
}

#[test]
fn test_inline_external_annotation() {
    // Test with inline format (not in comments)
    let source_content = r#"
#include <memory>

class Base {};
class Derived : public Base {};

@external: {
  dynamic_pointer_cast: [unsafe, template<T, U>(const std::shared_ptr<U>&) -> std::shared_ptr<T>]
}

// @unsafe - calling unsafe function dynamic_pointer_cast
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

    println!("=== Test: Inline external annotation (not in comments) ===");
    println!("{}", output);

    // Inline format should work
    assert!(
        success,
        "Inline external annotations should work. Output: {}",
        output
    );
}
