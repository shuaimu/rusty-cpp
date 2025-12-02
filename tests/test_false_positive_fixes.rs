/// Tests for false positive fixes in hard-coded pattern detection
///
/// This test suite ensures that overly broad string matching doesn't
/// cause false positives in move detection, operator detection, etc.

use std::fs;
use std::io::Write;
use std::process::Command;
use tempfile::NamedTempFile;

fn run_checker(code: &str) -> (String, bool) {
    let mut file = NamedTempFile::new().unwrap();
    file.write_all(code.as_bytes()).unwrap();
    file.flush().unwrap();

    let output = Command::new("cargo")
        .args(&["run", "--", file.path().to_str().unwrap()])
        .output()
        .expect("Failed to run rusty-cpp-checker");

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let combined = format!("{}\n{}", stdout, stderr);

    (combined, output.status.success())
}

/// Test that std::remove is NOT detected as std::move
#[test]
fn test_std_remove_not_detected_as_move() {
    let code = r#"
#include <algorithm>
#include <vector>

// @safe
void test_remove() {
    std::vector<int> vec = {1, 2, 3, 4, 5};
    int x = 42;

    // std::remove should NOT be detected as std::move
    auto new_end = std::remove(vec.begin(), vec.end(), 3);

    // Using x after remove should be fine - it wasn't moved
    int y = x;  // Should be OK
}
"#;

    let (output, success) = run_checker(code);

    // Should NOT report use-after-move for x
    assert!(
        !output.contains("Use after move") && !output.contains("has been moved"),
        "std::remove should not be detected as std::move. Output: {}",
        output
    );
}

/// Test that std::remove_if is NOT detected as std::move
#[test]
fn test_std_remove_if_not_detected_as_move() {
    let code = r#"
#include <algorithm>
#include <vector>

// @safe
void test_remove_if() {
    std::vector<int> vec = {1, 2, 3, 4, 5};
    int x = 10;

    // std::remove_if should NOT be detected as std::move
    auto new_end = std::remove_if(vec.begin(), vec.end(),
        [](int n) { return n > 3; });

    // Using x should be fine
    int y = x;  // Should be OK
}
"#;

    let (output, success) = run_checker(code);

    assert!(
        !output.contains("Use after move") && !output.contains("has been moved"),
        "std::remove_if should not be detected as std::move. Output: {}",
        output
    );
}

/// Test that custom functions with "move" in the name are NOT detected as std::move
#[test]
fn test_custom_move_functions_not_detected() {
    let code = r#"
// Custom functions that contain "move" but are not std::move
void movement_tracker(int x) {}
void remove_item(int x) {}
void my_move_function(int x) {}

// @safe
void test_custom_move_names() {
    int x = 42;

    // These should NOT be detected as moves
    movement_tracker(x);
    remove_item(x);
    my_move_function(x);

    // Using x should still be fine
    int y = x;  // Should be OK
}
"#;

    let (output, success) = run_checker(code);

    assert!(
        !output.contains("Use after move") && !output.contains("has been moved"),
        "Custom functions with 'move' in name should not be detected as std::move. Output: {}",
        output
    );
}

/// Test that operator*= is NOT detected as operator* (dereference)
#[test]
fn test_operator_multiply_assign_not_dereference() {
    let code = r#"
class Number {
public:
    int value;

    Number(int v) : value(v) {}

    // operator*= should NOT be detected as operator* (dereference)
    Number& operator*=(int factor) {
        value *= factor;
        return *this;
    }
};

// @safe
void test_multiply_assign() {
    Number n(10);
    int& ref = n.value;  // Immutable borrow

    // operator*= should NOT create a mutable borrow (it's not dereference)
    n *= 5;  // This is operator*=, not operator*

    // Using ref should be fine if operator*= wasn't detected as dereference
    // (Note: This might still fail for other reasons like method calls,
    //  but it shouldn't fail due to operator* detection)
}
"#;

    let (output, _) = run_checker(code);

    // Should NOT complain about operator* (dereference) for operator*=
    // The error message would typically mention dereference if it was confused
    assert!(
        !output.contains("dereference") || output.contains("operator*="),
        "operator*= should not be confused with operator* (dereference). Output: {}",
        output
    );
}

/// Test that operator== is NOT detected as operator= (assignment)
#[test]
fn test_operator_equals_not_assignment() {
    let code = r#"
class Value {
public:
    int data;

    Value(int d) : data(d) {}

    // operator== should NOT be detected as operator= (assignment)
    bool operator==(const Value& other) const {
        return data == other.data;
    }
};

// @safe
void test_equality() {
    Value v1(10);
    Value v2(10);

    // operator== should NOT be detected as assignment operator
    bool same = (v1 == v2);

    // Both v1 and v2 should still be usable
    int x = v1.data;
    int y = v2.data;
}
"#;

    let (output, _) = run_checker(code);

    // Should NOT report assignment-related errors for operator==
    // Check that it doesn't confuse == with =
    assert!(
        !output.contains("operator=") || output.contains("operator=="),
        "operator== should not be confused with operator=. Output: {}",
        output
    );
}

/// Test that operator!= is NOT detected as operator=
#[test]
fn test_operator_not_equals_not_assignment() {
    let code = r#"
class Value {
public:
    int data;

    Value(int d) : data(d) {}

    bool operator!=(const Value& other) const {
        return data != other.data;
    }
};

// @safe
void test_inequality() {
    Value v1(10);
    Value v2(20);

    bool different = (v1 != v2);

    int x = v1.data;
    int y = v2.data;
}
"#;

    let (output, _) = run_checker(code);

    assert!(
        !output.contains("operator=") || output.contains("operator!="),
        "operator!= should not be confused with operator=. Output: {}",
        output
    );
}

