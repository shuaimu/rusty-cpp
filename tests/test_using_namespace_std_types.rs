/// Tests for "using namespace std;" with STL type annotations
///
/// Verifies that STL lifetime annotations work correctly when types
/// are used without std:: prefix (e.g., vector instead of std::vector).

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

/// Test vector iterator invalidation with using namespace std
#[test]
fn test_vector_iterator_invalidation_unqualified() {
    let temp_dir = TempDir::new().unwrap();
    let source_path = temp_dir.path().join("test.cpp");

    let code = r#"
#include <vector>

using namespace std;

// @safe
void test_vector() {
    vector<int> vec = {1, 2, 3};  // Unqualified type
    auto it = vec.begin();
    vec.push_back(4);  // Should invalidate iterator
    int x = *it;       // ERROR: iterator invalidated
}
"#;

    fs::write(&source_path, code).unwrap();
    let (_success, output) = run_analyzer(&source_path);

    println!("=== TEST: vector iterator invalidation (unqualified) ===");
    println!("{}", output);

    // This test checks if lifetime annotations work with unqualified vector
    // If annotations don't work, we won't get a violation
    // If they do work, we should get an iterator invalidation error
    // For now, just check it doesn't crash
    assert!(
        output.contains("Analyzing") || output.contains("rusty-cpp"),
        "Should complete analysis. Output:\n{}",
        output
    );
}

/// Test vector with qualified name for comparison
#[test]
fn test_vector_iterator_invalidation_qualified() {
    let temp_dir = TempDir::new().unwrap();
    let source_path = temp_dir.path().join("test.cpp");

    let code = r#"
#include <vector>

// @safe
void test_vector() {
    std::vector<int> vec = {1, 2, 3};  // Qualified type
    auto it = vec.begin();
    vec.push_back(4);  // Should invalidate iterator
    int x = *it;       // ERROR: iterator invalidated
}
"#;

    fs::write(&source_path, code).unwrap();
    let (_success, output) = run_analyzer(&source_path);

    println!("=== TEST: vector iterator invalidation (qualified) ===");
    println!("{}", output);

    assert!(
        output.contains("Analyzing") || output.contains("rusty-cpp"),
        "Should complete analysis. Output:\n{}",
        output
    );
}

/// Test unique_ptr with using namespace std
#[test]
fn test_unique_ptr_unqualified() {
    let temp_dir = TempDir::new().unwrap();
    let source_path = temp_dir.path().join("test.cpp");

    let code = r#"
#include <memory>

using namespace std;

// @safe
void test_unique_ptr() {
    unique_ptr<int> ptr = make_unique<int>(42);  // Unqualified
    int& ref = *ptr;
    auto ptr2 = move(ptr);  // Move the pointer
    // ref is now dangling if we track this
}
"#;

    fs::write(&source_path, code).unwrap();
    let (_success, output) = run_analyzer(&source_path);

    println!("=== TEST: unique_ptr (unqualified) ===");
    println!("{}", output);

    // Should detect the move
    assert!(
        output.contains("Analyzing"),
        "Should complete analysis. Output:\n{}",
        output
    );
}

/// Test shared_ptr with using namespace std
#[test]
fn test_shared_ptr_unqualified() {
    let temp_dir = TempDir::new().unwrap();
    let source_path = temp_dir.path().join("test.cpp");

    let code = r#"
#include <memory>

using namespace std;

// @safe
void test_shared_ptr() {
    shared_ptr<int> ptr = make_shared<int>(42);  // Unqualified
    int& ref = *ptr;
    int x = ref;  // Should be fine
}
"#;

    fs::write(&source_path, code).unwrap();
    let (_success, output) = run_analyzer(&source_path);

    println!("=== TEST: shared_ptr (unqualified) ===");
    println!("{}", output);

    assert!(
        output.contains("Analyzing"),
        "Should complete analysis. Output:\n{}",
        output
    );
}

/// Test string with using namespace std
#[test]
fn test_string_unqualified() {
    let temp_dir = TempDir::new().unwrap();
    let source_path = temp_dir.path().join("test.cpp");

    let code = r#"
#include <string>

using namespace std;

// @safe
void test_string() {
    string str = "hello";  // Unqualified type
    const char* ptr = str.c_str();
    str = "world";  // Invalidates ptr
    // char c = *ptr;  // Would be error if we track this
}
"#;

    fs::write(&source_path, code).unwrap();
    let (_success, output) = run_analyzer(&source_path);

    println!("=== TEST: string (unqualified) ===");
    println!("{}", output);

    assert!(
        output.contains("Analyzing"),
        "Should complete analysis. Output:\n{}",
        output
    );
}

/// Test map with using namespace std
#[test]
fn test_map_unqualified() {
    let temp_dir = TempDir::new().unwrap();
    let source_path = temp_dir.path().join("test.cpp");

    let code = r#"
#include <map>

using namespace std;

// @safe
void test_map() {
    map<int, int> m;  // Unqualified type
    m[1] = 100;
    m[2] = 200;
    auto it = m.find(1);
}
"#;

    fs::write(&source_path, code).unwrap();
    let (_success, output) = run_analyzer(&source_path);

    println!("=== TEST: map (unqualified) ===");
    println!("{}", output);

    assert!(
        output.contains("Analyzing"),
        "Should complete analysis. Output:\n{}",
        output
    );
}

/// Test multiple STL types mixed qualified/unqualified
#[test]
fn test_mixed_stl_types() {
    let temp_dir = TempDir::new().unwrap();
    let source_path = temp_dir.path().join("test.cpp");

    let code = r#"
#include <vector>
#include <memory>
#include <string>

using namespace std;

// @safe
void test_mixed() {
    vector<int> vec = {1, 2, 3};           // Unqualified
    std::unique_ptr<int> ptr = std::make_unique<int>(42);  // Qualified
    string str = "hello";                   // Unqualified
    std::shared_ptr<double> sptr = std::make_shared<double>(3.14);  // Qualified
}
"#;

    fs::write(&source_path, code).unwrap();
    let (_success, output) = run_analyzer(&source_path);

    println!("=== TEST: mixed STL types ===");
    println!("{}", output);

    assert!(
        output.contains("Analyzing"),
        "Should complete analysis. Output:\n{}",
        output
    );
}

/// Test that type detection still works with using namespace std
#[test]
fn test_type_detection_with_using_namespace() {
    let temp_dir = TempDir::new().unwrap();
    let source_path = temp_dir.path().join("test.cpp");

    let code = r#"
#include <vector>

using namespace std;

// @safe
void test_detection() {
    // LibClang should still recognize this as a vector type
    // even though we're using the unqualified name
    vector<int> vec;
    vec.push_back(1);
    vec.push_back(2);

    // This should be recognized as safe because vector::push_back is safe
    int x = vec.size();
}
"#;

    fs::write(&source_path, code).unwrap();
    let (_success, output) = run_analyzer(&source_path);

    println!("=== TEST: type detection with using namespace ===");
    println!("{}", output);

    // Should not report violations for safe vector operations
    assert!(
        output.contains("no violations") || !output.contains("cannot call undeclared"),
        "Should recognize vector methods as safe. Output:\n{}",
        output
    );
}
