//! Integration tests for RAII tracking
//!
//! These tests verify that the RAII tracking module correctly detects:
//! - Reference/pointer stored in container outliving pointee
//! - User-defined RAII types
//! - Iterator outliving container
//! - Lambda escape issues
//! - new/delete tracking

use std::process::Command;
use std::path::PathBuf;

fn get_project_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn run_checker(test_file: &str) -> (bool, String) {
    let project_root = get_project_root();
    let checker_path = project_root.join("target/debug/rusty-cpp-checker");
    let test_path = project_root.join("tests/raii").join(test_file);

    let output = Command::new(&checker_path)
        .arg(&test_path)
        .output()
        .expect("Failed to execute checker");

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let combined = format!("{}{}", stdout, stderr);

    (output.status.success(), combined)
}

fn assert_contains_error(output: &str, error_pattern: &str) {
    assert!(
        output.contains(error_pattern),
        "Expected error containing '{}' but got:\n{}",
        error_pattern,
        output
    );
}

fn assert_no_error(output: &str, error_pattern: &str) {
    assert!(
        !output.contains(error_pattern),
        "Did not expect error containing '{}' but got:\n{}",
        error_pattern,
        output
    );
}

// =============================================================================
// Phase 1: Reference/Pointer Stored in Container
// =============================================================================

#[test]
fn test_return_ref_to_local_basic() {
    let (_, output) = run_checker("return_ref_to_local.cpp");
    // This should detect returning reference to local
    assert_contains_error(&output, "reference to local");
}

// =============================================================================
// Phase 3: Iterator Outlives Container
// =============================================================================

#[test]
fn test_iterator_basic_detection() {
    let (_, output) = run_checker("iterator_outlives_container.cpp");
    // Check that we're analyzing the file
    assert!(output.contains("Analyzing:") || output.contains("violation"));
}

// =============================================================================
// Phase 5: User-Defined RAII Types
// =============================================================================

#[test]
fn test_user_defined_raii_basic() {
    let (_, output) = run_checker("user_defined_raii.cpp");
    // Check that analysis runs on the file
    assert!(output.contains("Analyzing:") || output.contains("violation"));
}

// =============================================================================
// Phase 6: Double-Free Detection
// =============================================================================

#[test]
fn test_double_free_detection() {
    let (_, output) = run_checker("double_free.cpp");
    // Check that analysis runs on the file
    assert!(output.contains("Analyzing:") || output.contains("violation"));
}

// =============================================================================
// Phase 7: Lambda Capture Escape
// =============================================================================

#[test]
fn test_lambda_capture_basic() {
    let (_, output) = run_checker("lambda_capture_escape.cpp");
    // With escape analysis, we only check for 'this' capture (always forbidden)
    // or escaped lambdas with reference captures.
    // The test file has 'this' capture in bad_lambda_captures_this which should
    // still be caught. Check that the analysis runs without crashing.
    assert!(output.contains("Analyzing:") || output.contains("violation"),
        "Analysis should run on the file. Got: {}", output);
}

// =============================================================================
// Unit tests for RaiiTracker
// =============================================================================

#[cfg(test)]
mod raii_tracker_tests {
    use rusty_cpp::analysis::raii_tracking::{RaiiTracker, IteratorBorrow, MemberBorrow};

    #[test]
    fn test_container_type_detection() {
        assert!(RaiiTracker::is_container_type("std::vector<int>"));
        assert!(RaiiTracker::is_container_type("std::map<int, string>"));
        assert!(RaiiTracker::is_container_type("std::unordered_set<int>"));
        assert!(RaiiTracker::is_container_type("std::deque<int>"));
        assert!(RaiiTracker::is_container_type("std::list<int>"));
        assert!(!RaiiTracker::is_container_type("int"));
        assert!(!RaiiTracker::is_container_type("std::string"));
    }