/// Test that operator<= and operator>= are NOT detected as operator=
#[test]
fn test_operator_comparison_not_assignment() {
    let code = r#"
class Value {
public:
    int data;

    Value(int d) : data(d) {}

    bool operator<=(const Value& other) const {
        return data <= other.data;
    }

    bool operator>=(const Value& other) const {
        return data >= other.data;
    }
};

// @safe
void test_comparisons() {
    Value v1(10);
    Value v2(20);

    bool less_equal = (v1 <= v2);
    bool greater_equal = (v1 >= v2);

    int x = v1.data;
    int y = v2.data;
}
"#;

    let (output, _) = run_checker(code);

    assert!(
        !output.contains("operator=") ||
        output.contains("operator<=") ||
        output.contains("operator>="),
        "operator<= and operator>= should not be confused with operator=. Output: {}",
        output
    );
}

/// Test that std::forward is correctly detected (not a false positive test, but ensures precision)
#[test]
fn test_std_forward_detection_is_precise() {
    let code = r#"
#include <utility>

template<typename T>
void forwarder(T&& arg) {
    // Real std::forward - should be detected
}

// Function with "forward" in name but not std::forward
void forward_request(int x) {}
void go_forward(int x) {}

// @safe
void test_forward_precision() {
    int x = 42;

    // These should NOT be detected as std::forward
    forward_request(x);
    go_forward(x);

    // x should still be usable
    int y = x;  // Should be OK
}
"#;

    let (output, _) = run_checker(code);

    assert!(
        !output.contains("Use after move") && !output.contains("has been moved"),
        "Functions with 'forward' in name should not be detected as std::forward. Output: {}",
        output
    );
}

/// Integration test: multiple potential false positives in one function
#[test]
fn test_multiple_false_positives_together() {
    let code = r#"
#include <algorithm>
#include <vector>

class Counter {
public:
    int count;

    Counter(int c) : count(c) {}

    Counter& operator*=(int factor) {
        count *= factor;
        return *this;
    }

    bool operator==(const Counter& other) const {
        return count == other.count;
    }
};

void movement_tracker(int x) {}

// @safe
void test_combined() {
    std::vector<int> vec = {1, 2, 3, 4, 5};
    int x = 10;
    Counter c1(5);
    Counter c2(5);

    // std::remove - not a move
    auto new_end = std::remove(vec.begin(), vec.end(), 3);

    // movement_tracker - not std::move
    movement_tracker(x);

    // operator*= - not dereference
    c1 *= 2;

    // operator== - not assignment
    bool same = (c1 == c2);

    // All variables should still be usable
    int y = x;
    int z = c1.count;
}
"#;

    let (output, _) = run_checker(code);

    // Should not report any false positive errors
    assert!(
        !output.contains("Use after move") || output.contains("std::move"),
        "Multiple operations should not trigger false positives. Output: {}",
        output
    );
}

/// Test actual std::move IS still detected (regression test)
#[test]
fn test_real_std_move_still_detected() {
    let code = r#"
#include <utility>

// @safe
void test_real_move() {
    int x = 42;
    int y = std::move(x);  // Real move
    int z = x;  // Should error: use after move
}
"#;

    let (output, success) = run_checker(code);

    // SHOULD report use-after-move
    assert!(
        output.contains("Use after move") || output.contains("has been moved"),
        "Real std::move should still be detected. Output: {}",
        output
    );
}

/// Test that function parameter names are NOT detected as undeclared function calls
/// This tests the namespace collision fix where template function arguments
/// like `rhs`, `schema`, `vv` were incorrectly being treated as function names.
#[test]
fn test_parameter_names_not_detected_as_function_calls() {
    let code = r#"
// Helper class with a method that takes a parameter
class Data {
public:
    int value;

    // @safe
    void Assign(const Data& rhs) {
        value = rhs.value;
    }
};

// Template function that forwards a parameter to a method
template<typename T>
// @safe
void assign_helper(T& dest, const T& src) {
    // The parameter `src` should NOT be detected as a function call
    dest.Assign(src);
}

// Function with common parameter names that caused false positives
// @safe
void test_with_common_names(int schema, int vv, int rhs) {
    // These parameter names should NOT cause "undeclared function" errors
    int x = schema;
    int y = vv;
    int z = rhs;
}
"#;

    let (output, _) = run_checker(code);

    // Should NOT report undeclared function errors for parameter names
    // The old bug would report things like "Calling undeclared function 'rhs'"
    assert!(
        !output.contains("Calling undeclared function 'rhs'") &&
        !output.contains("Calling undeclared function 'schema'") &&
        !output.contains("Calling undeclared function 'vv'") &&
        !output.contains("Calling undeclared function 'src'"),
        "Parameter names should not be detected as function calls. Output: {}",
        output
    );
}

/// Test that template-dependent function calls still work correctly
/// after the namespace collision fix (regression test)
#[test]
fn test_template_dependent_calls_still_work() {
    let code = r#"
#include <utility>

template<typename T>
// @safe
T process(T x) {
    // std::move should still be detected as a function call
    T moved = std::move(x);
    return moved;
}
"#;

    let (output, _) = run_checker(code);

    // The template function should still be analyzed
    // std::move should be detected (it's in the whitelist, so no error expected)
    // But importantly, it should NOT cause crashes or incorrect parsing
    assert!(
        !output.contains("Parse error") && !output.contains("panicked"),
        "Template-dependent calls should parse correctly. Output: {}",
        output
    );
}
