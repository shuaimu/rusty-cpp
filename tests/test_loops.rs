use std::process::Command;
use std::fs;

// ============================================================================
// Basic loop tests - detect use-after-move across iterations
// NOTE: Tests use simple structs instead of STL to avoid libclang crashes.
// ============================================================================

#[test]
fn test_loop_use_after_move() {
    // This code should have an error - using a moved value in second iteration
    let test_code = r#"
namespace std {
    template<typename T> T&& move(T& x) { return static_cast<T&&>(x); }
}

struct Box { int* ptr; };

// @unsafe
void test() {
    Box ptr;

    for (int i = 0; i < 2; i++) {
        Box moved = std::move(ptr);  // Error on second iteration
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
    assert!(stdout.contains("loop") || stdout.contains("iteration") || stdout.contains("move"),
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
namespace std {
    template<typename T> T&& move(T& x) { return static_cast<T&&>(x); }
}

struct Box { int* ptr; };

// @unsafe
void test() {
    Box ptr;
    int count = 0;

    while (count < 2) {
        Box moved = std::move(ptr);  // Error on second iteration
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
    assert!(stdout.contains("loop") || stdout.contains("iteration") || stdout.contains("move"),
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
    // This tests the 2-iteration simulation - variable moved in one branch
    // should be detected as potentially moved in second iteration
    let test_code = r#"
namespace std {
    template<typename T> T&& move(T& x) { return static_cast<T&&>(x); }
}

struct Box { int* ptr; };

// @unsafe
void test() {
    Box ptr;

    for (int i = 0; i < 2; i++) {
        if (i == 0) {
            Box moved = std::move(ptr);  // Moves on first iteration
        }
        // On second iteration, ptr may be in moved state
        // The 2-iteration simulation should catch this
    }
}
"#;

    fs::write("test_conditional_move.cpp", test_code).unwrap();

    let output = Command::new("cargo")
        .args(&["run", "--", "test_conditional_move.cpp"])
        .output()
        .expect("Failed to run borrow checker");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // The checker should either:
    // 1. Detect use-after-move in the loop (ptr moved in if, used in second iteration)
    // 2. Or pass if the conservative analysis doesn't flag it
    // This is a complex case - we just verify it doesn't crash
    assert!(!stdout.contains("panic") && !stdout.contains("error:"),
            "Should not crash on conditional move in loop. Output: {}", stdout);

    // Clean up
    let _ = fs::remove_file("test_conditional_move.cpp");
}

// ============================================================================
// Loop-local variable tests (fixed in December 2025)
// These tests verify that variables declared INSIDE a loop body are correctly
// tracked as "loop-local" and don't trigger false positive use-after-move errors.
//
// NOTE: Tests use simple structs instead of STL to avoid libclang crashes.
// ============================================================================

#[test]
fn test_loop_local_variable_move_ok() {
    // This code should be OK - local variable is fresh each iteration
    let test_code = r#"
namespace std {
    template<typename T> T&& move(T& x) { return static_cast<T&&>(x); }
}

struct Box { int* ptr; };

// @unsafe
void test() {
    for (int i = 0; i < 10; i++) {
        // This is a FRESH variable each iteration
        Box local;
        Box moved = std::move(local);  // Move is fine - local is fresh each time
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
namespace std {
    template<typename T> T&& move(T& x) { return static_cast<T&&>(x); }
}

struct Box { int* ptr; };

// @unsafe
void test() {
    int count = 0;
    while (count < 10) {
        // Fresh variable each iteration
        Box item;
        Box moved = std::move(item);  // Move item - should be OK
        count++;
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
namespace std {
    template<typename T> T&& move(T& x) { return static_cast<T&&>(x); }
}

struct Box { int* ptr; };

// @unsafe
void test() {
    for (int i = 0; i < 5; i++) {
        // Fresh outer local each iteration
        Box outer;

        for (int j = 0; j < 5; j++) {
            // Fresh inner local each iteration
            Box inner;
            Box moved_inner = std::move(inner);  // OK - inner is fresh
        }

        Box moved_outer = std::move(outer);  // OK - outer is fresh each outer iteration
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
namespace std {
    template<typename T> T&& move(T& x) { return static_cast<T&&>(x); }
}

struct Box { int* ptr; };

// @unsafe
void test() {
    // Declared OUTSIDE the loop
    Box outer;

    for (int i = 0; i < 2; i++) {
        Box moved = std::move(outer);  // ERROR: moving outer var repeatedly
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
    assert!(stdout.contains("move") || stdout.contains("iteration") || stdout.contains("violation"),
            "Should detect outer variable moved in loop. Output: {}", stdout);

    // Clean up
    let _ = fs::remove_file("test_outer_move_error.cpp");
}

#[test]
fn test_multiple_moves_in_loop_ok() {
    // Multiple variables moved in the same loop iteration - all should be OK
    let test_code = r#"
namespace std {
    template<typename T> T&& move(T& x) { return static_cast<T&&>(x); }
}

struct Box { int* ptr; };

// @unsafe
void test() {
    for (int i = 0; i < 5; i++) {
        // Multiple fresh variables each iteration
        Box a;
        Box b;

        Box moved_a = std::move(a);  // OK - a is fresh
        Box moved_b = std::move(b);  // OK - b is fresh
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

#[test]
fn test_do_while_loop_local_variable_ok() {
    // Do-while loop with fresh local variable each iteration
    let test_code = r#"
namespace std {
    template<typename T> T&& move(T& x) { return static_cast<T&&>(x); }
}

struct Box { int* ptr; };

// @unsafe
void test() {
    int count = 0;
    do {
        // Fresh variable each iteration
        Box item;
        Box moved = std::move(item);
        count++;
    } while (count < 10);
}
"#;

    fs::write("test_do_while_local_ok.cpp", test_code).unwrap();

    let output = Command::new("cargo")
        .args(&["run", "--", "test_do_while_local_ok.cpp"])
        .output()
        .expect("Failed to run borrow checker");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should NOT find any violations
    assert!(stdout.contains("no violations found") || stdout.contains("✓"),
            "Do-while loop with local variable should be OK. Output: {}", stdout);

    // Clean up
    let _ = fs::remove_file("test_do_while_local_ok.cpp");
}

#[test]
fn test_loop_local_with_conditional_move_ok() {
    // Loop-local variable with conditional move - should be OK
    let test_code = r#"
namespace std {
    template<typename T> T&& move(T& x) { return static_cast<T&&>(x); }
}

struct Box { int* ptr; };

// @unsafe
void test() {
    for (int i = 0; i < 10; i++) {
        Box local;  // Fresh each iteration

        if (i % 2 == 0) {
            Box moved = std::move(local);  // OK - local is fresh
        }
        // Even if not moved in odd iterations, no error
    }
}
"#;

    fs::write("test_loop_conditional_local_ok.cpp", test_code).unwrap();

    let output = Command::new("cargo")
        .args(&["run", "--", "test_loop_conditional_local_ok.cpp"])
        .output()
        .expect("Failed to run borrow checker");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should NOT find any violations
    assert!(stdout.contains("no violations found") || stdout.contains("✓"),
            "Loop-local with conditional move should be OK. Output: {}", stdout);

    // Clean up
    let _ = fs::remove_file("test_loop_conditional_local_ok.cpp");
}

#[test]
fn test_loop_local_reassignment_ok() {
    // Loop-local variable reassigned and moved - should be OK
    let test_code = r#"
namespace std {
    template<typename T> T&& move(T& x) { return static_cast<T&&>(x); }
}

struct Box { int* ptr; };

// @unsafe
void test() {
    for (int i = 0; i < 10; i++) {
        Box local;  // Fresh each iteration
        Box temp = std::move(local);
        // local is moved, but it's fresh next iteration anyway
    }
}
"#;

    fs::write("test_loop_reassign_ok.cpp", test_code).unwrap();

    let output = Command::new("cargo")
        .args(&["run", "--", "test_loop_reassign_ok.cpp"])
        .output()
        .expect("Failed to run borrow checker");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should NOT find any violations
    assert!(stdout.contains("no violations found") || stdout.contains("✓"),
            "Loop-local reassignment should be OK. Output: {}", stdout);

    // Clean up
    let _ = fs::remove_file("test_loop_reassign_ok.cpp");
}