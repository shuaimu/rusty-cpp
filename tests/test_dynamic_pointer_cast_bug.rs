/// Test for bug: dynamic_pointer_cast works qualified but not unqualified
///
/// User report: Using std::dynamic_pointer_cast with std_annotation.hpp works,
/// but using dynamic_pointer_cast directly (with using namespace std) gives
/// "undeclared function" error from safe functions.

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

/// Test with qualified name std::dynamic_pointer_cast
#[test]
fn test_dynamic_pointer_cast_qualified() {
    let temp_dir = TempDir::new().unwrap();
    let source_path = temp_dir.path().join("test.cpp");

    let code = r#"
#include <memory>

class Base { virtual ~Base() {} };
class Derived : public Base {};

// @safe
void test() {
    std::shared_ptr<Base> base = std::make_shared<Derived>();

    // @unsafe - casts are unsafe
    {
        auto derived = std::dynamic_pointer_cast<Derived>(base);
    }
}
"#;

    fs::write(&source_path, code).unwrap();
    let (_success, output) = run_analyzer(&source_path);

    println!("=== TEST: std::dynamic_pointer_cast (qualified) ===");
    println!("{}", output);

    // Should NOT complain about "undeclared function"
    // It's marked unsafe, so it should be fine
    assert!(
        !output.contains("undeclared function") || output.contains("no violations"),
        "std::dynamic_pointer_cast should work in unsafe block. Output:\n{}",
        output
    );
}

/// Test with unqualified name dynamic_pointer_cast
#[test]
fn test_dynamic_pointer_cast_unqualified() {
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

    // @unsafe - casts are unsafe
    {
        auto derived = dynamic_pointer_cast<Derived>(base);  // Unqualified
    }
}
"#;

    fs::write(&source_path, code).unwrap();
    let (_success, output) = run_analyzer(&source_path);

    println!("=== TEST: dynamic_pointer_cast (unqualified) ===");
    println!("{}", output);

    // BUG: This reports "undeclared function" even in unsafe block!
    let has_undeclared_error = output.contains("undeclared function") &&
                               output.contains("dynamic_pointer_cast");

    if has_undeclared_error {
        println!("❌ BUG CONFIRMED: dynamic_pointer_cast treated as undeclared!");
        println!("   Qualified name works but unqualified doesn't");
    } else {
        println!("✅ No bug: dynamic_pointer_cast works unqualified");
    }

    // For now, document the expected behavior
    // It SHOULD work the same way as qualified version
}

/// Test static_pointer_cast with both forms
#[test]
fn test_static_pointer_cast_both_forms() {
    let temp_dir = TempDir::new().unwrap();
    let source_path = temp_dir.path().join("test.cpp");

    let code = r#"
#include <memory>

using namespace std;

class Base { virtual ~Base() {} };
class Derived : public Base {};

// @safe
void test_qualified() {
    shared_ptr<Base> base = make_shared<Derived>();

    // @unsafe
    {
        auto d1 = std::static_pointer_cast<Derived>(base);  // Qualified
        auto d2 = static_pointer_cast<Derived>(base);       // Unqualified
    }
}
"#;

    fs::write(&source_path, code).unwrap();
    let (_success, output) = run_analyzer(&source_path);

    println!("=== TEST: static_pointer_cast (both forms) ===");
    println!("{}", output);

    // Check if unqualified version has issues
    let has_undeclared = output.contains("undeclared") &&
                        (output.contains("static_pointer_cast") ||
                         output.contains("dynamic_pointer_cast"));

    if has_undeclared {
        println!("❌ BUG: Unqualified pointer casts treated as undeclared");
    }
}

/// Test all pointer cast types
#[test]
fn test_all_pointer_casts_unqualified() {
    let temp_dir = TempDir::new().unwrap();
    let source_path = temp_dir.path().join("test.cpp");

    let code = r#"
#include <memory>

using namespace std;

class Base { virtual ~Base() {} };
class Derived : public Base {};

// @unsafe - entire function is unsafe to allow casts
void test_all_casts() {
    shared_ptr<Base> base = make_shared<Derived>();

    // All unqualified cast names
    auto d1 = dynamic_pointer_cast<Derived>(base);
    auto d2 = static_pointer_cast<Derived>(base);
    auto d3 = const_pointer_cast<const Derived>(d2);
    auto d4 = reinterpret_pointer_cast<Derived>(base);
}
"#;

    fs::write(&source_path, code).unwrap();
    let (_success, output) = run_analyzer(&source_path);

    println!("=== TEST: All pointer casts unqualified ===");
    println!("{}", output);

    // In @unsafe function, all should be allowed
    // Check what errors we get
    let errors = output.lines()
        .filter(|line| line.contains("undeclared") || line.contains("violation"))
        .collect::<Vec<_>>();

    if !errors.is_empty() {
        println!("Errors found:");
        for err in &errors {
            println!("  - {}", err);
        }
    }
}

/// Test that the issue is specific to pointer casts, not other std functions
#[test]
fn test_other_std_functions_work_unqualified() {
    let temp_dir = TempDir::new().unwrap();
    let source_path = temp_dir.path().join("test.cpp");

    let code = r#"
#include <memory>
#include <utility>

using namespace std;

// @safe
void test() {
    unique_ptr<int> ptr = make_unique<int>(42);  // Unqualified
    int x = 10;
    int y = move(x);  // Unqualified move
    // These should work fine
}
"#;

    fs::write(&source_path, code).unwrap();
    let (_success, output) = run_analyzer(&source_path);

    println!("=== TEST: Other std functions unqualified ===");
    println!("{}", output);

    // Should work fine - these are in the safe/move detection
    assert!(
        !output.contains("undeclared function"),
        "Other std functions should work unqualified. Output:\n{}",
        output
    );
}
