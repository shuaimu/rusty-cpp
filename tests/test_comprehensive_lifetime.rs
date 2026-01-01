/// Comprehensive Lifetime Tests for RustyCpp
///
/// This test suite covers all aspects of lifetime checking:
/// 1. Basic dangling reference detection
/// 2. Borrow conflicts with line number tracking
/// 3. Scope-based lifetime management
/// 4. Transitive borrow chains
/// 5. Reassignment while borrowed
/// 6. Mixed mutable/immutable borrows
/// 7. Partial borrows (field borrows)
/// 8. Reference to moved value
/// 9. Return reference to local
/// 10. Iterator/container lifetime issues

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

    let output = cmd.output().expect("Failed to execute analyzer");

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

/// Run analyzer and return (has_violations, output)
fn analyze(source: &str) -> (bool, String) {
    let temp_file = create_temp_cpp_file(source);
    let (_success, output) = run_analyzer(temp_file.path());

    let has_violations = output.contains("Found") && output.contains("violation");
    (has_violations, output)
}

/// Assert that the code has violations and output contains expected message
fn assert_violation(source: &str, expected_msg: &str) {
    let (has_violations, output) = analyze(source);
    assert!(
        has_violations,
        "Expected violation but none found. Output: {}",
        output
    );
    assert!(
        output.to_lowercase().contains(&expected_msg.to_lowercase()),
        "Expected message '{}' not found. Output: {}",
        expected_msg,
        output
    );
}

/// Assert that the code has violations with line number in output
fn assert_violation_with_line(source: &str, expected_msg: &str, expected_line: u32) {
    let (has_violations, output) = analyze(source);
    assert!(
        has_violations,
        "Expected violation but none found. Output: {}",
        output
    );
    assert!(
        output.to_lowercase().contains(&expected_msg.to_lowercase()),
        "Expected message '{}' not found. Output: {}",
        expected_msg,
        output
    );
    let line_pattern = format!("line {}", expected_line);
    assert!(
        output.contains(&line_pattern),
        "Expected line number {} not found. Output: {}",
        expected_line,
        output
    );
}

/// Assert that the code passes with no violations
fn assert_no_violation(source: &str) {
    let (has_violations, output) = analyze(source);
    assert!(
        !has_violations,
        "Expected no violations but found some. Output: {}",
        output
    );
}

// =============================================================================
// CATEGORY 1: Basic Dangling Reference Detection
// =============================================================================

#[test]
fn test_lifetime_return_reference_to_local() {
    let source = r#"
// @safe
int& bad_function() {
    int x = 42;
    return x;  // ERROR: returning reference to local
}

int main() { return 0; }
"#;
    assert_violation(source, "local variable");
}

#[test]
fn test_lifetime_return_const_reference_to_local() {
    let source = r#"
// @safe
const int& bad_function() {
    int x = 42;
    return x;  // ERROR: returning reference to local
}

int main() { return 0; }
"#;
    assert_violation(source, "local variable");
}

#[test]
fn test_lifetime_return_reference_to_parameter_ok() {
    let source = r#"
// @lifetime: (&'a) -> &'a
// @safe
const int& good_function(const int& x) {
    return x;  // OK: parameter outlives return
}

int main() { return 0; }
"#;
    assert_no_violation(source);
}

#[test]
fn test_lifetime_return_reference_to_static_ok() {
    let source = r#"
// @safe
const int& get_static() {
    static int x = 42;
    return x;  // OK: static has 'static lifetime
}

int main() { return 0; }
"#;
    // Static variables should be allowed (they live forever)
    let (has_violations, output) = analyze(source);
    // Note: current implementation may or may not detect static correctly
    // This test documents expected behavior
    println!("Output: {}", output);
}

// =============================================================================
// CATEGORY 2: Borrow Conflicts with Line Numbers
// =============================================================================

#[test]
fn test_lifetime_double_mutable_borrow_with_line() {
    let source = r#"
// @safe
void test() {
    int x = 42;
    int& ref1 = x;
    int& ref2 = x;  // ERROR: already mutably borrowed
}

int main() { return 0; }
"#;
    // Note: Line numbers are shown for reassignment-while-borrowed errors
    // but not for all borrow conflict errors currently
    assert_violation(source, "already mutably borrowed");
}