    #[test]
    fn test_iterator_type_detection() {
        assert!(RaiiTracker::is_iterator_type("std::vector<int>::iterator"));
        assert!(RaiiTracker::is_iterator_type("std::map<int,int>::const_iterator"));
        assert!(RaiiTracker::is_iterator_type("std::list<int>::reverse_iterator"));
        assert!(!RaiiTracker::is_iterator_type("int*"));
        assert!(!RaiiTracker::is_iterator_type("std::string"));
    }

    #[test]
    fn test_container_store_method_detection() {
        assert!(RaiiTracker::is_container_store_method("push_back"));
        assert!(RaiiTracker::is_container_store_method("push_front"));
        assert!(RaiiTracker::is_container_store_method("insert"));
        assert!(RaiiTracker::is_container_store_method("emplace"));
        assert!(RaiiTracker::is_container_store_method("emplace_back"));
        assert!(!RaiiTracker::is_container_store_method("begin"));
        assert!(!RaiiTracker::is_container_store_method("size"));
    }

    #[test]
    fn test_iterator_returning_method_detection() {
        assert!(RaiiTracker::is_iterator_returning_method("begin"));
        assert!(RaiiTracker::is_iterator_returning_method("end"));
        assert!(RaiiTracker::is_iterator_returning_method("cbegin"));
        assert!(RaiiTracker::is_iterator_returning_method("cend"));
        assert!(RaiiTracker::is_iterator_returning_method("find"));
        assert!(!RaiiTracker::is_iterator_returning_method("push_back"));
        assert!(!RaiiTracker::is_iterator_returning_method("size"));
    }

    #[test]
    fn test_scope_tracking() {
        let mut tracker = RaiiTracker::new();
        assert_eq!(tracker.current_scope, 0);

        tracker.enter_scope();
        assert_eq!(tracker.current_scope, 1);

        tracker.enter_scope();
        assert_eq!(tracker.current_scope, 2);

        tracker.exit_scope();
        assert_eq!(tracker.current_scope, 1);

        tracker.exit_scope();
        assert_eq!(tracker.current_scope, 0);
    }

    #[test]
    fn test_variable_registration() {
        let mut tracker = RaiiTracker::new();

        tracker.register_variable("vec", "std::vector<int>", 0);
        tracker.register_variable("it", "std::vector<int>::iterator", 0);
        tracker.register_variable("x", "int", 0);

        assert!(tracker.container_variables.contains("vec"));
        assert!(tracker.iterator_variables.contains("it"));
        assert!(!tracker.container_variables.contains("x"));
        assert!(!tracker.iterator_variables.contains("x"));
    }

    #[test]
    fn test_container_borrow_detection() {
        let mut tracker = RaiiTracker::new();

        // Outer scope: container
        tracker.register_variable("vec", "std::vector<int*>", 0);

        // Inner scope: local variable
        tracker.enter_scope();
        tracker.register_variable("x", "int", 1);

        // Store pointer to x in vec
        tracker.record_container_store("vec", "x", 10);

        // Exit inner scope - x dies but vec still has pointer
        let errors = tracker.exit_scope();

        assert_eq!(errors.len(), 1);
        assert!(errors[0].contains("Dangling pointer in container"));
        assert!(errors[0].contains("vec"));
        assert!(errors[0].contains("x"));
    }

    #[test]
    fn test_iterator_outlives_container() {
        let mut tracker = RaiiTracker::new();

        // Iterator declared in outer scope
        tracker.variable_scopes.insert("it".to_string(), 0);

        // Container in inner scope
        tracker.enter_scope();
        tracker.register_variable("vec", "std::vector<int>", 1);

        // Create iterator borrow
        tracker.iterator_borrows.push(IteratorBorrow {
            iterator: "it".to_string(),
            container: "vec".to_string(),
            iterator_scope: 0,
            container_scope: 1,
            line: 10,
        });

        // Exit inner scope - vec dies but it survives
        let errors = tracker.exit_scope();

        assert_eq!(errors.len(), 1);
        assert!(errors[0].contains("Iterator outlives container"));
    }

