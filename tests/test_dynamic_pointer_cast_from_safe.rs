/// Test dynamic_pointer_cast called from @safe function without @unsafe block
///
/// This reproduces the user's scenario more accurately

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

/// Test calling std::dynamic_pointer_cast from @safe function (qualified)
#[test]
fn test_safe_calls_qualified_dynamic_pointer_cast() {
    let temp_dir = TempDir::new().unwrap();
    let source_path = temp_dir.path().join("test.cpp");

    let code = r#"
#include <memory>

class Base { virtual ~Base() {} };
class Derived : public Base {};

// @safe
void test() {
    std::shared_ptr<Base> base = std::make_shared<Derived>();
    auto derived = std::dynamic_pointer_cast<Derived>(base);  // Direct call from @safe
}
"#;

    fs::write(&source_path, code).unwrap();
    let (_success, output) = run_analyzer(&source_path);

    println!("=== TEST: @safe calls std::dynamic_pointer_cast (qualified) ===");
    println!("{}", output);

    // According to three-state system: @safe CAN call @unsafe
    // But dynamic_pointer_cast is UNDECLARED (not marked @safe or @unsafe)
    // So @safe CANNOT call undeclared

    let has_undeclared_error = output.contains("undeclared") ||
                               output.contains("must be explicitly marked");

    if has_undeclared_error {
        println!("Expected: @safe cannot call undeclared function");
    }
}

/// Test calling dynamic_pointer_cast from @safe function (unqualified)
#[test]
fn test_safe_calls_unqualified_dynamic_pointer_cast() {
    let temp_dir = TempDir::new().unwrap();
    let source_path = temp_dir.path().join("test.cpp");

    let code = r#"
#include <memory>

using namespace std;

class Base { virtual ~Base() {} };
class Derived : public Base {};

// @safe
void test() {
    shared_ptr<Base> base = make_shared<Derived>();
    auto derived = dynamic_pointer_cast<Derived>(base);  // Unqualified, direct call from @safe
}
"#;

    fs::write(&source_path, code).unwrap();
    let (_success, output) = run_analyzer(&source_path);

    println!("=== TEST: @safe calls dynamic_pointer_cast (unqualified) ===");
    println!("{}", output);

    let has_undeclared_error = output.contains("undeclared") ||
                               output.contains("must be explicitly marked");

    if has_undeclared_error {
        println!("Expected: @safe cannot call undeclared function");
    }

    // The question is: does the error message differ between qualified and unqualified?
}

/// Compare the error messages for qualified vs unqualified
#[test]
fn test_compare_error_messages() {
    let temp_dir = TempDir::new().unwrap();

    // Test 1: Qualified
    let qualified_path = temp_dir.path().join("qualified.cpp");
    let qualified_code = r#"
#include <memory>
class Base { virtual ~Base() {} };
class Derived : public Base {};

// @safe
void test() {
    std::shared_ptr<Base> base = std::make_shared<Derived>();
    auto d = std::dynamic_pointer_cast<Derived>(base);
}
"#;
    fs::write(&qualified_path, qualified_code).unwrap();
    let (_success1, output1) = run_analyzer(&qualified_path);

    // Test 2: Unqualified
    let unqualified_path = temp_dir.path().join("unqualified.cpp");
    let unqualified_code = r#"
#include <memory>
using namespace std;

class Base { virtual ~Base() {} };
class Derived : public Base {};

// @safe
void test() {
    shared_ptr<Base> base = make_shared<Derived>();
    auto d = dynamic_pointer_cast<Derived>(base);
}
"#;
    fs::write(&unqualified_path, unqualified_code).unwrap();
    let (_success2, output2) = run_analyzer(&unqualified_path);

    println!("=== QUALIFIED VERSION ===");
    println!("{}", output1);
    println!("\n=== UNQUALIFIED VERSION ===");
    println!("{}", output2);
    println!("\n=== COMPARISON ===");

    // Extract just the error messages
    let errors1: Vec<&str> = output1.lines()
        .filter(|line| line.contains("violation") || line.contains("undeclared") || line.contains("Calling"))
        .collect();

    let errors2: Vec<&str> = output2.lines()
        .filter(|line| line.contains("violation") || line.contains("undeclared") || line.contains("Calling"))
        .collect();

    println!("Qualified errors:");
    for err in &errors1 {
        println!("  {}", err);
    }

    println!("\nUnqualified errors:");
    for err in &errors2 {
        println!("  {}", err);
    }

    // Check if both report undeclared or if there's a difference
    let both_undeclared = errors1.iter().any(|e| e.contains("undeclared")) &&
                         errors2.iter().any(|e| e.contains("undeclared"));

    if both_undeclared {
        println!("\n✅ Both treated the same (both report undeclared)");
    } else if errors1.is_empty() && errors2.is_empty() {
        println!("\n✅ Both work (no errors)");
    } else {
        println!("\n❌ DIFFERENT BEHAVIOR:");
        println!("   Qualified: {} errors", errors1.len());
        println!("   Unqualified: {} errors", errors2.len());
    }
}
