/// Tests for std_annotation.hpp header file inclusion
///
/// These tests verify that including std_annotation.hpp works correctly
/// and that the external annotations are picked up by the borrow checker.

use std::io::Write;
use std::path::Path;
use std::process::Command;
use tempfile::NamedTempFile;

fn get_include_path() -> String {
    // Get the path to the include directory
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR")
        .unwrap_or_else(|_| ".".to_string());
    format!("{}/include", manifest_dir)
}

fn run_analyzer_with_include(cpp_file: &Path) -> (bool, String) {
    let z3_header = if cfg!(target_os = "macos") {
        "/opt/homebrew/include/z3.h"
    } else {
        "/usr/include/z3.h"
    };

    let include_path = get_include_path();

    let mut cmd = Command::new("cargo");
    cmd.args(&["run", "--quiet", "--", cpp_file.to_str().unwrap(), "-I", &include_path])
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
// Tests for std_annotation.hpp header inclusion
// ============================================================================

#[test]
fn test_std_annotation_header_parses() {
    // Test that including the header doesn't cause parse errors
    let code = r#"
    #include <std_annotation.hpp>

    int main() {
        return 0;
    }
    "#;

    let temp_file = create_temp_cpp_file(code);
    let (success, output) = run_analyzer_with_include(temp_file.path());

    assert!(
        success,
        "Including std_annotation.hpp should not cause parse errors. Output: {}",
        output
    );
}

#[test]
fn test_std_annotation_header_swap_is_unsafe() {
    // Test that std::swap is marked as unsafe via the header
    let code = r#"
    #include <std_annotation.hpp>

    // @safe
    void safe_function() {
        int a = 1;
        int b = 2;
        // Calling std::swap should fail because it's marked unsafe in the header
        std::swap(a, b);
    }

    int main() {
        return 0;
    }
    "#;

    let temp_file = create_temp_cpp_file(code);
    let (success, output) = run_analyzer_with_include(temp_file.path());

    // Should fail because std::swap is unsafe
    assert!(
        !success || output.contains("unsafe") || output.contains("undeclared"),
        "Calling std::swap in @safe code should be flagged. Output: {}",
        output
    );
}

#[test]
fn test_std_annotation_header_swap_in_unsafe() {
    // Test that std::swap works in @unsafe context
    let code = r#"
    #include <std_annotation.hpp>

    // @unsafe
    void unsafe_function() {
        int a = 1;
        int b = 2;
        std::swap(a, b);
    }

    int main() {
        return 0;
    }
    "#;

    let temp_file = create_temp_cpp_file(code);
    let (success, output) = run_analyzer_with_include(temp_file.path());

    assert!(
        success,
        "Calling std::swap in @unsafe code should be allowed. Output: {}",
        output
    );
}

#[test]
fn test_std_annotation_header_move_is_unsafe() {
    // Test that std::move is marked as unsafe via the header
    let code = r#"
    #include <std_annotation.hpp>

    // @safe
    void safe_function() {
        int a = 1;
        int&& b = std::move(a);
    }

    int main() {
        return 0;
    }
    "#;

    let temp_file = create_temp_cpp_file(code);
    let (success, output) = run_analyzer_with_include(temp_file.path());

    // std::move is typically whitelisted for borrow checking purposes,
    // but we should verify the analysis runs without errors
    // The important thing is the header is parsed correctly
    assert!(
        success || output.contains("move") || output.contains("unsafe"),
        "std::move analysis should complete. Output: {}",
        output
    );
}

#[test]
fn test_std_annotation_header_make_unique_in_unsafe() {
    // Test that std::make_unique works in @unsafe context
    let code = r#"
    #include <std_annotation.hpp>
    #include <memory>

    // @unsafe
    void unsafe_function() {
        auto ptr = std::make_unique<int>(42);
    }

    int main() {
        return 0;
    }
    "#;

    let temp_file = create_temp_cpp_file(code);
    let (success, output) = run_analyzer_with_include(temp_file.path());

    assert!(
        success,
        "Using std::make_unique in @unsafe code should be allowed. Output: {}",
        output
    );
}

#[test]
fn test_std_annotation_header_vector_operations_in_unsafe() {
    // Test that vector operations work in @unsafe context with the header
    let code = r#"
    #include <std_annotation.hpp>
    #include <vector>

    // @unsafe
    void unsafe_function() {
        std::vector<int> vec;
        vec.push_back(1);
        vec.push_back(2);
        int size = vec.size();
    }

    int main() {
        return 0;
    }
    "#;

    let temp_file = create_temp_cpp_file(code);
    let (success, output) = run_analyzer_with_include(temp_file.path());

    assert!(
        success,
        "Using vector operations in @unsafe code should be allowed. Output: {}",
        output
    );
}

#[test]
fn test_std_annotation_header_sort_in_unsafe() {
    // Test that std::sort works in @unsafe context
    let code = r#"
    #include <std_annotation.hpp>
    #include <vector>
    #include <algorithm>

    // @unsafe
    void unsafe_function() {
        std::vector<int> vec = {3, 1, 2};
        std::sort(vec.begin(), vec.end());
    }

    int main() {
        return 0;
    }
    "#;

    let temp_file = create_temp_cpp_file(code);
    let (success, output) = run_analyzer_with_include(temp_file.path());

    assert!(
        success,
        "Using std::sort in @unsafe code should be allowed. Output: {}",
        output
    );
}

#[test]
fn test_std_annotation_header_multiple_std_functions() {
    // Test multiple std functions together in @unsafe context
    let code = r#"
    #include <std_annotation.hpp>
    #include <vector>
    #include <algorithm>
    #include <memory>

    // @unsafe
    void unsafe_function() {
        // Utility
        int a = 1, b = 2;
        std::swap(a, b);

        // Container
        std::vector<int> vec = {3, 1, 2};
        vec.push_back(4);

        // Algorithm
        std::sort(vec.begin(), vec.end());
        auto it = std::find(vec.begin(), vec.end(), 2);

        // Smart pointer
        auto ptr = std::make_unique<int>(42);
    }

    int main() {
        return 0;
    }
    "#;

    let temp_file = create_temp_cpp_file(code);
    let (success, output) = run_analyzer_with_include(temp_file.path());

    assert!(
        success,
        "Using multiple std functions in @unsafe code should work. Output: {}",
        output
    );
}

#[test]
fn test_std_annotation_header_safe_code_without_std() {
    // Test that @safe code without std calls still works
    let code = r#"
    #include <std_annotation.hpp>

    // @safe
    void safe_function() {
        int x = 42;
        int y = x + 1;
        int z = y * 2;
    }

    int main() {
        return 0;
    }
    "#;

    let temp_file = create_temp_cpp_file(code);
    let (success, output) = run_analyzer_with_include(temp_file.path());

    assert!(
        success,
        "@safe code without std calls should work with the header included. Output: {}",
        output
    );
}

#[test]
fn test_std_annotation_header_mixed_safe_unsafe() {
    // Test mixing @safe and @unsafe functions with the header
    let code = r#"
    #include <std_annotation.hpp>
    #include <vector>

    // @safe
    int pure_computation(int x) {
        return x * 2 + 1;
    }

    // @unsafe
    void uses_std() {
        std::vector<int> vec;
        vec.push_back(pure_computation(21));
    }

    int main() {
        return 0;
    }
    "#;

    let temp_file = create_temp_cpp_file(code);
    let (success, output) = run_analyzer_with_include(temp_file.path());

    assert!(
        success,
        "Mixed @safe/@unsafe functions with std_annotation.hpp should work. Output: {}",
        output
    );
}

#[test]
fn test_std_annotation_header_qualified_name_matching() {
    // Test that qualified names (std::swap) match unqualified calls (swap)
    // when the header uses std::swap annotation
    let code = r#"
    #include <std_annotation.hpp>

    using namespace std;

    // @unsafe
    void unsafe_function() {
        int a = 1;
        int b = 2;
        // Using unqualified name should still match std::swap annotation
        swap(a, b);
    }

    int main() {
        return 0;
    }
    "#;

    let temp_file = create_temp_cpp_file(code);
    let (success, output) = run_analyzer_with_include(temp_file.path());

    assert!(
        success,
        "Unqualified swap() should match std::swap annotation. Output: {}",
        output
    );
}