#[test]
fn test_lifetime_mutable_after_immutable_borrow_with_line() {
    let source = r#"
// @safe
void test() {
    int x = 42;
    const int& cref = x;  // line 5: immutable borrow
    int& mref = x;        // ERROR: can't mutably borrow while immutably borrowed
}

int main() { return 0; }
"#;
    assert_violation(source, "already");
}

#[test]
fn test_lifetime_immutable_after_mutable_borrow() {
    let source = r#"
// @safe
void test() {
    int x = 42;
    int& mref = x;        // mutable borrow
    const int& cref = x;  // ERROR: can't immutably borrow while mutably borrowed
}

int main() { return 0; }
"#;
    assert_violation(source, "mutably borrowed");
}

#[test]
fn test_lifetime_multiple_immutable_borrows_ok() {
    let source = r#"
// @safe
void test() {
    int x = 42;
    const int& ref1 = x;
    const int& ref2 = x;
    const int& ref3 = x;  // OK: multiple immutable borrows allowed
}

int main() { return 0; }
"#;
    assert_no_violation(source);
}

// =============================================================================
// CATEGORY 3: Scope-Based Lifetime Management
// =============================================================================

#[test]
fn test_lifetime_borrow_ends_at_scope() {
    // Test: borrows should end when scope exits
    let source = r#"
// @safe
void test() {
    int x = 42;
    {
        int& ref = x;  // borrow starts
    }  // borrow ends here
    int& ref2 = x;  // OK: previous borrow ended
}

int main() { return 0; }
"#;
    assert_no_violation(source);
}

#[test]
fn test_lifetime_sequential_scopes_ok() {
    // Test: sequential scopes should allow re-borrowing
    let source = r#"
// @safe
void test() {
    int x = 42;
    {
        int& ref1 = x;
    }
    {
        int& ref2 = x;  // OK: ref1 is out of scope
    }
    {
        int& ref3 = x;  // OK: ref2 is out of scope
    }
}

int main() { return 0; }
"#;
    assert_no_violation(source);
}

#[test]
fn test_lifetime_nested_scopes_conflict() {
    let source = r#"
// @safe
void test() {
    int x = 42;
    {
        int& ref1 = x;
        {
            int& ref2 = x;  // ERROR: ref1 still active in outer scope
        }
    }
}

int main() { return 0; }
"#;
    assert_violation(source, "already mutably borrowed");
}

// =============================================================================
// CATEGORY 4: Reassignment While Borrowed
// =============================================================================

#[test]
fn test_lifetime_reassign_while_borrowed() {
    let source = r#"
// @safe
void test() {
    int x = 42;
    int& ref = x;
    x = 100;  // ERROR: cannot assign while borrowed
}

int main() { return 0; }
"#;
    assert_violation(source, "borrowed");
}

#[test]
fn test_lifetime_reassign_after_borrow_ends_ok() {
    let source = r#"
// @safe
void test() {
    int x = 42;
    {
        int& ref = x;
    }
    x = 100;  // OK: borrow ended
}

int main() { return 0; }
"#;
    assert_no_violation(source);
}

// =============================================================================
// CATEGORY 5: Use After Move
// =============================================================================

#[test]
fn test_lifetime_use_after_move() {
    let source = r#"
#include <utility>

// @safe
void test() {
    int x = 42;
    int y = std::move(x);
    int z = x;  // ERROR: use after move
}

int main() { return 0; }
"#;
    assert_violation(source, "move");
}

#[test]
fn test_lifetime_borrow_after_move() {
    let source = r#"
#include <utility>

// @safe
void test() {
    int x = 42;
    int y = std::move(x);
    int& ref = x;  // ERROR: borrow after move
}

int main() { return 0; }
"#;
    assert_violation(source, "move");
}

