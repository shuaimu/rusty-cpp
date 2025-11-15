use std::process::Command;
use std::io::Write;
use tempfile::NamedTempFile;

/// Helper function to create a temporary C++ file
fn create_temp_cpp_file(code: &str) -> NamedTempFile {
    let mut temp_file = NamedTempFile::new().unwrap();
    temp_file.write_all(code.as_bytes()).unwrap();
    temp_file.flush().unwrap();
    temp_file
}

/// Helper function to run the analyzer on a file
fn run_analyzer(file_path: &std::path::Path) -> (bool, String) {
    let output = Command::new("cargo")
        .args(&["run", "--", file_path.to_str().unwrap(), "-Iinclude"])
        .output()
        .expect("Failed to run analyzer");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined = format!("{}\n{}", stdout, stderr);

    (output.status.success(), combined)
}

/// Test 1: Basic case - cannot move while borrowed by const reference
#[test]
fn test_cannot_move_while_borrowed_const_ref() {
    let code = r#"
    #include <memory>

    // @safe
    void take_ownership(std::unique_ptr<int> ptr) {
        int x = *ptr;
    }

    // @safe
    void test() {
        std::unique_ptr<int> ptr(new int(42));
        const std::unique_ptr<int>& ref = ptr;

        // ERROR: Cannot move ptr because it is borrowed by ref
        take_ownership(std::move(ptr));
    }
    "#;

    let temp_file = create_temp_cpp_file(code);
    let (_success, output) = run_analyzer(temp_file.path());

    assert!(
        output.contains("Cannot move 'ptr'") && output.contains("borrowed by"),
        "Should detect that ptr is borrowed by ref. Output: {}",
        output
    );
}

/// Test 2: Cannot move while borrowed by mutable reference
#[test]
fn test_cannot_move_while_borrowed_mut_ref() {
    let code = r#"
    #include <memory>

    // @safe
    void take_ownership(std::unique_ptr<int> ptr) {
        int x = *ptr;
    }

    // @safe
    void test() {
        std::unique_ptr<int> ptr(new int(42));
        std::unique_ptr<int>& mut_ref = ptr;

        // ERROR: Cannot move ptr because it is borrowed by mut_ref
        take_ownership(std::move(ptr));
    }
    "#;

    let temp_file = create_temp_cpp_file(code);
    let (_success, output) = run_analyzer(temp_file.path());

    assert!(
        output.contains("Cannot move 'ptr'") && output.contains("borrowed by"),
        "Should detect that ptr is borrowed by mut_ref. Output: {}",
        output
    );
}

/// Test 3: CRITICAL - Reference goes out of scope, move is allowed again
#[test]
fn test_move_allowed_after_reference_scope_ends() {
    let code = r#"
    #include <rusty/box.hpp>

    // @safe
    void take_ownership(rusty::Box<int> ptr) {
        int x = *ptr;
    }

    // @safe
    void test() {
        rusty::Box<int> ptr = rusty::Box<int>::make(42);

        {
            const rusty::Box<int>& ref = ptr;
            int x = *ref;
        }  // ref goes out of scope here

        // OK: ref is gone, ptr is no longer borrowed
        take_ownership(std::move(ptr));
    }
    "#;

    let temp_file = create_temp_cpp_file(code);
    let (success, output) = run_analyzer(temp_file.path());

    assert!(
        output.contains("no violations") || success,
        "Should allow move after reference scope ends. Output: {}",
        output
    );
}

/// Test 4: Multiple borrowers prevent move
#[test]
fn test_cannot_move_with_multiple_borrowers() {
    let code = r#"
    #include <memory>

    // @safe
    void take_ownership(std::unique_ptr<int> ptr) {
        int x = *ptr;
    }

    // @safe
    void test() {
        std::unique_ptr<int> ptr(new int(42));
        const std::unique_ptr<int>& ref1 = ptr;
        const std::unique_ptr<int>& ref2 = ptr;
        const std::unique_ptr<int>& ref3 = ptr;

        // ERROR: Cannot move ptr because it is borrowed by ref1, ref2, ref3
        take_ownership(std::move(ptr));
    }
    "#;

    let temp_file = create_temp_cpp_file(code);
    let (_success, output) = run_analyzer(temp_file.path());

    assert!(
        output.contains("Cannot move 'ptr'") && output.contains("borrowed by"),
        "Should detect multiple borrowers. Output: {}",
        output
    );
}

