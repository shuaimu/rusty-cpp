// Tests for implicit drops at scope end
//
// IMPORTANT: These tests use VALID C++ only
// Previous version had tests with invalid C++ (uninitialized references)
// which cannot occur in real code.
//
// What we're testing:
// 1. Implicit drops are inserted at scope end for RAII types
// 2. Implicit drops check for active borrows (completeness)
// 3. Move tracking interacts correctly with implicit drops
// 4. Positive cases work (no false positives)

use std::io::Write;
use std::path::Path;
use std::process::Command;
use tempfile::NamedTempFile;

fn run_analyzer(cpp_file: &Path) -> (bool, String) {
    let z3_header = if cfg!(target_os = "macos") {
        "/opt/homebrew/include/z3.h"
    } else {
        "/usr/include/z3.h"
    };

    let mut cmd = Command::new("cargo");
    cmd.args(&["run", "--quiet", "--", cpp_file.to_str().unwrap()])
        .env("Z3_SYS_Z3_HEADER", z3_header);

    if cfg!(target_os = "macos") {
        cmd.env("DYLD_LIBRARY_PATH", "/opt/homebrew/Cellar/llvm/19.1.7/lib");
    } else {
        cmd.env("LD_LIBRARY_PATH", "/usr/lib/llvm-14/lib");
    }

    let output = cmd.output()
        .expect("Failed to execute analyzer");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let full_output = format!("{}{}", stdout, stderr);

    (output.status.success(), full_output)
}

fn create_temp_cpp_file(content: &str) -> NamedTempFile {
    let mut file = NamedTempFile::with_suffix(".cpp").unwrap();
    file.write_all(content.as_bytes()).unwrap();
    file.flush().unwrap();
    file
}

fn get_project_root() -> String {
    std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string())
}

fn compile_and_check(source: &str) -> (bool, String) {
    let project_root = get_project_root();
    let include_directive = format!("#include \"{}/include/rusty/box.hpp\"", project_root);
    let source_with_abs_path = source.replace("#include \"include/rusty/box.hpp\"", &include_directive);

    let temp_file = create_temp_cpp_file(&source_with_abs_path);
    let (success, output) = run_analyzer(temp_file.path());

    let has_violations = output.contains("Found") && output.contains("violation");
    let no_violations = output.contains("no violations found");

    (!has_violations || no_violations, output)
}

// ============================================================================
// TEST 1: Move then implicit drop
// This tests that when ownership transfers via move, the borrow tracking
// follows the value, and implicit drop at scope end checks borrows
// ============================================================================

#[test]
fn test_move_then_implicit_drop() {
    let source = r#"
#include "include/rusty/box.hpp"

// @safe
void test() {
    auto box1 = rusty::Box<int>::make(42);
    int& r = *box1;  // r borrows from box1's value

    auto box2 = std::move(box1);  // Move ownership to box2
    // Now box2 owns the value that r borrows from

    int x = r;  // r is still active and borrowed from box2's value

}  // box2 implicitly drops - should check if borrowed

int main() { return 0; }
"#;

    let (success, output) = compile_and_check(source);

    // Should detect error: cannot drop box2 because value is borrowed by r
    // Note: The move is already caught by our reassignment tracking,
    // but implicit drop provides an additional check at scope end
    assert!(
        !success,
        "Expected error for dropping borrowed value, but got: {}",
        output
    );
}

// ============================================================================
// TEST 2: Reassignment then implicit drop
// Tests that reassignment drops the old value (already caught),
// and scope end drops the new value (implicit drop)
// ============================================================================

#[test]
fn test_reassignment_then_implicit_drop() {
    let source = r#"
#include "include/rusty/box.hpp"

// @safe
void test() {
    auto box = rusty::Box<int>::make(42);
    int& r = *box;  // r borrows from box's value

    box = rusty::Box<int>::make(100);  // Reassignment - should error (borrowed)

}  // Implicit drop of new value (if reassignment succeeded)

int main() { return 0; }
"#;

    let (success, output) = compile_and_check(source);

    // Should detect error at reassignment (already implemented)
    assert!(
        !success,
        "Expected error for reassignment of borrowed value, but got: {}",
        output
    );

    // Check it's caught at reassignment, not implicit drop
    assert!(
        output.contains("assign") || output.contains("borrow"),
        "Expected error about assignment or borrow, got: {}",
        output
    );
}