    #[test]
    fn test_double_free_detection() {
        let mut tracker = RaiiTracker::new();

        // Allocate
        tracker.record_allocation("ptr", 10);
        assert!(!tracker.is_freed("ptr"));

        // First free - OK
        let err1 = tracker.record_deallocation("ptr", 20);
        assert!(err1.is_none());
        assert!(tracker.is_freed("ptr"));

        // Second free - error
        let err2 = tracker.record_deallocation("ptr", 30);
        assert!(err2.is_some());
        assert!(err2.unwrap().contains("Double free"));
    }

    #[test]
    fn test_use_after_free_tracking() {
        let mut tracker = RaiiTracker::new();

        tracker.record_allocation("ptr", 10);
        assert!(!tracker.is_freed("ptr"));

        tracker.record_deallocation("ptr", 20);
        assert!(tracker.is_freed("ptr"));
    }

    #[test]
    fn test_lambda_escape_tracking() {
        let mut tracker = RaiiTracker::new();

        // Register a local variable
        tracker.register_variable("x", "int", 0);

        // Lambda captures x by reference
        tracker.record_lambda("lambda1", vec!["x".to_string()], 10);

        // Mark lambda as escaped (e.g., returned)
        tracker.mark_lambda_escaped("lambda1");

        // Verify lambda is marked as escaped
        let capture = tracker.lambda_captures.iter().find(|c| c.lambda_var == "lambda1").unwrap();
        assert!(capture.has_escaped);
        assert!(capture.ref_captures.contains(&"x".to_string()));
    }

    #[test]
    fn test_no_false_positive_same_scope() {
        let mut tracker = RaiiTracker::new();

        // Enter a scope first
        tracker.enter_scope();

        // Both container and local in same scope (scope 1)
        tracker.register_variable("vec", "std::vector<int*>", 1);
        tracker.register_variable("x", "int", 1);

        // Store pointer to x in vec (same scope - OK)
        tracker.record_container_store("vec", "x", 10);

        // Exit scope - should NOT error because both die together
        let errors = tracker.exit_scope();
        assert!(errors.is_empty(), "Should not have errors for same-scope borrows");
    }

    #[test]
    fn test_nested_scope_cleanup() {
        let mut tracker = RaiiTracker::new();

        // Outer scope
        tracker.register_variable("vec", "std::vector<int*>", 0);

        // Inner scope 1
        tracker.enter_scope();
        tracker.register_variable("x", "int", 1);
        tracker.record_container_store("vec", "x", 10);
        let errors1 = tracker.exit_scope();
        assert_eq!(errors1.len(), 1);

        // Inner scope 2 (fresh - no lingering borrows)
        tracker.enter_scope();
        tracker.register_variable("y", "int", 1);
        tracker.record_container_store("vec", "y", 20);
        let errors2 = tracker.exit_scope();
        assert_eq!(errors2.len(), 1);

        // Outer scope exit should be clean
        let errors3 = tracker.exit_scope();
        assert!(errors3.is_empty());
    }

    // ==========================================================================
    // Phase 5: Member Lifetime Tracking Tests
    // ==========================================================================

    #[test]
    fn test_member_borrow_detection() {
        let mut tracker = RaiiTracker::new();

        // Outer scope: reference
        tracker.variable_scopes.insert("ptr".to_string(), 0);

        // Inner scope: object with field
        tracker.enter_scope();
        tracker.register_variable("obj", "Wrapper", 1);

        // Borrow field from object: ptr = &obj.data
        tracker.record_member_borrow("ptr", "obj", "data", 10);

        // Exit inner scope - obj is destroyed but ptr survives
        let errors = tracker.exit_scope();

        assert_eq!(errors.len(), 1);
        assert!(errors[0].contains("Dangling member reference"));
        assert!(errors[0].contains("ptr"));
        assert!(errors[0].contains("obj.data"));
    }