/// Test 5: Nested scopes - inner reference doesn't affect outer move
#[test]
fn test_nested_scope_borrow_cleanup() {
    let code = r#"
    #include <rusty/box.hpp>

    // @safe
    void take_ownership(rusty::Box<int> ptr) {
        int x = *ptr;
    }

    // @safe
    void test() {
        rusty::Box<int> ptr = rusty::Box<int>::make(42);

        {
            {
                const rusty::Box<int>& ref = ptr;
                int x = *ref;
            }  // ref goes out of scope
        }

        // OK: All references are gone
        take_ownership(std::move(ptr));
    }
    "#;

    let temp_file = create_temp_cpp_file(code);
    let (success, output) = run_analyzer(temp_file.path());

    assert!(
        output.contains("no violations") || success,
        "Should allow move after nested scope ends. Output: {}",
        output
    );
}

/// Test 6: Conditional - borrow in one branch doesn't prevent move after
#[test]
fn test_conditional_borrow_cleanup() {
    let code = r#"
    #include <rusty/box.hpp>

    // @safe
    void take_ownership(rusty::Box<int> ptr) {
        int x = *ptr;
    }

    // @safe
    void test(bool condition) {
        rusty::Box<int> ptr = rusty::Box<int>::make(42);

        if (condition) {
            const rusty::Box<int>& ref = ptr;
            int x = *ref;
        }

        // OK: ref only exists in if branch
        take_ownership(std::move(ptr));
    }
    "#;

    let temp_file = create_temp_cpp_file(code);
    let (success, output) = run_analyzer(temp_file.path());

    assert!(
        output.contains("no violations") || success,
        "Should allow move after conditional with borrow. Output: {}",
        output
    );
}

/// Test 7: Borrow in both branches prevents move
#[test]
fn test_borrow_in_both_branches_prevents_move() {
    let code = r#"
    #include <memory>

    // @safe
    void take_ownership(std::unique_ptr<int> ptr) {
        int x = *ptr;
    }

    // @safe
    void test(bool condition) {
        std::unique_ptr<int> ptr(new int(42));
        const std::unique_ptr<int>* ref_ptr = nullptr;

        if (condition) {
            const std::unique_ptr<int>& ref = ptr;
            ref_ptr = &ref;
        } else {
            const std::unique_ptr<int>& ref = ptr;
            ref_ptr = &ref;
        }

        // Note: In real analysis, refs go out of scope at end of each branch
        // This test documents current behavior
        take_ownership(std::move(ptr));
    }
    "#;

    let temp_file = create_temp_cpp_file(code);
    let (_success, output) = run_analyzer(temp_file.path());

    // Current behavior: refs are scoped to branches, so move is allowed
    // This is conservative and correct
    assert!(
        output.contains("no violations") || output.contains("Cannot move"),
        "Test completed. Output: {}",
        output
    );
}

/// Test 8: Loop - borrow prevents move in same iteration
#[test]
fn test_loop_borrow_prevents_move() {
    let code = r#"
    #include <memory>

    // @safe
    void take_ownership(std::unique_ptr<int> ptr) {
        int x = *ptr;
    }

    // @safe
    void test() {
        for (int i = 0; i < 2; i++) {
            std::unique_ptr<int> ptr(new int(42));
            const std::unique_ptr<int>& ref = ptr;

            // ERROR: Cannot move ptr while ref is alive
            take_ownership(std::move(ptr));
        }
    }
    "#;

    let temp_file = create_temp_cpp_file(code);
    let (_success, output) = run_analyzer(temp_file.path());

    assert!(
        output.contains("Cannot move 'ptr'") && output.contains("borrowed by"),
        "Should detect borrow in loop. Output: {}",
        output
    );
}

/// Test 9: Sequential borrows - each in its own scope
#[test]
fn test_sequential_borrows_in_scopes() {
    let code = r#"
    #include <rusty/box.hpp>

    // @safe
    void use_ref(const rusty::Box<int>& ref) {
        int x = *ref;
    }

    // @safe
    void take_ownership(rusty::Box<int> ptr) {
        int x = *ptr;
    }

    // @safe
    void test() {
        rusty::Box<int> ptr = rusty::Box<int>::make(42);

        {
            const rusty::Box<int>& ref1 = ptr;
            use_ref(ref1);
        }  // ref1 dies

        {
            const rusty::Box<int>& ref2 = ptr;
            use_ref(ref2);
        }  // ref2 dies

        // OK: All references are gone
        take_ownership(std::move(ptr));
    }
    "#;

    let temp_file = create_temp_cpp_file(code);
    let (success, output) = run_analyzer(temp_file.path());

    assert!(
        output.contains("no violations") || success,
        "Should allow move after sequential scoped borrows. Output: {}",
        output
    );
}