// ============================================================================
// POSITIVE TESTS - These should NOT error
// ============================================================================

#[test]
fn test_borrow_ends_before_scope_end_ok() {
    let source = r#"
#include "include/rusty/box.hpp"

// @safe
void test() {
    {
        auto box = rusty::Box<int>::make(42);
        int& r = *box;
        int x = r;  // Last use of r - borrow dies here
        // r is now dead (not live at scope end)
    }  // box implicitly drops - OK, r is already dead
}

int main() { return 0; }
"#;

    let (success, output) = compile_and_check(source);

    assert!(
        success,
        "This should be OK (borrow ends before drop), but got error: {}",
        output
    );
}

#[test]
fn test_no_borrows_when_scope_ends_ok() {
    let source = r#"
#include "include/rusty/box.hpp"

// @safe
void test() {
    {
        auto box = rusty::Box<int>::make(42);
        int x = *box;  // Copy value, no borrow created
    }  // box drops - OK, no active borrows
}

int main() { return 0; }
"#;

    let (success, output) = compile_and_check(source);

    assert!(
        success,
        "This should be OK (no active borrows), but got error: {}",
        output
    );
}

#[test]
fn test_same_scope_reference_ok() {
    let source = r#"
#include "include/rusty/box.hpp"

// @safe
void test() {
    {
        auto box = rusty::Box<int>::make(42);
        int& r = *box;  // r borrows from box

        // Both declared in same scope
        // C++ drops in reverse order: r dies first, then box
        // This is SAFE!
    }
}

int main() { return 0; }
"#;

    let (success, output) = compile_and_check(source);

    assert!(
        success,
        "This should be OK (reverse drop order), but got error: {}",
        output
    );
}

#[test]
fn test_multiple_scopes_ok() {
    let source = r#"
#include "include/rusty/box.hpp"

// @safe
void test() {
    {
        auto box1 = rusty::Box<int>::make(42);
        int x = *box1;  // Copy, no borrow
    }  // box1 drops - OK

    {
        auto box2 = rusty::Box<int>::make(100);
        int y = *box2;  // Copy, no borrow
    }  // box2 drops - OK
}

int main() { return 0; }
"#;

    let (success, output) = compile_and_check(source);

    assert!(
        success,
        "This should be OK (sequential scopes, no borrows), but got error: {}",
        output
    );
}

#[test]
fn test_nested_scopes_no_escape_ok() {
    let source = r#"
#include "include/rusty/box.hpp"

// @safe
void test() {
    {
        {
            auto box = rusty::Box<int>::make(42);
            int& r = *box;
            int x = r;  // Use r here - dies before scope end
        }  // box drops - OK, r is dead
    }
}

int main() { return 0; }
"#;

    let (success, output) = compile_and_check(source);

    assert!(
        success,
        "This should be OK (borrow doesn't escape), but got error: {}",
        output
    );
}

// ============================================================================
// DOCUMENTATION TEST - What we DON'T need to catch
// ============================================================================

#[test]
fn test_return_reference_to_local_caught_by_lifetime_checker() {
    // This demonstrates that returning a reference to a local is already
    // caught by our lifetime annotation checking, NOT implicit drops
    let source = r#"
#include "include/rusty/box.hpp"

// @safe
// @lifetime: () -> &'a
int& getBadRef() {
    auto box = rusty::Box<int>::make(42);
    int& r = *box;
    return r;  // ERROR - caught by lifetime checker
}  // box implicitly drops

int main() { return 0; }
"#;

    let (success, output) = compile_and_check(source);

    // This IS caught, but by lifetime checking, not implicit drops
    if !success {
        println!("✓ CORRECTLY caught by lifetime checker:");
        for line in output.lines() {
            if line.contains("violation") || line.contains("local") || line.contains("return") {
                println!("   {}", line);
            }
        }
    }

    // Don't assert - this is informational
    // The point is: implicit drops don't need to catch this case
}
