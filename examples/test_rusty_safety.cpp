// Test that Rusty types properly enforce safety rules
// This file contains deliberate errors to test the checker

#include "rusty/rusty.hpp"

extern "C" int printf(const char*, ...);

// @safe
namespace safety_tests {

// Test 1: Box use-after-move (should error)
// @safe
void test_box_use_after_move() {
    auto box1 = rusty::make_box<int>(42);
    auto box2 = std::move(box1);
    
    // ERROR: Use after move
    int value = *box1;  // This should be caught
    printf("Value: %d\n", value);
}

// Test 2: Vec use-after-move (should error)
// @safe
void test_vec_use_after_move() {
    rusty::Vec<int> vec;
    vec.push(10);
    vec.push(20);
    
    auto vec2 = std::move(vec);
    
    // ERROR: Use after move
    vec.push(30);  // This should be caught
}

// Test 3: Multiple mutable references via get_mut (should be safe)
// @safe
void test_rc_get_mut_safety() {
    auto rc1 = rusty::make_rc<int>(100);

    // Get mutable reference when unique - OK
    if (auto opt = rc1.get_mut(); opt.is_some()) {
        opt.unwrap() = 200;
    }

    // Create another reference
    auto rc2 = rc1.clone();

    // Try to get mutable when shared - returns None, safe
    if (auto opt = rc1.get_mut(); opt.is_some()) {
        opt.unwrap() = 300;  // This won't execute
    }
}

// Test 4: Option unwrap on None (runtime error, but type-safe)
// @safe
void test_option_unwrap_none() {
    rusty::Option<int> none = rusty::None;
    
    // This compiles but will throw at runtime
    // The type system ensures we handle Option explicitly
    if (none.is_some()) {
        int value = none.unwrap();  // Safe path
        printf("Value: %d\n", value);
    } else {
        // Forced to handle None case
        printf("No value\n");
    }
}

// Test 5: Result error propagation
// @safe
rusty::Result<int, const char*> may_fail(bool fail) {
    if (fail) {
        return rusty::Err<int, const char*>("Operation failed");
    }
    return rusty::Ok<int, const char*>(42);
}

// @safe
void test_result_propagation() {
    auto result = may_fail(true)
        .map([](int x) { return x * 2; })
        .and_then([](int x) { return may_fail(false); });
    
    // Forced to check result
    if (result.is_err()) {
        printf("Error: %s\n", result.unwrap_err());
    }
}

// Test 6: Arc prevents mutable access
// @safe
void test_arc_immutability() {
    auto arc1 = rusty::make_arc<int>(50);
    auto arc2 = arc1.clone();
    
    // Can only get const references
    const int& ref1 = *arc1;
    const int& ref2 = *arc2;
    
    // Cannot get mutable reference (by design)
    // int& mut_ref = *arc1;  // Compile error: no non-const operator*
    
    printf("Values: %d, %d\n", ref1, ref2);
}

// Test 7: Box double-free prevention
// @safe
void test_box_no_double_free() {
    rusty::Box<int> box1 = rusty::make_box<int>(100);
    
    // Release ownership
    int* raw = box1.release();
    
    // Box is now empty, destructor won't double-free
    // box1 is empty but still valid object
    
    // Manual cleanup required for released pointer
    delete raw;
    
    // This is safe - box1's destructor handles empty state
}

// Test 8: Vec bounds checking
// @safe
void test_vec_bounds() {
    rusty::Vec<int> vec;
    vec.push(1);
    vec.push(2);
    
    // Safe access
    int first = vec[0];
    int second = vec[1];
    
    // This would assert/crash at runtime (bounds check)
    // int third = vec[2];  // Runtime error, not compile-time
    
    printf("Elements: %d, %d\n", first, second);
}

} // namespace safety_tests

int main() {
    printf("Testing Rusty type safety...\n");
    
    // Comment out tests with deliberate errors for normal runs
    // safety_tests::test_box_use_after_move();  // ERROR
    // safety_tests::test_vec_use_after_move();  // ERROR
    
    // These should run safely
    safety_tests::test_rc_get_mut_safety();
    safety_tests::test_option_unwrap_none();
    safety_tests::test_result_propagation();
    safety_tests::test_arc_immutability();
    safety_tests::test_box_no_double_free();
    safety_tests::test_vec_bounds();
    
    printf("Safe tests completed\n");
    return 0;
}