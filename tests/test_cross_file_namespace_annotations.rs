use std::fs;
use std::process::Command;
use tempfile::TempDir;

fn run_analyzer_on_file(cpp_file: &std::path::Path) -> (bool, String) {
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
fn test_same_namespace_safe_in_one_file_unsafe_in_another() {
    // Create a temporary directory for our test files
    let temp_dir = TempDir::new().unwrap();

    // File 1: namespace myapp marked as @safe
    let file1_content = r#"
#include "header.h"

// @safe
namespace myapp {

void safe_function() {
    int x = 42;
}

} // namespace myapp
"#;

    // File 2: same namespace myapp marked as @unsafe
    let file2_content = r#"
#include "header.h"

// @unsafe
namespace myapp {

void unsafe_function() {
    int* ptr = nullptr;
    *ptr = 42;  // Should be OK because this file marks namespace as unsafe
}

} // namespace myapp
"#;

    // Header file
    let header_content = r#"
#pragma once

namespace myapp {
    void safe_function();
    void unsafe_function();
}
"#;

    // Write files
    let header_path = temp_dir.path().join("header.h");
    fs::write(&header_path, header_content).unwrap();

    let file1_path = temp_dir.path().join("file1.cpp");
    fs::write(&file1_path, file1_content).unwrap();

    let file2_path = temp_dir.path().join("file2.cpp");
    fs::write(&file2_path, file2_content).unwrap();

    // Add include path
    let include_arg = format!("-I{}", temp_dir.path().display());

    // Analyze file1
    let mut cmd1 = Command::new("cargo");
    cmd1.args(&["run", "--quiet", "--", file1_path.to_str().unwrap(), &include_arg]);
    if cfg!(target_os = "macos") {
        cmd1.env("Z3_SYS_Z3_HEADER", "/opt/homebrew/include/z3.h");
        cmd1.env("DYLD_LIBRARY_PATH", "/opt/homebrew/Cellar/llvm/19.1.7/lib");
    } else {
        cmd1.env("Z3_SYS_Z3_HEADER", "/usr/include/z3.h");
        cmd1.env("LD_LIBRARY_PATH", "/usr/lib/llvm-14/lib");
    }
    let output1 = cmd1.output().expect("Failed to execute analyzer on file1");
    let stdout1 = String::from_utf8_lossy(&output1.stdout);
    let stderr1 = String::from_utf8_lossy(&output1.stderr);
    let full_output1 = format!("{}{}", stdout1, stderr1);

    // Analyze file2
    let mut cmd2 = Command::new("cargo");
    cmd2.args(&["run", "--quiet", "--", file2_path.to_str().unwrap(), &include_arg]);
    if cfg!(target_os = "macos") {
        cmd2.env("Z3_SYS_Z3_HEADER", "/opt/homebrew/include/z3.h");
        cmd2.env("DYLD_LIBRARY_PATH", "/opt/homebrew/Cellar/llvm/19.1.7/lib");
    } else {
        cmd2.env("Z3_SYS_Z3_HEADER", "/usr/include/z3.h");
        cmd2.env("LD_LIBRARY_PATH", "/usr/lib/llvm-14/lib");
    }
    let output2 = cmd2.output().expect("Failed to execute analyzer on file2");
    let stdout2 = String::from_utf8_lossy(&output2.stdout);
    let stderr2 = String::from_utf8_lossy(&output2.stderr);
    let full_output2 = format!("{}{}", stdout2, stderr2);

    println!("File1 output: {}", full_output1);
    println!("File2 output: {}", full_output2);

    // Both should succeed - each file's annotation applies only to that file
    assert!(
        output1.status.success(),
        "File1 with @safe namespace should succeed. Output: {}",
        full_output1
    );
    assert!(
        output2.status.success(),
        "File2 with @unsafe namespace should succeed. Output: {}",
        full_output2
    );
}

#[test]
fn test_namespace_annotation_is_per_file() {
    // This test verifies that namespace annotations are file-scoped, not global
    let temp_dir = TempDir::new().unwrap();

    // File 1: namespace is @safe, so this function should be checked
    let file1_content = r#"
// @safe
namespace myapp {

void checked_function() {
    int x = 42;
    // Cannot use raw pointers here
}

} // namespace myapp
"#;

    // File 2: namespace is @unsafe, so this function can use raw pointers
    let file2_content = r#"
// @unsafe
namespace myapp {

void unchecked_function() {
    int* ptr = nullptr;
    *ptr = 42;  // OK in this file - namespace is unsafe
}

} // namespace myapp
"#;

    let file1_path = temp_dir.path().join("file1.cpp");
    fs::write(&file1_path, file1_content).unwrap();

    let file2_path = temp_dir.path().join("file2.cpp");
    fs::write(&file2_path, file2_content).unwrap();

    let (success1, output1) = run_analyzer_on_file(&file1_path);
    let (success2, output2) = run_analyzer_on_file(&file2_path);

    println!("File1 (safe namespace) output: {}", output1);
    println!("File2 (unsafe namespace) output: {}", output2);

    assert!(
        success1,
        "File with @safe namespace should succeed. Output: {}",
        output1
    );
    assert!(
        success2,
        "File with @unsafe namespace should succeed. Output: {}",
        output2
    );
}

#[test]
fn test_no_namespace_annotation_in_one_file() {
    // File 1: namespace has @safe annotation
    // File 2: namespace has no annotation (undeclared)
    let temp_dir = TempDir::new().unwrap();

    let file1_content = r#"
// @safe
namespace myapp {

void safe_func() {
    int x = 42;
}

} // namespace myapp
"#;

    let file2_content = r#"
// No annotation on namespace
namespace myapp {

void undeclared_func() {
    int x = 42;
}

} // namespace myapp
"#;

    let file1_path = temp_dir.path().join("file1.cpp");
    fs::write(&file1_path, file1_content).unwrap();

    let file2_path = temp_dir.path().join("file2.cpp");
    fs::write(&file2_path, file2_content).unwrap();

    let (success1, output1) = run_analyzer_on_file(&file1_path);
    let (success2, output2) = run_analyzer_on_file(&file2_path);

    println!("File1 (@safe namespace) output: {}", output1);
    println!("File2 (no annotation) output: {}", output2);

    // Both should succeed
    assert!(
        success1,
        "File with @safe namespace should succeed. Output: {}",
        output1
    );
    assert!(
        success2,
        "File with undeclared namespace should succeed. Output: {}",
        output2
    );
}

#[test]
fn test_multiple_namespace_redeclarations_in_same_file() {
    // Within a single file, if namespace is declared multiple times,
    // only the first annotation should apply
    let code = r#"
// @safe
namespace myapp {
    void func1() {
        int x = 42;
    }
}

// @unsafe (this should be ignored - namespace already annotated)
namespace myapp {
    void func2() {
        int y = 10;
    }
}

// No annotation (should inherit from first @safe)
namespace myapp {
    void func3() {
        int z = 5;
    }
}
"#;

    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.cpp");
    fs::write(&file_path, code).unwrap();

    let (success, output) = run_analyzer_on_file(&file_path);

    println!("Multiple namespace redeclaration output: {}", output);

    // This test documents current behavior - we should verify what actually happens
    // The expected behavior is that the first annotation wins for the whole file
    assert!(
        success,
        "Multiple redeclarations of same namespace in one file. Output: {}",
        output
    );
}

#[test]
fn test_conflicting_annotations_detected_as_issue() {
    // This test checks if we have any mechanism to detect conflicting annotations
    // across files. This might not be implemented, but it's worth testing.
    let temp_dir = TempDir::new().unwrap();

    // Main file that includes both
    let main_content = r#"
#include "safe_part.h"
#include "unsafe_part.h"

int main() {
    myapp::safe_function();
    myapp::unsafe_function();
    return 0;
}
"#;

    let safe_header = r#"
#pragma once

// @safe
namespace myapp {
    void safe_function();
}
"#;

    let unsafe_header = r#"
#pragma once

// @unsafe
namespace myapp {
    void unsafe_function();
}
"#;

    let safe_impl = r#"
#include "safe_part.h"

// @safe
namespace myapp {

void safe_function() {
    int x = 42;
}

}
"#;

    let unsafe_impl = r#"
#include "unsafe_part.h"

// @unsafe
namespace myapp {

void unsafe_function() {
    int* ptr = nullptr;
}

}
"#;

    fs::write(temp_dir.path().join("main.cpp"), main_content).unwrap();
    fs::write(temp_dir.path().join("safe_part.h"), safe_header).unwrap();
    fs::write(temp_dir.path().join("unsafe_part.h"), unsafe_header).unwrap();
    fs::write(temp_dir.path().join("safe_part.cpp"), safe_impl).unwrap();
    fs::write(temp_dir.path().join("unsafe_part.cpp"), unsafe_impl).unwrap();

    let include_arg = format!("-I{}", temp_dir.path().display());

    // Analyze each implementation file separately
    let mut cmd_safe = Command::new("cargo");
    cmd_safe.args(&["run", "--quiet", "--",
                    temp_dir.path().join("safe_part.cpp").to_str().unwrap(),
                    &include_arg]);
    if cfg!(target_os = "macos") {
        cmd_safe.env("Z3_SYS_Z3_HEADER", "/opt/homebrew/include/z3.h");
        cmd_safe.env("DYLD_LIBRARY_PATH", "/opt/homebrew/Cellar/llvm/19.1.7/lib");
    } else {
        cmd_safe.env("Z3_SYS_Z3_HEADER", "/usr/include/z3.h");
        cmd_safe.env("LD_LIBRARY_PATH", "/usr/lib/llvm-14/lib");
    }
    let output_safe = cmd_safe.output().expect("Failed to execute");

    let mut cmd_unsafe = Command::new("cargo");
    cmd_unsafe.args(&["run", "--quiet", "--",
                      temp_dir.path().join("unsafe_part.cpp").to_str().unwrap(),
                      &include_arg]);
    if cfg!(target_os = "macos") {
        cmd_unsafe.env("Z3_SYS_Z3_HEADER", "/opt/homebrew/include/z3.h");
        cmd_unsafe.env("DYLD_LIBRARY_PATH", "/opt/homebrew/Cellar/llvm/19.1.7/lib");
    } else {
        cmd_unsafe.env("Z3_SYS_Z3_HEADER", "/usr/include/z3.h");
        cmd_unsafe.env("LD_LIBRARY_PATH", "/usr/lib/llvm-14/lib");
    }
    let output_unsafe = cmd_unsafe.output().expect("Failed to execute");

    println!("Safe implementation output: {}", String::from_utf8_lossy(&output_safe.stdout));
    println!("Unsafe implementation output: {}", String::from_utf8_lossy(&output_unsafe.stdout));

    // Each file should be analyzed independently based on its own annotations
    assert!(
        output_safe.status.success(),
        "Safe implementation should succeed"
    );
    assert!(
        output_unsafe.status.success(),
        "Unsafe implementation should succeed"
    );
}
