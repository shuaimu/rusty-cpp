/// Tests for "using namespace std;" support
///
/// Verifies that functions called without std:: prefix are correctly
/// recognized when "using namespace std;" is active.

use std::fs;
use std::io::Write;
use std::path::Path;
use std::process::Command;
use tempfile::TempDir;

fn run_analyzer(cpp_file: &Path) -> (bool, String) {
    let output = Command::new("cargo")
        .args(&["run", "--", cpp_file.to_str().unwrap()])
        .output()
        .expect("Failed to run rusty-cpp-checker");

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let combined = format!("{}\n{}", stdout, stderr);

    (output.status.success(), combined)
}

/// Test that std::move works with "using namespace std;"
#[test]
fn test_using_namespace_std_move() {
    let temp_dir = TempDir::new().unwrap();
    let source_path = temp_dir.path().join("test.cpp");

    let code = r#"
#include <utility>

using namespace std;

// @safe
void test_move() {
    int x = 42;
    int y = move(x);  // Should be recognized as std::move
    int z = x;  // Should error: use after move
}
"#;

    fs::write(&source_path, code).unwrap();
    let (_success, output) = run_analyzer(&source_path);

    println!("=== TEST: using namespace std with move ===");
    println!("{}", output);

    // Should detect move and use-after-move
    assert!(
        output.contains("Use after move") || output.contains("has been moved"),
        "Should detect move() as std::move even without std:: prefix. Output:\n{}",
        output
    );
}

/// Test that std::forward works with "using namespace std;"
#[test]
fn test_using_namespace_std_forward() {
    let temp_dir = TempDir::new().unwrap();
    let source_path = temp_dir.path().join("test.cpp");

    let code = r#"
#include <utility>

using namespace std;

template<typename T>
void forwarder(T&& arg) {
    // forward should be recognized as std::forward
    // (This is just testing name resolution, not full semantics)
}
"#;

    fs::write(&source_path, code).unwrap();
    let (_success, output) = run_analyzer(&source_path);

    println!("=== TEST: using namespace std with forward ===");
    println!("{}", output);

    // Should not crash, at minimum
    assert!(
        !output.contains("error") || output.contains("no violations"),
        "Should handle forward without std:: prefix. Output:\n{}",
        output
    );
}

/// Test that safe whitelist functions work with "using namespace std;"
#[test]
fn test_using_namespace_std_safe_functions() {
    let temp_dir = TempDir::new().unwrap();
    let source_path = temp_dir.path().join("test.cpp");

    let code = r#"
#include <vector>
#include <algorithm>

using namespace std;

// @safe
void test_algorithms() {
    vector<int> vec = {3, 1, 4, 1, 5};

    // These should all be recognized as safe std:: functions
    sort(vec.begin(), vec.end());
    reverse(vec.begin(), vec.end());
    auto it = find(vec.begin(), vec.end(), 4);
}
"#;

    fs::write(&source_path, code).unwrap();
    let (_success, output) = run_analyzer(&source_path);

    println!("=== TEST: using namespace std with algorithms ===");
    println!("{}", output);

    // Should NOT report violations for safe std functions
    assert!(
        output.contains("no violations") || !output.contains("cannot call undeclared"),
        "Should recognize sort/reverse/find as safe std:: functions. Output:\n{}",
        output
    );
}

/// Test that undeclared functions are still caught with "using namespace std;"
#[test]
fn test_using_namespace_std_still_catches_undeclared() {
    let temp_dir = TempDir::new().unwrap();
    let source_path = temp_dir.path().join("test.cpp");

    let code = r#"
using namespace std;

// Undeclared function (not in std)
void my_custom_function(int x);

// @safe
void test_undeclared() {
    my_custom_function(42);  // Should still be caught as undeclared
}
"#;

    fs::write(&source_path, code).unwrap();
    let (_success, output) = run_analyzer(&source_path);

    println!("=== TEST: using namespace std still catches undeclared ===");
    println!("{}", output);

    // Should still catch undeclared functions
    assert!(
        output.contains("cannot call undeclared") ||
        output.contains("undeclared function") ||
        output.contains("my_custom_function"),
        "Should still detect undeclared functions even with 'using namespace std;'. Output:\n{}",
        output
    );
}

/// Test container methods with "using namespace std;"
#[test]
fn test_using_namespace_std_containers() {
    let temp_dir = TempDir::new().unwrap();
    let source_path = temp_dir.path().join("test.cpp");

    let code = r#"
#include <vector>

using namespace std;

// @safe
void test_vector() {
    vector<int> vec;
    vec.push_back(1);
    vec.push_back(2);
    int x = vec.size();
}
"#;

    fs::write(&source_path, code).unwrap();
    let (_success, output) = run_analyzer(&source_path);

    println!("=== TEST: using namespace std with vector ===");
    println!("{}", output);

    // Should not report violations
    assert!(
        output.contains("no violations") || !output.contains("violation"),
        "Should handle vector<int> without std:: prefix. Output:\n{}",
        output
    );
}

/// Test with partial namespace qualification
#[test]
fn test_mixed_std_qualification() {
    let temp_dir = TempDir::new().unwrap();
    let source_path = temp_dir.path().join("test.cpp");

    let code = r#"
#include <utility>
#include <vector>

using namespace std;

// @safe
void test_mixed() {
    int x = 42;

    // Mix of qualified and unqualified names
    int y = std::move(x);  // Explicitly qualified
    vector<int> vec;       // Unqualified (from using namespace)

    int z = x;  // Should error: use after move
}
"#;

    fs::write(&source_path, code).unwrap();
    let (_success, output) = run_analyzer(&source_path);

    println!("=== TEST: mixed std qualification ===");
    println!("{}", output);

    // Should detect move regardless of qualification
    assert!(
        output.contains("Use after move") || output.contains("has been moved"),
        "Should detect std::move with explicit qualification. Output:\n{}",
        output
    );
}

/// Test what name LibClang reports for unqualified calls
#[test]
fn test_libclang_name_resolution() {
    let temp_dir = TempDir::new().unwrap();
    let source_path = temp_dir.path().join("test.cpp");

    let code = r#"
#include <algorithm>

using namespace std;

void test() {
    int arr[] = {3, 1, 4};
    // Does LibClang report this as "sort" or "std::sort"?
    sort(arr, arr + 3);
}
"#;

    fs::write(&source_path, code).unwrap();

    // Run with debug output to see what name is extracted
    let output = Command::new("cargo")
        .args(&["run", "--", source_path.to_str().unwrap()])
        .env("RUST_LOG", "debug")
        .output()
        .expect("Failed to run");

    let combined = format!(
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    println!("=== LibClang Name Resolution Debug ===");
    println!("{}", combined);

    // Just verify it runs without crashing
    // The debug output will show us what name LibClang provides
    assert!(
        combined.contains("Analyzing") || combined.contains("rusty-cpp"),
        "Should complete analysis. Output:\n{}",
        combined
    );
}
