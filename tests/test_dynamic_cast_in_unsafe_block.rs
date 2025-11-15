/// Test dynamic_pointer_cast in @unsafe blocks with qualified vs unqualified names
///
/// The user might be reporting that unqualified names don't work even in @unsafe blocks

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

/// Test std::dynamic_pointer_cast in @unsafe block (should work)
#[test]
fn test_qualified_in_unsafe_block() {
    let temp_dir = TempDir::new().unwrap();
    let source_path = temp_dir.path().join("test.cpp");

    let code = r#"
#include <memory>

class Base { virtual ~Base() {} };
class Derived : public Base {};

// @safe
void test() {
    // @unsafe
    {
        std::shared_ptr<Base> base = std::make_shared<Derived>();
        auto derived = std::dynamic_pointer_cast<Derived>(base);
    }
}
"#;

    fs::write(&source_path, code).unwrap();
    let (_success, output) = run_analyzer(&source_path);

    println!("=== std::dynamic_pointer_cast in @unsafe block ===");
    println!("{}", output);

    // Should be fine - @unsafe allows calling undeclared functions
    // Check for actual violations (not just "no violations found")
    let has_violations = output.contains("violation(s):") ||
                        (output.contains("Found") && output.contains("violation") && !output.contains("no violations"));

    if has_violations {
        println!("❌ ERROR: Should work in @unsafe block");
        assert!(false, "std::dynamic_pointer_cast should work in @unsafe block. Output:\n{}", output);
    } else {
        println!("✅ Works as expected");
    }
}

/// Test dynamic_pointer_cast (unqualified) in @unsafe block
#[test]
fn test_unqualified_in_unsafe_block() {
    let temp_dir = TempDir::new().unwrap();
    let source_path = temp_dir.path().join("test.cpp");

    let code = r#"
#include <memory>

using namespace std;

class Base { virtual ~Base() {} };
class Derived : public Base {};

// @safe
void test() {
    // @unsafe
    {
        shared_ptr<Base> base = make_shared<Derived>();
        auto derived = dynamic_pointer_cast<Derived>(base);  // Unqualified
    }
}
"#;

    fs::write(&source_path, code).unwrap();
    let (_success, output) = run_analyzer(&source_path);

    println!("=== dynamic_pointer_cast (unqualified) in @unsafe block ===");
    println!("{}", output);

    // Should also be fine - @unsafe allows calling undeclared functions
    // Check for actual violations (not just "no violations found")
    let has_violations = output.contains("violation(s):") ||
                        (output.contains("Found") && output.contains("violation") && !output.contains("no violations"));

    if has_violations {
        println!("❌ BUG CONFIRMED: Unqualified doesn't work in @unsafe block");
        println!("This is the issue the user reported!");
        assert!(false, "dynamic_pointer_cast should work in @unsafe block. Output:\n{}", output);
    } else {
        println!("✅ Works as expected");
    }
}

/// Test that both work the same way in @unsafe blocks
#[test]
fn test_both_in_same_unsafe_block() {
    let temp_dir = TempDir::new().unwrap();
    let source_path = temp_dir.path().join("test.cpp");

    let code = r#"
#include <memory>

using namespace std;

class Base { virtual ~Base() {} };
class Derived : public Base {};

// @safe
void test() {
    shared_ptr<Base> base1 = make_shared<Derived>();
    shared_ptr<Base> base2 = make_shared<Derived>();

    // @unsafe
    {
        auto d1 = std::dynamic_pointer_cast<Derived>(base1);  // Qualified
        auto d2 = dynamic_pointer_cast<Derived>(base2);       // Unqualified
    }
}
"#;

    fs::write(&source_path, code).unwrap();
    let (_success, output) = run_analyzer(&source_path);

    println!("=== Both qualified and unqualified in same @unsafe block ===");
    println!("{}", output);

    // Count violations
    let violation_lines: Vec<&str> = output.lines()
        .filter(|line| line.contains("Calling") && line.contains("undeclared"))
        .collect();

    println!("Violations found: {}", violation_lines.len());
    for (i, line) in violation_lines.iter().enumerate() {
        println!("  {}. {}", i+1, line);
    }

    if violation_lines.is_empty() {
        println!("✅ Both work - no violations");
    } else if violation_lines.len() == 1 {
        println!("❌ BUG: One works but the other doesn't");
        if violation_lines[0].contains("std::dynamic_pointer_cast") {
            println!("   Qualified version has issue");
        } else {
            println!("   Unqualified version has issue");
        }
    } else {
        println!("Both have violations (might be expected if not in @unsafe function)");
    }
}