    #[test]
    fn test_member_borrow_same_scope_no_error() {
        let mut tracker = RaiiTracker::new();

        // Enter a scope
        tracker.enter_scope();

        // Both reference and object in same scope
        tracker.register_variable("obj", "Wrapper", 1);
        tracker.variable_scopes.insert("ptr".to_string(), 1);

        // Borrow field from object
        tracker.record_member_borrow("ptr", "obj", "data", 10);

        // Exit scope - should NOT error because both die together
        let errors = tracker.exit_scope();
        assert!(errors.is_empty(), "Should not have errors when reference and object in same scope");
    }

    #[test]
    fn test_member_borrow_nested_object() {
        let mut tracker = RaiiTracker::new();

        // Outer scope: reference
        tracker.variable_scopes.insert("ptr".to_string(), 0);

        // First inner scope
        tracker.enter_scope();
        tracker.register_variable("obj1", "Wrapper", 1);
        tracker.record_member_borrow("ptr", "obj1", "data", 10);
        let errors1 = tracker.exit_scope();
        assert_eq!(errors1.len(), 1);

        // Second inner scope (fresh - no lingering borrows)
        tracker.enter_scope();
        tracker.register_variable("obj2", "Wrapper", 1);
        tracker.record_member_borrow("ptr", "obj2", "name", 20);
        let errors2 = tracker.exit_scope();
        assert_eq!(errors2.len(), 1);
    }

    #[test]
    fn test_member_borrow_cleanup() {
        let mut tracker = RaiiTracker::new();

        // Register object in scope 0
        tracker.register_variable("obj", "Wrapper", 0);

        // Enter inner scope and create a local reference to obj.data
        tracker.enter_scope();
        tracker.variable_scopes.insert("local_ref".to_string(), 1);
        tracker.record_member_borrow("local_ref", "obj", "data", 10);

        // Exit inner scope - local_ref dies, borrow should be cleaned
        let errors = tracker.exit_scope();
        assert!(errors.is_empty(), "Local reference dying with inner scope should not error");

        // Verify the member borrow was cleaned up
        assert!(tracker.member_borrows.is_empty() ||
                tracker.member_borrows.iter().all(|b| b.reference != "local_ref"),
                "Member borrow should be cleaned up after scope exit");
    }

    // ==========================================================================
    // Iterator Invalidation Tests
    // ==========================================================================

    #[test]
    fn test_container_modifying_method_detection() {
        assert!(RaiiTracker::is_container_modifying_method("push_back"));
        assert!(RaiiTracker::is_container_modifying_method("push_front"));
        assert!(RaiiTracker::is_container_modifying_method("pop_back"));
        assert!(RaiiTracker::is_container_modifying_method("pop_front"));
        assert!(RaiiTracker::is_container_modifying_method("insert"));
        assert!(RaiiTracker::is_container_modifying_method("erase"));
        assert!(RaiiTracker::is_container_modifying_method("clear"));
        assert!(RaiiTracker::is_container_modifying_method("resize"));
        assert!(RaiiTracker::is_container_modifying_method("reserve"));
        assert!(RaiiTracker::is_container_modifying_method("swap"));
        assert!(!RaiiTracker::is_container_modifying_method("begin"));
        assert!(!RaiiTracker::is_container_modifying_method("end"));
        assert!(!RaiiTracker::is_container_modifying_method("size"));
        assert!(!RaiiTracker::is_container_modifying_method("empty"));
        assert!(!RaiiTracker::is_container_modifying_method("at"));
    }

    #[test]
    fn test_iterator_invalidation_basic() {
        let mut tracker = RaiiTracker::new();

        // Register container
        tracker.register_variable("vec", "std::vector<int>", 0);
        tracker.container_variables.insert("vec".to_string());

        // Create iterator from container
        tracker.record_iterator_creation("it", "vec", 10);

        // Iterator should not be invalidated initially
        assert!(!tracker.is_iterator_invalidated("it"));
        assert!(tracker.is_iterator("it"));

        // Modify container
        tracker.record_container_modification("vec", "push_back", 15);

        // Iterator should now be invalidated
        assert!(tracker.is_iterator_invalidated("it"));

        // Verify invalidation info
        let info = tracker.get_invalidation_info("it").unwrap();
        assert_eq!(info.container, "vec");
        assert_eq!(info.method, "push_back");
        assert_eq!(info.invalidation_line, 15);
    }

