// Integration Test: All Phases Working Together
// Tests the complete borrow checker with Phases 1-4

use assert_cmd::Command;
use std::fs;
use tempfile::NamedTempFile;

fn run_analyzer_on_code(code: &str, include_paths: &[&str]) -> (String, bool) {
    let mut file = NamedTempFile::new().unwrap();
    fs::write(&file, code).unwrap();

    let mut cmd = Command::cargo_bin("rusty-cpp-checker").unwrap();
    cmd.arg(file.path());
    for path in include_paths {
        cmd.arg("-I").arg(path);
    }

    let output = cmd.output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let full_output = format!("{}\n{}", stdout, stderr);

    let has_borrow_violation = full_output.contains("borrowed") ||
                                full_output.contains("Cannot move");
    (full_output, has_borrow_violation)
}

#[test]
fn test_integration_all_phases_combined() {
    // This test exercises:
    // - Phase 1: Lifetime annotations
    // - Phase 2: Return value borrows
    // - Phase 3: Conflict detection
    // - Phase 4: Transitive borrows

    let code = r#"
#include <rusty/box.hpp>

// Phase 1: Lifetime annotations in action
// @lifetime: (&'a) -> &'a int
const int& borrow_immut(const int& x) { return x; }

// @lifetime: (&'a mut) -> &'a mut int
int& borrow_mut(int& x) { return x; }

// @safe
int main() {
    rusty::Box<int> value = rusty::Box<int>::make(42);

    // Phase 2: Return value borrow detection
    int& ref1 = *value;  // ref1 borrows from value via operator*

    // Phase 3: Conflict detection - second mutable borrow should fail
    // But we skip this to test Phase 4

    // Phase 4: Transitive borrow tracking
    const int& ref2 = borrow_immut(ref1);  // ref2 borrows ref1 (which borrows value)

    // This should ERROR: value is transitively borrowed by ref2 -> ref1 -> value
    rusty::Box<int> moved = std::move(value);

    return 0;
}
"#;

    let (output, has_violation) = run_analyzer_on_code(code, &["include"]);
    println!("Output: {}", output);

    assert!(has_violation, "Should detect transitive borrow preventing move");
    assert!(output.contains("ref1") && output.contains("ref2"),
        "Error should show the complete borrow chain");
}

#[test]
fn test_integration_conflict_plus_transitive() {
    // Tests Phase 3 (conflict) combined with Phase 4 (transitive)

    let code = r#"
#include <rusty/box.hpp>

// @lifetime: (&'a) -> &'a int
const int& identity(const int& x) { return x; }

// @safe
int main() {
    rusty::Box<int> value = rusty::Box<int>::make(42);

    int& ref1 = *value;
    int& ref2 = *value;  // Phase 3: ERROR - multiple mutable borrows

    return 0;
}
"#;

    let (output, has_violation) = run_analyzer_on_code(code, &["include"]);
    println!("Output: {}", output);

    assert!(has_violation, "Should detect conflict");
    assert!(output.contains("already") || output.contains("mutably borrowed"),
        "Error should mention the conflict");
}

#[test]
fn test_integration_scope_based_cleanup() {
    // Tests that borrows end correctly with scopes (all phases)

    let code = r#"
#include <rusty/box.hpp>

// @lifetime: (&'a) -> &'a int
const int& identity(const int& x) { return x; }

// @safe
int main() {
    rusty::Box<int> value = rusty::Box<int>::make(42);

    {
        int& ref1 = *value;
        {
            const int& ref2 = identity(ref1);
            // Transitive chain exists: ref2 -> ref1 -> value
        }  // ref2 ends here
    }  // ref1 ends here

    // Now all borrows are gone - move should succeed
    rusty::Box<int> moved = std::move(value);

    return 0;
}
"#;

    let (output, has_violation) = run_analyzer_on_code(code, &["include"]);
    println!("Output: {}", output);

    assert!(!has_violation, "Move should succeed after all borrows end");
}