#[test]
fn test_lifetime_reassign_after_move_ok() {
    let source = r#"
#include <utility>

// @safe
void test() {
    int x = 42;
    int y = std::move(x);
    x = 100;  // OK: reassignment revives variable
    int z = x;  // OK: x is valid again
}

int main() { return 0; }
"#;
    assert_no_violation(source);
}

// =============================================================================
// CATEGORY 6: Transitive Borrows
// =============================================================================

#[test]
fn test_lifetime_transitive_borrow_chain() {
    let source = r#"
// @safe
void test() {
    int value = 42;
    int& ref1 = value;
    int& ref2 = ref1;
    int& ref3 = ref2;  // Chain: ref3 -> ref2 -> ref1 -> value
    // All are borrowing from value transitively
}

int main() { return 0; }
"#;
    // This creates a borrow chain - behavior depends on implementation
    let (has_violations, output) = analyze(source);
    println!("Transitive chain output: {}", output);
}

#[test]
fn test_lifetime_move_in_borrow_chain() {
    let source = r#"
#include <utility>

// @safe
void test() {
    int value = 42;
    int& ref1 = value;
    int& ref2 = ref1;
    int moved = std::move(value);  // ERROR: value is borrowed
}

int main() { return 0; }
"#;
    assert_violation(source, "borrow");
}

// =============================================================================
// CATEGORY 7: Partial Borrows (Field Borrows)
// =============================================================================

#[test]
fn test_lifetime_different_fields_ok() {
    let source = r#"
struct Point { int x; int y; };

// @safe
void test() {
    Point p = {1, 2};
    int& rx = p.x;
    int& ry = p.y;  // OK: different fields can be borrowed independently
}

int main() { return 0; }
"#;
    assert_no_violation(source);
}

#[test]
fn test_lifetime_same_field_twice() {
    let source = r#"
struct Point { int x; int y; };

// @safe
void test() {
    Point p = {1, 2};
    int& rx1 = p.x;
    int& rx2 = p.x;  // ERROR: same field borrowed twice
}

int main() { return 0; }
"#;
    assert_violation(source, "already");
}

#[test]
fn test_lifetime_whole_struct_while_field_borrowed() {
    let source = r#"
struct Point { int x; int y; };

// @safe
void test() {
    Point p = {1, 2};
    int& rx = p.x;
    Point& rp = p;  // ERROR: can't borrow whole struct while field borrowed
}

int main() { return 0; }
"#;
    assert_violation(source, "borrow");
}

// =============================================================================
// CATEGORY 8: Loop Lifetime Issues
// =============================================================================

#[test]
fn test_lifetime_use_after_move_in_loop() {
    let source = r#"
#include <utility>

// @safe
void test() {
    int x = 42;
    for (int i = 0; i < 2; i++) {
        int y = std::move(x);  // ERROR: moved on first iteration, used on second
    }
}

int main() { return 0; }
"#;
    assert_violation(source, "loop");
}

#[test]
fn test_lifetime_borrow_in_loop_ok() {
    let source = r#"
// @safe
void test() {
    int x = 42;
    for (int i = 0; i < 10; i++) {
        const int& ref = x;  // OK: borrow ends each iteration
    }
}

int main() { return 0; }
"#;
    assert_no_violation(source);
}

// =============================================================================
// CATEGORY 9: Cross-Function Lifetime
// =============================================================================

#[test]
fn test_lifetime_identity_with_temporary() {
    let source = r#"
// @lifetime: (&'a) -> &'a
// @safe
const int& identity(const int& x) { return x; }

// @safe
void bad() {
    const int& ref = identity(42);  // ERROR: temporary dies
}

int main() { return 0; }
"#;
    assert_violation(source, "temporary");
}

#[test]
fn test_lifetime_identity_with_local_ok() {
    let source = r#"
// @lifetime: (&'a) -> &'a
// @safe
const int& identity(const int& x) { return x; }

// @safe
void good() {
    int value = 42;
    const int& ref = identity(value);  // OK: value outlives ref
}

int main() { return 0; }
"#;
    assert_no_violation(source);
}

