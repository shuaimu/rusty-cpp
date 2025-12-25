use std::process::Command;
use std::fs;

#[test]
fn test_loop_use_after_move() {
    // This code should have an error - using a moved value in second iteration
    let test_code = r#"
#include <memory>

// @safe
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
fn test_while_loop_use_after_move() {
    // Test with while loop
    let test_code = r#"
#include <memory>

// @safe
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
fn test_loop_conditional_move() {
    // Test move that only happens sometimes in loop
    let test_code = r#"
#include <memory>

// @safe
void test() {
    std::unique_ptr<int> ptr(new int(42));

    for (int i = 0; i < 2; i++) {
        if (i == 0) {
            auto moved = std::move(ptr);  // Moves on first iteration
        } else {
            // ptr is already moved on second iteration
            auto value = *ptr;  // Error: use after move (requires dereference tracking)
        }
    }
}
"#;
    
    fs::write("test_conditional_move.cpp", test_code).unwrap();
    
    let output = Command::new("cargo")
        .args(&["run", "--", "test_conditional_move.cpp"])
        .output()
        .expect("Failed to run borrow checker");
    
    let stdout = String::from_utf8_lossy(&output.stdout);
    
    // Should find use-after-move
    assert!(stdout.contains("moved") || stdout.contains("violation"),
            "Should detect conditional use-after-move in loop. Output: {}", stdout);

    // Clean up
    let _ = fs::remove_file("test_conditional_move.cpp");
}

// ============================================================================
// Loop-local variable tests (fixed in December 2025)
// ============================================================================

#[test]
fn test_loop_local_variable_move_ok() {
    // This code should be OK - local variable is fresh each iteration
    let test_code = r#"
#include <memory>

void consume(std::unique_ptr<int> p);

// @safe
void test() {
    // @unsafe
    {
        for (int i = 0; i < 10; i++) {
            // This is a FRESH variable each iteration
            std::unique_ptr<int> local = std::make_unique<int>(i);
            consume(std::move(local));  // Move is fine - local is fresh each time
        }
    }
}
"#;

    fs::write("test_loop_local_ok.cpp", test_code).unwrap();

    let output = Command::new("cargo")
        .args(&["run", "--", "test_loop_local_ok.cpp"])
        .output()
        .expect("Failed to run borrow checker");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should NOT find any violations - loop-local variables are fresh each iteration
    assert!(stdout.contains("no violations found") || stdout.contains("✓"),
            "Loop-local variable move should be OK. Output: {}", stdout);

    // Clean up
    let _ = fs::remove_file("test_loop_local_ok.cpp");
}

#[test]
fn test_while_loop_local_variable_ok() {
    // While loop with fresh local variable each iteration
    let test_code = r#"
#include <memory>
#include <list>

void process(std::unique_ptr<int> req);

// @safe
void test(std::list<std::unique_ptr<int>>& items) {
    // @unsafe
    {
        while (!items.empty()) {
            // Fresh variable each iteration
            std::unique_ptr<int> item = std::move(items.front());
            items.pop_front();
            process(std::move(item));  // Move item - should be OK
        }
    }
}
"#;

    fs::write("test_while_local_ok.cpp", test_code).unwrap();

    let output = Command::new("cargo")
        .args(&["run", "--", "test_while_local_ok.cpp"])
        .output()
        .expect("Failed to run borrow checker");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should NOT find any violations
    assert!(stdout.contains("no violations found") || stdout.contains("✓"),
            "While loop with local variable should be OK. Output: {}", stdout);

    // Clean up
    let _ = fs::remove_file("test_while_local_ok.cpp");
}

#[test]
fn test_nested_loops_local_variables_ok() {
    // Nested loops with fresh local variables - should be OK
    let test_code = r#"
#include <memory>

void consume(std::unique_ptr<int> p);

// @safe
void test() {
    // @unsafe
    {
        for (int i = 0; i < 5; i++) {
            // Fresh outer local each iteration
            std::unique_ptr<int> outer = std::make_unique<int>(i);

            for (int j = 0; j < 5; j++) {
                // Fresh inner local each iteration
                std::unique_ptr<int> inner = std::make_unique<int>(j);
                consume(std::move(inner));  // OK - inner is fresh
            }

            consume(std::move(outer));  // OK - outer is fresh each outer iteration
        }
    }
}
"#;

    fs::write("test_nested_local_ok.cpp", test_code).unwrap();

    let output = Command::new("cargo")
        .args(&["run", "--", "test_nested_local_ok.cpp"])
        .output()
        .expect("Failed to run borrow checker");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should NOT find any violations
    assert!(stdout.contains("no violations found") || stdout.contains("✓"),
            "Nested loops with local variables should be OK. Output: {}", stdout);

    // Clean up
    let _ = fs::remove_file("test_nested_local_ok.cpp");
}

#[test]
fn test_outer_variable_moved_in_loop_error() {
    // Outer variable moved in loop - SHOULD still be detected as error
    let test_code = r#"
#include <memory>

void consume(std::unique_ptr<int> p);

// @safe
void test() {
    // @unsafe
    {
        // Declared OUTSIDE the loop
        std::unique_ptr<int> outer = std::make_unique<int>(42);

        for (int i = 0; i < 2; i++) {
            consume(std::move(outer));  // ERROR: moving outer var repeatedly
        }
    }
}
"#;

    fs::write("test_outer_move_error.cpp", test_code).unwrap();

    let output = Command::new("cargo")
        .args(&["run", "--", "test_outer_move_error.cpp"])
        .output()
        .expect("Failed to run borrow checker");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // SHOULD find violation - outer variable is moved in loop
    assert!(stdout.contains("moved") || stdout.contains("iteration") || stdout.contains("violation"),
            "Should detect outer variable moved in loop. Output: {}", stdout);

    // Clean up
    let _ = fs::remove_file("test_outer_move_error.cpp");
}

#[test]
fn test_multiple_moves_in_loop_ok() {
    // Multiple variables moved in the same loop iteration - all should be OK
    let test_code = r#"
#include <memory>

void consume1(std::unique_ptr<int> p);
void consume2(std::unique_ptr<int> p);

// @safe
void test() {
    // @unsafe
    {
        for (int i = 0; i < 5; i++) {
            // Multiple fresh variables each iteration
            std::unique_ptr<int> a = std::make_unique<int>(i);
            std::unique_ptr<int> b = std::make_unique<int>(i * 2);

            consume1(std::move(a));  // OK - a is fresh
            consume2(std::move(b));  // OK - b is fresh
        }
    }
}
"#;

    fs::write("test_multi_move_loop_ok.cpp", test_code).unwrap();

    let output = Command::new("cargo")
        .args(&["run", "--", "test_multi_move_loop_ok.cpp"])
        .output()
        .expect("Failed to run borrow checker");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should NOT find any violations
    assert!(stdout.contains("no violations found") || stdout.contains("✓"),
            "Multiple moves in loop should be OK. Output: {}", stdout);

    // Clean up
    let _ = fs::remove_file("test_multi_move_loop_ok.cpp");
}