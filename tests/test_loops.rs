use std::process::Command;
use std::fs;

#[test]
#[ignore] // Requires std::unique_ptr support
fn test_loop_use_after_move() {
    // This code should have an error - using a moved value in second iteration
    let test_code = r#"
#include <memory>

void test() {
    std::unique_ptr<int> ptr(new int(42));
    
    for (int i = 0; i < 2; i++) {
        auto moved = std::move(ptr);  // Error on second iteration
    }
}
"#;
    
    fs::write("test_loop_move.cpp", test_code).unwrap();
    
    let output = Command::new("cargo")
        .args(&["run", "--", "test_loop_move.cpp"])
        .env("Z3_SYS_Z3_HEADER", "/opt/homebrew/include/z3.h")
        .output()
        .expect("Failed to run borrow checker");
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    
    // Should find the use-after-move error in loop
    assert!(stdout.contains("loop") || stdout.contains("iteration"),
            "Should detect use-after-move in loop. stdout: {}, stderr: {}", stdout, stderr);
    
    // Clean up
    let _ = fs::remove_file("test_loop_move.cpp");
}

#[test]
fn test_loop_without_move_ok() {
    // This code should be OK - no moves in loop
    let test_code = r#"
void test() {
    int value = 42;
    
    for (int i = 0; i < 2; i++) {
        int& ref = value;
        ref = i;
    }
}
"#;
    
    fs::write("test_loop_ok.cpp", test_code).unwrap();
    
    let output = Command::new("cargo")
        .args(&["run", "--", "test_loop_ok.cpp"])
        .env("Z3_SYS_Z3_HEADER", "/opt/homebrew/include/z3.h")
        .output()
        .expect("Failed to run borrow checker");
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    
    // Should NOT find any violations
    assert!(stdout.contains("no violations found") || 
            stdout.contains("✓") ||
            !stdout.contains("violation"),
            "Loop without moves should be OK. Output: {}", stdout);
    
    // Clean up
    let _ = fs::remove_file("test_loop_ok.cpp");
}

#[test]
#[ignore] // Requires std::unique_ptr support
fn test_while_loop_use_after_move() {
    // Test with while loop
    let test_code = r#"
#include <memory>

void test() {
    std::unique_ptr<int> ptr(new int(42));
    int count = 0;
    
    while (count < 2) {
        auto moved = std::move(ptr);  // Error on second iteration
        count++;
    }
}
"#;
    
    fs::write("test_while_move.cpp", test_code).unwrap();
    
    let output = Command::new("cargo")
        .args(&["run", "--", "test_while_move.cpp"])
        .env("Z3_SYS_Z3_HEADER", "/opt/homebrew/include/z3.h")
        .output()
        .expect("Failed to run borrow checker");
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    
    // Should find the use-after-move error in loop
    assert!(stdout.contains("loop") || stdout.contains("iteration") || stdout.contains("moved"),
            "Should detect use-after-move in while loop. Output: {}", stdout);
    
    // Clean up
    let _ = fs::remove_file("test_while_move.cpp");
}

#[test]
fn test_nested_loop_borrows() {
    // Test nested loops with borrows
    let test_code = r#"// @safe
void test() {
    int value = 42;
    
    for (int i = 0; i < 2; i++) {
        int& ref1 = value;
        for (int j = 0; j < 2; j++) {
            const int& ref2 = value;  // Should error - mutable borrow exists
        }
    }
}
"#;
    
    fs::write("test_nested_loops.cpp", test_code).unwrap();
    
    let output = Command::new("cargo")
        .args(&["run", "--", "test_nested_loops.cpp"])
        .env("Z3_SYS_Z3_HEADER", "/opt/homebrew/include/z3.h")
        .output()
        .expect("Failed to run borrow checker");
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    
    // Should find borrow checking violation
    assert!(stdout.contains("already mutably borrowed") || stdout.contains("violation"),
            "Should detect borrow conflicts in nested loops. Output: {}", stdout);
    
    // Clean up
    let _ = fs::remove_file("test_nested_loops.cpp");
}

#[test]
#[ignore] // Requires std::unique_ptr support
fn test_loop_conditional_move() {
    // Test move that only happens sometimes in loop
    let test_code = r#"
#include <memory>

void test() {
    std::unique_ptr<int> ptr(new int(42));
    
    for (int i = 0; i < 2; i++) {
        if (i == 0) {
            auto moved = std::move(ptr);  // Moves on first iteration
        } else {
            // ptr is already moved on second iteration
            auto value = *ptr;  // Error: use after move
        }
    }
}
"#;
    
    fs::write("test_conditional_move.cpp", test_code).unwrap();
    
    let output = Command::new("cargo")
        .args(&["run", "--", "test_conditional_move.cpp"])
        .env("Z3_SYS_Z3_HEADER", "/opt/homebrew/include/z3.h")
        .output()
        .expect("Failed to run borrow checker");
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    
    // Should find use-after-move
    assert!(stdout.contains("moved") || stdout.contains("violation"),
            "Should detect conditional use-after-move in loop. Output: {}", stdout);
    
    // Clean up
    let _ = fs::remove_file("test_conditional_move.cpp");
}