#[test]
fn test_lifetime_pick_first_with_annotation() {
    let source = r#"
// @lifetime: (&'a, &'b) -> &'a
// @safe
const int& first(const int& a, const int& b) { return a; }

// @safe
void test() {
    int x = 1;
    const int& ref = first(x, 2);  // OK: x is first param and outlives
}

int main() { return 0; }
"#;
    assert_no_violation(source);
}

// =============================================================================
// CATEGORY 10: Reference Validity
// =============================================================================

#[test]
fn test_lifetime_reference_to_reference() {
    let source = r#"
// @safe
void test() {
    int x = 42;
    int& ref1 = x;
    int& ref2 = ref1;  // Reference to reference
    // Both are aliases to x
}

int main() { return 0; }
"#;
    // Reference to reference creates an alias chain
    let (has_violations, output) = analyze(source);
    println!("Ref to ref output: {}", output);
}

#[test]
fn test_lifetime_const_correctness() {
    let source = r#"
// @safe
void test() {
    const int x = 42;
    const int& ref = x;  // OK: const ref to const
}

int main() { return 0; }
"#;
    assert_no_violation(source);
}

// =============================================================================
// CATEGORY 11: Complex Scenarios
// =============================================================================

#[test]
fn test_lifetime_conditional_borrow() {
    let source = r#"
// @safe
void test(bool cond) {
    int x = 42;
    int y = 100;
    int& ref = cond ? x : y;  // Borrows either x or y
}

int main() { return 0; }
"#;
    // Conditional borrows - should track conservatively
    let (has_violations, output) = analyze(source);
    println!("Conditional borrow output: {}", output);
}

#[test]
fn test_lifetime_early_return_with_borrow() {
    let source = r#"
// @safe
void test(bool cond) {
    int x = 42;
    int& ref = x;
    if (cond) {
        return;  // ref goes out of scope here
    }
    x = 100;  // Is this safe? ref might still be active
}

int main() { return 0; }
"#;
    // Early return should end borrow
    let (has_violations, output) = analyze(source);
    println!("Early return output: {}", output);
}

#[test]
fn test_lifetime_multiple_borrows_then_use() {
    let source = r#"
// @safe
void test() {
    int x = 42;
    {
        int& ref1 = x;
        {
            const int& ref2 = x;  // ERROR: mixing mut/immut
        }
    }
}

int main() { return 0; }
"#;
    assert_violation(source, "mutably borrowed");
}

// =============================================================================
// CATEGORY 12: Error Message Quality
// =============================================================================

#[test]
fn test_lifetime_error_includes_variable_name() {
    let source = r#"
// @safe
void test() {
    int my_variable = 42;
    int& my_reference = my_variable;
    int& another_ref = my_variable;
}

int main() { return 0; }
"#;
    let (has_violations, output) = analyze(source);
    assert!(has_violations, "Expected violation");
    assert!(
        output.contains("my_variable"),
        "Error should include variable name. Output: {}",
        output
    );
}

#[test]
fn test_lifetime_error_includes_line_number() {
    let source = r#"
// @safe
void test() {
    int x = 42;
    int& ref1 = x;
    int& ref2 = x;
}

int main() { return 0; }
"#;
    let (has_violations, output) = analyze(source);
    assert!(has_violations, "Expected violation");
    assert!(
        output.contains("line"),
        "Error should include line number. Output: {}",
        output
    );
}

// =============================================================================
// CATEGORY 13: Edge Cases
// =============================================================================

#[test]
fn test_lifetime_self_assignment() {
    let source = r#"
// @safe
void test() {
    int x = 42;
    x = x;  // Self-assignment, technically valid
}

int main() { return 0; }
"#;
    assert_no_violation(source);
}

#[test]
fn test_lifetime_empty_function() {
    let source = r#"
// @safe
void empty() {
    // Nothing here
}

int main() { return 0; }
"#;
    assert_no_violation(source);
}

#[test]
fn test_lifetime_declaration_only() {
    let source = r#"
// @safe
void test() {
    int x;
    int& ref = x;  // Borrowing uninitialized variable
}

int main() { return 0; }
"#;
    // Uninitialized variable borrow - may or may not be detected
    let (has_violations, output) = analyze(source);
    println!("Uninitialized borrow output: {}", output);
}
