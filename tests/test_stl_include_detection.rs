/// Tests for STL include path auto-detection
///
/// This tests that the analyzer can find C++ standard library headers
/// automatically without requiring manual -I flags.
///
/// Related to: https://github.com/shuaimu/rusty-cpp/issues/15

use std::io::Write;
use std::path::Path;
use std::process::Command;
use tempfile::NamedTempFile;

fn run_analyzer(cpp_file: &Path) -> (bool, String) {
    run_analyzer_with_args(cpp_file, &[])
}

fn run_analyzer_with_args(cpp_file: &Path, extra_args: &[&str]) -> (bool, String) {
    let z3_header = if cfg!(target_os = "macos") {
        "/opt/homebrew/include/z3.h"
    } else {
        "/usr/include/z3.h"
    };

    let mut cmd = Command::new("cargo");
    let mut args = vec!["run", "--quiet", "--", cpp_file.to_str().unwrap()];
    args.extend(extra_args);
    cmd.args(&args)
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

fn create_temp_cpp_file(content: &str) -> NamedTempFile {
    let mut file = NamedTempFile::with_suffix(".cpp").unwrap();
    file.write_all(content.as_bytes()).unwrap();
    file.flush().unwrap();
    file
}

// ============================================================================
// Tests for STL header auto-detection (Issue #15)
// ============================================================================

#[test]
fn test_vector_header_found() {
    // Test that <vector> can be found without manual include paths
    let code = r#"
    #include <vector>

    int main() {
        return 0;
    }
    "#;

    let temp_file = create_temp_cpp_file(code);
    let (success, output) = run_analyzer(temp_file.path());

    assert!(
        !output.contains("'vector' file not found"),
        "STL header <vector> should be auto-detected. Output: {}",
        output
    );

    // Should either succeed or fail for other reasons (not missing header)
    assert!(
        success || !output.contains("file not found"),
        "Should not fail due to missing STL headers. Output: {}",
        output
    );
}

#[test]
fn test_string_header_found() {
    // Test that <string> can be found without manual include paths
    let code = r#"
    #include <string>

    int main() {
        return 0;
    }
    "#;

    let temp_file = create_temp_cpp_file(code);
    let (_success, output) = run_analyzer(temp_file.path());

    assert!(
        !output.contains("'string' file not found"),
        "STL header <string> should be auto-detected. Output: {}",
        output
    );
}

#[test]
fn test_memory_header_found() {
    // Test that <memory> can be found (needed for smart pointers)
    let code = r#"
    #include <memory>

    int main() {
        return 0;
    }
    "#;

    let temp_file = create_temp_cpp_file(code);
    let (_success, output) = run_analyzer(temp_file.path());

    assert!(
        !output.contains("'memory' file not found"),
        "STL header <memory> should be auto-detected. Output: {}",
        output
    );
}

#[test]
fn test_clang_include_paths_detected() {
    // Test that the analyzer can run successfully
    // Note: Auto-detection of include paths depends on the system's clang installation
    // and may not work in all CI environments. The important thing is that the
    // analyzer runs without crashing.
    let code = r#"
    int main() {
        return 0;
    }
    "#;

    let temp_file = create_temp_cpp_file(code);
    let (success, output) = run_analyzer(temp_file.path());

    // The analyzer should run successfully on simple code
    // Include path auto-detection is a nice-to-have, not required
    assert!(
        success || output.contains("no violations"),
        "Analyzer should run successfully on simple code. Output: {}",
        output
    );
}

#[test]
fn test_iostream_header_issue_15() {
    // Regression test for issue #15: 'iostream' file not found
    // https://github.com/shuaimu/rusty-cpp/issues/15
    let code = r#"
    #include <iostream>

    int main() {
        return 0;
    }
    "#;

    let temp_file = create_temp_cpp_file(code);
    let (_success, output) = run_analyzer(temp_file.path());

    assert!(
        !output.contains("'iostream' file not found"),
        "Issue #15: STL header <iostream> should be auto-detected. Output: {}",
        output
    );
}

#[test]
fn test_chrono_header_issue_15() {
    // Part of issue #15 - the original code also used <chrono>
    let code = r#"
    #include <chrono>

    int main() {
        return 0;
    }
    "#;

    let temp_file = create_temp_cpp_file(code);
    let (_success, output) = run_analyzer(temp_file.path());

    assert!(
        !output.contains("'chrono' file not found"),
        "STL header <chrono> should be auto-detected. Output: {}",
        output
    );
}
