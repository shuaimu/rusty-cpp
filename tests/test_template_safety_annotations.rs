/// Test that safety annotations work correctly for template functions
/// This tests the specific bug: template functions in headers not being recognized

use std::io::Write;
use std::path::Path;
use std::process::Command;
use tempfile::{NamedTempFile, TempDir};

fn run_analyzer(cpp_file: &Path) -> (bool, String) {
    let z3_header = if cfg!(target_os = "macos") {
        "/opt/homebrew/include/z3.h"
    } else {
        "/usr/include/z3.h"
    };

    let mut cmd = Command::new("cargo");
    cmd.args(&["run", "--quiet", "--", cpp_file.to_str().unwrap()])
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

fn create_temp_file_with_suffix(content: &str, suffix: &str) -> (TempDir, std::path::PathBuf) {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join(format!("test{}", suffix));
    std::fs::write(&file_path, content).unwrap();
    (temp_dir, file_path)
}

#[test]
fn test_template_function_unsafe_annotation_in_header() {
    // Create header with template function marked @unsafe
    let header_content = r#"
#pragma once

namespace test {

// @unsafe
template <typename T>
T create_thing() {
    return T{};
}

} // namespace test
"#;

    // With two-state model: @safe calling @unsafe without @unsafe block should fail
    // This test verifies the annotation is recognized (error message says "non-safe", not "undeclared")
    let source_content = r#"
#include "test.h"

namespace test {

// @safe
void safe_caller() {
    int x = create_thing<int>();  // ERROR: @safe calling @unsafe without @unsafe block
}

} // namespace test
"#;

    let temp_dir = TempDir::new().unwrap();

    // Write header
    let header_path = temp_dir.path().join("test.h");
    std::fs::write(&header_path, header_content).unwrap();

    // Write source
    let source_path = temp_dir.path().join("test.cpp");
    std::fs::write(&source_path, source_content).unwrap();

    // Add include path
    let include_arg = format!("-I{}", temp_dir.path().display());

    let mut cmd = Command::new("cargo");
    cmd.args(&["run", "--quiet", "--", source_path.to_str().unwrap(), &include_arg]);

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

    // With two-state model: @safe CANNOT call @unsafe without @unsafe block
    // Should FAIL - but the key is the error message says "non-safe" not "undeclared"
    // This verifies the @unsafe annotation in header was recognized
    assert!(
        !output.status.success() || full_output.contains("non-safe") || full_output.contains("@unsafe"),
        "Template function marked @unsafe should require @unsafe block to call from @safe. Output: {}",
        full_output
    );

    // Should NOT contain "undeclared" error - proves the annotation was recognized
    assert!(
        !full_output.contains("undeclared"),
        "Should not report function as undeclared when it's marked @unsafe in header. Output: {}",
        full_output
    );
}

#[test]
fn test_variadic_template_function_unsafe_annotation_in_header() {
    // This replicates the exact scenario from the bug report
    let header_content = r#"
#pragma once
#include <memory>

namespace rrr {

class Reactor {
public:
    // @unsafe
    template <typename Ev, typename... Args>
    static std::shared_ptr<Ev> CreateSpEvent(Args&&... args) {
        return std::make_shared<Ev>(args...);
    }
};

} // namespace rrr
"#;

    let source_content = r#"
#include "reactor.h"

class IntEvent {
public:
    int value = 0;
};

namespace janus {

// @unsafe
std::shared_ptr<IntEvent> SendAppendEntries() {
    // This is the exact call that fails in the bug report
    auto ret = rrr::Reactor::CreateSpEvent<IntEvent>();
    return ret;
}

} // namespace janus
"#;

    let temp_dir = TempDir::new().unwrap();

    // Write header
    let header_path = temp_dir.path().join("reactor.h");
    std::fs::write(&header_path, header_content).unwrap();

    // Write source
    let source_path = temp_dir.path().join("commo.cpp");
    std::fs::write(&source_path, source_content).unwrap();

    // Add include path
    let include_arg = format!("-I{}", temp_dir.path().display());

    let mut cmd = Command::new("cargo");
    cmd.args(&["run", "--quiet", "--", source_path.to_str().unwrap(), &include_arg]);

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

    println!("Output: {}", full_output);

    // Should succeed because @safe can call @unsafe
    assert!(
        output.status.success(),
        "Variadic template function CreateSpEvent should be recognized as @unsafe from header. Output: {}",
        full_output
    );

    // Should NOT contain "undeclared" error
    assert!(
        !full_output.contains("undeclared"),
        "CreateSpEvent should not be reported as undeclared when marked @unsafe in header. Output: {}",
        full_output
    );
}

#[test]
fn test_template_function_safe_annotation_in_header() {
    // Test that @safe template functions are also recognized
    let header_content = r#"
#pragma once

namespace test {

// @safe
template <typename T>
T safe_create() {
    T result{};
    return result;
}

} // namespace test
"#;

    let source_content = r#"
#include "test.h"

namespace test {

// @safe
void safe_caller() {
    int x = safe_create<int>();  // Should be OK
}

} // namespace test
"#;

    let temp_dir = TempDir::new().unwrap();

    // Write header
    let header_path = temp_dir.path().join("test.h");
    std::fs::write(&header_path, header_content).unwrap();

    // Write source
    let source_path = temp_dir.path().join("test.cpp");
    std::fs::write(&source_path, source_content).unwrap();

    // Add include path
    let include_arg = format!("-I{}", temp_dir.path().display());

    let mut cmd = Command::new("cargo");
    cmd.args(&["run", "--quiet", "--", source_path.to_str().unwrap(), &include_arg]);

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

    // Should succeed
    assert!(
        output.status.success(),
        "Safe template function should be recognized. Output: {}",
        full_output
    );
}