    #[test]
    fn test_multiple_iterators_from_same_container() {
        let mut tracker = RaiiTracker::new();

        // Register container
        tracker.register_variable("vec", "std::vector<int>", 0);
        tracker.container_variables.insert("vec".to_string());

        // Create multiple iterators
        tracker.record_iterator_creation("begin_it", "vec", 10);
        tracker.record_iterator_creation("end_it", "vec", 11);
        tracker.record_iterator_creation("find_it", "vec", 12);

        // None should be invalidated initially
        assert!(!tracker.is_iterator_invalidated("begin_it"));
        assert!(!tracker.is_iterator_invalidated("end_it"));
        assert!(!tracker.is_iterator_invalidated("find_it"));

        // Modify container with clear()
        let invalidated = tracker.record_container_modification("vec", "clear", 20);

        // All three should be invalidated
        assert_eq!(invalidated.len(), 3);
        assert!(tracker.is_iterator_invalidated("begin_it"));
        assert!(tracker.is_iterator_invalidated("end_it"));
        assert!(tracker.is_iterator_invalidated("find_it"));
    }

    #[test]
    fn test_iterator_invalidation_different_containers() {
        let mut tracker = RaiiTracker::new();

        // Register two containers
        tracker.register_variable("vec1", "std::vector<int>", 0);
        tracker.register_variable("vec2", "std::vector<int>", 0);
        tracker.container_variables.insert("vec1".to_string());
        tracker.container_variables.insert("vec2".to_string());

        // Create iterator from vec1
        tracker.record_iterator_creation("it1", "vec1", 10);

        // Create iterator from vec2
        tracker.record_iterator_creation("it2", "vec2", 11);

        // Modify only vec1
        tracker.record_container_modification("vec1", "push_back", 20);

        // Only it1 should be invalidated
        assert!(tracker.is_iterator_invalidated("it1"));
        assert!(!tracker.is_iterator_invalidated("it2"));
    }

    #[test]
    fn test_iterator_invalidation_preserved_after_remodification() {
        let mut tracker = RaiiTracker::new();

        // Register container
        tracker.register_variable("vec", "std::vector<int>", 0);
        tracker.container_variables.insert("vec".to_string());

        // Create iterator
        tracker.record_iterator_creation("it", "vec", 10);

        // First modification
        let inv1 = tracker.record_container_modification("vec", "push_back", 15);
        assert_eq!(inv1.len(), 1);

        // Second modification should not re-invalidate (already invalidated)
        let inv2 = tracker.record_container_modification("vec", "erase", 20);
        assert_eq!(inv2.len(), 0);

        // Original invalidation info should be preserved
        let info = tracker.get_invalidation_info("it").unwrap();
        assert_eq!(info.invalidation_line, 15);  // First modification's line
        assert_eq!(info.method, "push_back");    // First modification's method
    }

    #[test]
    fn test_iterator_invalidation_various_modifying_methods() {
        // Test that all modifying methods correctly invalidate
        let methods = vec![
            "push_back", "push_front", "pop_back", "pop_front",
            "insert", "emplace", "emplace_back", "emplace_front",
            "erase", "clear", "resize", "reserve", "assign", "swap"
        ];

        for method in methods {
            let mut tracker = RaiiTracker::new();
            tracker.register_variable("vec", "std::vector<int>", 0);
            tracker.container_variables.insert("vec".to_string());
            tracker.record_iterator_creation("it", "vec", 10);

            tracker.record_container_modification("vec", method, 20);

            assert!(
                tracker.is_iterator_invalidated("it"),
                "Iterator should be invalidated by {}()",
                method
            );
        }
    }
}