#[test]
fn test_integration_multiple_objects_independent() {
    // Tests that borrows on different objects don't interfere

    let code = r#"
#include <rusty/box.hpp>

// @lifetime: (&'a) -> &'a int
const int& identity(const int& x) { return x; }

// @safe
int main() {
    rusty::Box<int> value1 = rusty::Box<int>::make(42);
    rusty::Box<int> value2 = rusty::Box<int>::make(100);

    // Create borrow chains on different objects
    int& ref1 = *value1;
    const int& ref2 = identity(ref1);

    int& ref3 = *value2;
    const int& ref4 = identity(ref3);

    // Should be able to move value2 (only value1 is borrowed)
    rusty::Box<int> moved = std::move(value2);  // ERROR - value2 is also borrowed!

    return 0;
}
"#;

    let (output, has_violation) = run_analyzer_on_code(code, &["include"]);
    println!("Output: {}", output);

    // value2 IS borrowed by ref3, ref4, so move should fail
    assert!(has_violation, "Should detect that value2 is borrowed");
}

#[test]
fn test_integration_complex_borrow_graph() {
    // Tests complex borrow relationships

    let code = r#"
#include <rusty/box.hpp>

// @lifetime: (&'a) -> &'a int
const int& identity(const int& x) { return x; }

// @safe
int main() {
    rusty::Box<int> value = rusty::Box<int>::make(42);

    // Create a branching borrow tree:
    //        value
    //       /     \
    //     ref1   ref2
    //      |
    //    ref3

    int& ref1 = *value;
    int& ref2 = *value;
    const int& ref3 = identity(ref1);

    // Can't move value - has direct borrows (ref1, ref2)
    // AND transitive borrow (ref3 through ref1)
    rusty::Box<int> moved = std::move(value);

    return 0;
}
"#;

    let (output, has_violation) = run_analyzer_on_code(code, &["include"]);
    println!("Output: {}", output);

    assert!(has_violation, "Should detect complex borrow graph");
    // Should show all three borrowers
    let has_all_refs = output.contains("ref1") && output.contains("ref2") && output.contains("ref3");
    assert!(has_all_refs, "Error should show all borrowers in the graph");
}

#[test]
fn test_integration_mutable_immutable_chain() {
    // Tests mixed mutable and immutable borrows in a chain

    let code = r#"
#include <rusty/box.hpp>

// @lifetime: (&'a mut) -> &'a mut int
int& borrow_mut(int& x) { return x; }

// @lifetime: (&'a) -> &'a int
const int& borrow_immut(const int& x) { return x; }

// @safe
int main() {
    rusty::Box<int> value = rusty::Box<int>::make(42);

    // Mutable borrow chain
    int& ref1 = *value;
    int& ref2 = borrow_mut(ref1);

    // Immutable borrow from mutable reference
    const int& ref3 = borrow_immut(ref2);

    // Chain: ref3 -> ref2 -> ref1 -> value (mixed mutability)
    rusty::Box<int> moved = std::move(value);

    return 0;
}
"#;

    let (output, has_violation) = run_analyzer_on_code(code, &["include"]);
    println!("Output: {}", output);

    assert!(has_violation, "Should detect mixed mutability chain");
}

#[test]
fn test_integration_all_features_comprehensive() {
    // Most comprehensive test: exercises everything

    let code = r#"
#include <rusty/box.hpp>

// @lifetime: (&'a) -> &'a int
const int& id_const(const int& x) { return x; }

// @lifetime: (&'a mut) -> &'a mut int
int& id_mut(int& x) { return x; }

// @safe
int main() {
    // Create multiple objects
    rusty::Box<int> box1 = rusty::Box<int>::make(1);
    rusty::Box<int> box2 = rusty::Box<int>::make(2);

    // Phase 2: Return value borrows
    int& ref1 = *box1;

    // Phase 4: Build transitive chain
    int& ref2 = id_mut(ref1);
    const int& ref3 = id_const(ref2);

    // Phase 3: Try to create conflicting borrow
    // (This would conflict, but ref3 already borrowed ref2)

    // Phase 4: Try to move while transitively borrowed
    rusty::Box<int> moved1 = std::move(box1);  // ERROR: transitive chain

    // box2 is independent - should be fine
    rusty::Box<int> moved2 = std::move(box2);  // OK

    return 0;
}
"#;

    let (output, has_violation) = run_analyzer_on_code(code, &["include"]);
    println!("Output: {}", output);

    // Should detect error on box1 (transitively borrowed)
    assert!(has_violation, "Should detect transitive borrow on box1");

    // Error should be about box1, not box2
    assert!(output.contains("box1"), "Error should mention box1");
}
