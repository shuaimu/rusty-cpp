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

/// Test that template-dependent member function calls (like values.size()) are NOT
/// reported as "unknown" undeclared functions in free template functions.
/// This tests the fix for extracting template parameters from FunctionTemplate entities.
#[test]
fn test_template_dependent_member_calls_in_free_functions() {
    let code = r#"
#include <vector>

class Value {};

template<class Container>
// @safe
void process_container(const Container& values) {
    // values.size() is a template-dependent member function call
    // The parser can't resolve "size" because Container is a template parameter
    // But this should NOT be reported as "calling undeclared function 'unknown'"
    std::vector<const Value*> values_ptr(values.size(), nullptr);
}
"#;

    let (output, _) = run_checker(code);

    // Should NOT report "unknown" function errors for template-dependent calls
    // in free template functions
    assert!(
        !output.contains("Calling unsafe function 'unknown") &&
        !output.contains("Calling undeclared function 'unknown"),
        "Template-dependent member calls in free template functions should not cause 'unknown' errors. Output: {}",
        output
    );
}

/// Test that returning a reference ALIAS is NOT flagged as "returning reference to local variable"
/// When we have `T& value = some_function();` and `return value;`, the `value` is a reference
/// alias, not a local object. It's safe to return because it inherits the lifetime of whatever
/// it was bound to.
#[test]
fn test_reference_alias_return_not_flagged_as_local() {
    let code = r#"
class Node {
public:
    int data;
};

class Container {
    Node m_node;
public:
    // @safe
    Node& get() {
        return m_node;  // Safe: returning reference to member
    }
};

// @safe
Node& get_node_alias(Container& c) {
    // 'value' is a REFERENCE ALIAS, not a local object
    // Returning it should NOT be flagged as "returning reference to local variable"
    Node& value = c.get();
    return value;  // This is safe - value is just an alias for c.m_node
}
"#;

    let (output, _) = run_checker(code);

    // Should NOT report "returning reference to local variable" for reference aliases
    assert!(
        !output.contains("return reference to local variable") &&
        !output.contains("Returning reference to local variable") &&
        !output.contains("Cannot return reference to local variable"),
        "Reference alias return should not be flagged as returning local. Output: {}",
        output
    );
}

/// Test that unqualified function names don't match qualified annotations
/// This tests the fix for yaml-cpp's `get` being incorrectly matched to `rusty::Cell::get`
/// when both are unrelated functions in different namespaces.
#[test]
fn test_unqualified_get_does_not_match_qualified_annotation() {
    let code = r#"
// Simulating a user-defined class with a @safe get method
namespace rusty {
    class Cell {
    public:
        // @safe
        int get() const {
            return 0;
        }
    };
}

// Simulating an external library function with an unqualified get
// (like yaml-cpp's node_data::get)
namespace external_lib {
    // No annotation - undeclared
    class Node {
    public:
        int get() const {
            return 0;
        }
    };
}

// Undeclared function (can call other undeclared functions)
void use_external_lib() {
    external_lib::Node node;
    int x = node.get();  // Should NOT match rusty::Cell::get
}
"#;

    let (output, success) = run_checker(code);

    // The undeclared function use_external_lib() can call external_lib::Node::get()
    // because undeclared can call undeclared.
    // The key point is that external_lib::Node::get should NOT be treated as @safe
    // just because rusty::Cell::get is @safe.
    assert!(
        success || !output.contains("rusty::Cell::get"),
        "Unqualified external 'get' should not match 'rusty::Cell::get'. Output: {}",
        output
    );
}

/// Test that functions in different namespaces with the same simple name
/// are correctly distinguished when one is annotated and one is not.
#[test]
fn test_same_name_different_namespace_no_collision() {
    let code = r#"
namespace safe_ns {
    // @safe
    int helper() {
        return 1;
    }
}

namespace undeclared_ns {
    // No annotation - undeclared
    int helper() {
        return 2;
    }
}

// @safe
void safe_caller_to_safe() {
    int x = safe_ns::helper();  // OK - calling @safe function
}

// @safe
void safe_caller_to_undeclared() {
    int x = undeclared_ns::helper();  // ERROR - @safe calling undeclared
}
"#;

    let (output, _) = run_checker(code);

    // Should detect safe calling undeclared for undeclared_ns::helper
    // The key test: it should identify the CORRECT namespace
    assert!(
        output.contains("undeclared_ns::helper"),
        "Should detect @safe calling undeclared 'undeclared_ns::helper'. Output: {}",
        output
    );

    // Make sure we don't flag the call to safe_ns::helper (which is @safe)
    // Note: 'safe_ns::helper' should not appear in error messages
    assert!(
        !output.contains("safe_ns::helper"),
        "Should NOT flag call to safe_ns::helper (which is @safe). Output: {}",
        output
    );
}
