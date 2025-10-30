/// Test to verify if STL lifetime annotations are actually enforced
///
/// This tests whether the iterator invalidation detection actually works.

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

/// Test if iterator invalidation is detected with std::vector
#[test]
fn test_are_stl_annotations_enforced_qualified() {
    let temp_dir = TempDir::new().unwrap();
    let source_path = temp_dir.path().join("test.cpp");

    let code = r#"
#include <vector>

// @safe
void test() {
    std::vector<int> vec = {1, 2, 3};
    auto it = vec.begin();
    vec.push_back(4);  // Invalidates iterator
    int x = *it;       // Use of invalidated iterator
}
"#;

    fs::write(&source_path, code).unwrap();
    let (_success, output) = run_analyzer(&source_path);

    println!("=== STL Annotation Enforcement Test (qualified) ===");
    println!("{}", output);
    println!("=================================================");

    // Check if we get a borrow/lifetime violation (not a parse error!)
    // Must NOT be a fatal error or file not found error
    let has_borrow_violation = (output.contains("violation") || output.contains("borrow")) &&
                               !output.contains("Fatal error") &&
                               !output.contains("file not found");

    if has_borrow_violation {
        println!("✅ STL annotations ARE enforced (qualified names)");
    } else {
        println!("❌ STL annotations NOT currently enforced");
        println!("   (This is expected - STL lifetime annotations may be documentation only)");
    }
}

/// Test if iterator invalidation is detected with using namespace std
#[test]
fn test_are_stl_annotations_enforced_unqualified() {
    let temp_dir = TempDir::new().unwrap();
    let source_path = temp_dir.path().join("test.cpp");

    let code = r#"
#include <vector>

using namespace std;

// @safe
void test() {
    vector<int> vec = {1, 2, 3};  // Unqualified
    auto it = vec.begin();
    vec.push_back(4);  // Invalidates iterator
    int x = *it;       // Use of invalidated iterator
}
"#;

    fs::write(&source_path, code).unwrap();
    let (_success, output) = run_analyzer(&source_path);

    println!("=== STL Annotation Enforcement Test (unqualified) ===");
    println!("{}", output);
    println!("======================================================");

    let has_borrow_violation = (output.contains("violation") || output.contains("borrow")) &&
                               !output.contains("Fatal error") &&
                               !output.contains("file not found");

    if has_borrow_violation {
        println!("✅ STL annotations ARE enforced with 'using namespace std'");
    } else {
        println!("❌ STL annotations NOT currently enforced with 'using namespace std'");
        println!("   (This is expected - STL lifetime annotations may be documentation only)");
    }
}