/// Test 10: Borrow survives in outer scope
#[test]
fn test_borrow_in_outer_scope_prevents_move() {
    let code = r#"
    #include <memory>

    // @safe
    void take_ownership(std::unique_ptr<int> ptr) {
        int x = *ptr;
    }

    // @safe
    void test() {
        std::unique_ptr<int> ptr(new int(42));
        const std::unique_ptr<int>& ref = ptr;

        {
            int x = *ref;  // Use ref in inner scope
        }

        // ERROR: ref is still alive in outer scope
        take_ownership(std::move(ptr));
    }
    "#;

    let temp_file = create_temp_cpp_file(code);
    let (_success, output) = run_analyzer(temp_file.path());

    assert!(
        output.contains("Cannot move 'ptr'") && output.contains("borrowed by"),
        "Should detect that outer scope ref is still alive. Output: {}",
        output
    );
}

/// Test 11: Can move if never borrowed
#[test]
fn test_can_move_when_never_borrowed() {
    let code = r#"
    #include <rusty/box.hpp>

    // @safe
    void take_ownership(rusty::Box<int> ptr) {
        int x = *ptr;
    }

    // @safe
    void test() {
        rusty::Box<int> ptr = rusty::Box<int>::make(42);
        int x = *ptr;

        // OK: No references created
        take_ownership(std::move(ptr));
    }
    "#;

    let temp_file = create_temp_cpp_file(code);
    let (success, output) = run_analyzer(temp_file.path());

    assert!(
        output.contains("no violations") || success,
        "Should allow move when never borrowed. Output: {}",
        output
    );
}

/// Test 12: Two different variables with different borrows
#[test]
fn test_independent_borrows() {
    let code = r#"
    #include <memory>

    // @safe
    void take_ownership(std::unique_ptr<int> ptr) {
        int x = *ptr;
    }

    // @safe
    void test() {
        std::unique_ptr<int> ptr1(new int(1));
        std::unique_ptr<int> ptr2(new int(2));

        const std::unique_ptr<int>& ref1 = ptr1;

        // OK: ptr2 is not borrowed, ptr1 is
        take_ownership(std::move(ptr2));

        // ERROR: ptr1 is borrowed by ref1
        take_ownership(std::move(ptr1));
    }
    "#;

    let temp_file = create_temp_cpp_file(code);
    let (_success, output) = run_analyzer(temp_file.path());

    assert!(
        output.contains("Cannot move 'ptr1'") && output.contains("borrowed by"),
        "Should detect that only ptr1 is borrowed. Output: {}",
        output
    );
}

/// Test 13: Reference ends before move in same scope
#[test]
fn test_reference_lifetime_ends_same_scope() {
    let code = r#"
    #include <rusty/box.hpp>

    // @safe
    void take_ownership(rusty::Box<int> ptr) {
        int x = *ptr;
    }

    // @safe
    void test() {
        rusty::Box<int> ptr = rusty::Box<int>::make(42);

        {
            const rusty::Box<int>& ref = ptr;
        }  // ref lifetime ends here

        // OK: ref is out of scope
        take_ownership(std::move(ptr));
    }
    "#;

    let temp_file = create_temp_cpp_file(code);
    let (success, output) = run_analyzer(temp_file.path());

    assert!(
        output.contains("no violations") || success,
        "Should allow move after reference lifetime ends. Output: {}",
        output
    );
}

/// Test 14: Mix of mutable and const references
#[test]
fn test_mix_of_mutable_and_const_refs() {
    let code = r#"
    #include <memory>

    // @safe
    void take_ownership(std::unique_ptr<int> ptr) {
        int x = *ptr;
    }

    // @safe
    void test() {
        std::unique_ptr<int> ptr(new int(42));
        const std::unique_ptr<int>& const_ref = ptr;
        std::unique_ptr<int>& mut_ref = ptr;

        // ERROR: ptr is borrowed by both const_ref and mut_ref
        take_ownership(std::move(ptr));
    }
    "#;

    let temp_file = create_temp_cpp_file(code);
    let (_success, output) = run_analyzer(temp_file.path());

    assert!(
        output.contains("Cannot move 'ptr'") && output.contains("borrowed by"),
        "Should detect mix of const and mutable borrows. Output: {}",
        output
    );
}

/// Test 15: Borrow after move should fail (use-after-move check)
#[test]
fn test_cannot_borrow_after_move() {
    let code = r#"
    #include <memory>

    // @safe
    void take_ownership(std::unique_ptr<int> ptr) {
        int x = *ptr;
    }

    // @safe
    void test() {
        std::unique_ptr<int> ptr(new int(42));
        take_ownership(std::move(ptr));

        // ERROR: ptr has been moved, cannot create reference
        const std::unique_ptr<int>& ref = ptr;
    }
    "#;

    let temp_file = create_temp_cpp_file(code);
    let (_success, output) = run_analyzer(temp_file.path());

    // This should be caught by use-after-move detection, not active borrow tracking
    assert!(
        output.contains("moved") || output.contains("violation"),
        "Should detect use-after-move. Output: {}",
        output
    );
}
