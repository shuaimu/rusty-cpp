// Comprehensive tests for rusty::Function<Sig>
//
// Tests cover:
// - Basic callable storage (function pointers, lambdas)
// - Move-only captures (Box, unique_ptr)
// - Small buffer optimization (SBO)
// - Move semantics
// - Empty state handling
// - Const-qualified signatures

#include "../include/rusty/function.hpp"
#include "../include/rusty/box.hpp"
#include "../include/rusty/arc.hpp"
#include <cassert>
#include <cstdio>
#include <memory>
#include <string>
#include <vector>

using namespace rusty;

// ============================================================================
// Test 1: Basic Function Pointer
// ============================================================================
int add(int a, int b) { return a + b; }
int multiply(int a, int b) { return a * b; }

void test_function_pointer() {
    printf("test_function_pointer: ");

    Function<int(int, int)> fn = add;
    assert(fn);
    assert(fn(2, 3) == 5);

    fn = multiply;
    assert(fn(4, 5) == 20);

    printf("PASS\n");
}

// ============================================================================
// Test 2: Lambda Without Capture
// ============================================================================
void test_lambda_no_capture() {
    printf("test_lambda_no_capture: ");

    Function<int(int)> fn = [](int x) { return x * x; };
    assert(fn);
    assert(fn(5) == 25);
    assert(fn(7) == 49);

    printf("PASS\n");
}

// ============================================================================
// Test 3: Lambda With Copy Capture
// ============================================================================
void test_lambda_copy_capture() {
    printf("test_lambda_copy_capture: ");

    int multiplier = 10;
    Function<int(int)> fn = [multiplier](int x) { return x * multiplier; };

    assert(fn);
    assert(fn(3) == 30);
    assert(fn(7) == 70);

    printf("PASS\n");
}

// ============================================================================
// Test 4: Lambda With Reference Capture
// ============================================================================
void test_lambda_ref_capture() {
    printf("test_lambda_ref_capture: ");

    int counter = 0;
    Function<void()> fn = [&counter]() { counter++; };

    assert(fn);
    fn();
    assert(counter == 1);
    fn();
    assert(counter == 2);

    printf("PASS\n");
}

// ============================================================================
// Test 5: Move-Only Capture with unique_ptr
// ============================================================================
void test_move_only_unique_ptr() {
    printf("test_move_only_unique_ptr: ");

    auto ptr = std::make_unique<int>(42);

    // Lambda captures unique_ptr by move - this wouldn't work with std::function!
    Function<int()> fn = [p = std::move(ptr)]() { return *p; };

    assert(fn);
    assert(fn() == 42);

    printf("PASS\n");
}

// ============================================================================
// Test 6: Move-Only Capture with rusty::Box
// ============================================================================
void test_move_only_box() {
    printf("test_move_only_box: ");

    auto box = Box<int>::make(100);

    Function<int()> fn = [b = std::move(box)]() { return *b; };

    assert(fn);
    assert(fn() == 100);

    printf("PASS\n");
}

// ============================================================================
// Test 7: Move-Only Capture with rusty::Arc
// ============================================================================
void test_move_only_arc() {
    printf("test_move_only_arc: ");

    auto arc = Arc<int>::make(200);

    // Arc can be cloned, but we test move capture
    Function<int()> fn = [a = std::move(arc)]() { return *a; };

    assert(fn);
    assert(fn() == 200);

    printf("PASS\n");
}

// ============================================================================
// Test 8: Multiple Move-Only Captures
// ============================================================================
void test_multiple_move_only() {
    printf("test_multiple_move_only: ");

    auto ptr1 = std::make_unique<int>(10);
    auto ptr2 = std::make_unique<int>(20);

    Function<int()> fn = [p1 = std::move(ptr1), p2 = std::move(ptr2)]() {
        return *p1 + *p2;
    };

    assert(fn);
    assert(fn() == 30);

    printf("PASS\n");
}

// ============================================================================
// Test 9: Move Semantics - Move Constructor
// ============================================================================
void test_move_constructor() {
    printf("test_move_constructor: ");

    auto ptr = std::make_unique<int>(42);
    Function<int()> fn1 = [p = std::move(ptr)]() { return *p; };

    assert(fn1);

    // Move construct fn2 from fn1
    Function<int()> fn2 = std::move(fn1);

    assert(!fn1);  // fn1 should be empty after move
    assert(fn2);   // fn2 should have the callable
    assert(fn2() == 42);

    printf("PASS\n");
}

// ============================================================================
// Test 10: Move Semantics - Move Assignment
// ============================================================================
void test_move_assignment() {
    printf("test_move_assignment: ");

    Function<int()> fn1 = []() { return 100; };
    Function<int()> fn2 = []() { return 200; };

    assert(fn1() == 100);
    assert(fn2() == 200);

    fn2 = std::move(fn1);

    assert(!fn1);  // fn1 should be empty
    assert(fn2);
    assert(fn2() == 100);  // fn2 now has fn1's callable

    printf("PASS\n");
}

// ============================================================================
// Test 11: Empty Function State
// ============================================================================
void test_empty_function() {
    printf("test_empty_function: ");

    Function<int()> fn;
    assert(!fn);
    assert(fn.is_empty());
    assert(fn == nullptr);

    // Assign nullptr explicitly
    fn = nullptr;
    assert(!fn);

    printf("PASS\n");
}

// ============================================================================
// Test 12: Assign Nullptr to Clear
// ============================================================================
void test_assign_nullptr() {
    printf("test_assign_nullptr: ");

    Function<int()> fn = []() { return 42; };
    assert(fn);
    assert(fn() == 42);

    fn = nullptr;
    assert(!fn);
    assert(fn.is_empty());

    printf("PASS\n");
}

// ============================================================================
// Test 13: Small Buffer Optimization (SBO)
// ============================================================================
void test_sbo() {
    printf("test_sbo: ");

    // Small lambda should use SBO
    int x = 10;
    Function<int()> small_fn = [x]() { return x; };
    assert(small_fn);
    assert(small_fn.is_inline());
    assert(small_fn() == 10);

    // Function pointer should use SBO
    Function<int(int, int)> fn_ptr = add;
    assert(fn_ptr.is_inline());

    printf("PASS\n");
}

// ============================================================================
// Test 14: Large Callable (Heap Allocation)
// ============================================================================
void test_heap_allocation() {
    printf("test_heap_allocation: ");

    // Large lambda that exceeds SBO size
    std::string s1 = "hello";
    std::string s2 = "world";
    std::string s3 = "test";
    std::string s4 = "large";

    Function<std::string()> fn = [s1, s2, s3, s4]() {
        return s1 + " " + s2 + " " + s3 + " " + s4;
    };

    assert(fn);
    // This may or may not be inline depending on string SSO
    std::string result = fn();
    assert(result == "hello world test large");

    printf("PASS\n");
}

// ============================================================================
// Test 15: Const Signature
// ============================================================================
void test_const_signature() {
    printf("test_const_signature: ");

    int value = 42;
    Function<int() const> fn = [value]() { return value; };

    assert(fn);

    // Can call const Function from const context
    const Function<int() const>& const_ref = fn;
    assert(const_ref() == 42);

    printf("PASS\n");
}

// ============================================================================
// Test 16: Void Return Type
// ============================================================================
void test_void_return() {
    printf("test_void_return: ");

    int counter = 0;
    Function<void()> fn = [&counter]() { counter++; };

    assert(fn);
    fn();
    fn();
    fn();
    assert(counter == 3);

    printf("PASS\n");
}

// ============================================================================
// Test 17: Multiple Arguments
// ============================================================================
void test_multiple_args() {
    printf("test_multiple_args: ");

    Function<int(int, int, int, int)> fn = [](int a, int b, int c, int d) {
        return a + b + c + d;
    };

    assert(fn);
    assert(fn(1, 2, 3, 4) == 10);

    printf("PASS\n");
}

// ============================================================================
// Test 18: String Arguments
// ============================================================================
void test_string_args() {
    printf("test_string_args: ");

    Function<std::string(const std::string&, const std::string&)> fn =
        [](const std::string& a, const std::string& b) {
            return a + b;
        };

    assert(fn);
    assert(fn("hello", "world") == "helloworld");

    printf("PASS\n");
}

// ============================================================================
// Test 19: Swap Functions
// ============================================================================
void test_swap() {
    printf("test_swap: ");

    Function<int()> fn1 = []() { return 1; };
    Function<int()> fn2 = []() { return 2; };

    assert(fn1() == 1);
    assert(fn2() == 2);

    fn1.swap(fn2);

    assert(fn1() == 2);
    assert(fn2() == 1);

    // Also test non-member swap
    swap(fn1, fn2);

    assert(fn1() == 1);
    assert(fn2() == 2);

    printf("PASS\n");
}

// ============================================================================
// Test 20: Reassignment
// ============================================================================
void test_reassignment() {
    printf("test_reassignment: ");

    Function<int()> fn = []() { return 1; };
    assert(fn() == 1);

    fn = []() { return 2; };
    assert(fn() == 2);

    fn = []() { return 3; };
    assert(fn() == 3);

    printf("PASS\n");
}

// ============================================================================
// Test 21: Callable Object (Functor)
// ============================================================================
struct Adder {
    int offset;
    Adder(int o) : offset(o) {}
    int operator()(int x) const { return x + offset; }
};

void test_functor() {
    printf("test_functor: ");

    Function<int(int)> fn = Adder(100);
    assert(fn);
    assert(fn(5) == 105);
    assert(fn(10) == 110);

    printf("PASS\n");
}

// ============================================================================
// Test 22: Move-Only Functor
// ============================================================================
struct MoveOnlyFunctor {
    std::unique_ptr<int> value;

    MoveOnlyFunctor(int v) : value(std::make_unique<int>(v)) {}
    MoveOnlyFunctor(MoveOnlyFunctor&&) = default;
    MoveOnlyFunctor& operator=(MoveOnlyFunctor&&) = default;

    // Delete copy
    MoveOnlyFunctor(const MoveOnlyFunctor&) = delete;
    MoveOnlyFunctor& operator=(const MoveOnlyFunctor&) = delete;

    int operator()() const { return *value; }
};

void test_move_only_functor() {
    printf("test_move_only_functor: ");

    Function<int()> fn = MoveOnlyFunctor(42);
    assert(fn);
    assert(fn() == 42);

    printf("PASS\n");
}

// ============================================================================
// Test 23: Comparison Operators
// ============================================================================
void test_comparison() {
    printf("test_comparison: ");

    Function<int()> fn1;
    Function<int()> fn2 = []() { return 42; };

    assert(fn1 == nullptr);
    assert(nullptr == fn1);
    assert(fn2 != nullptr);
    assert(nullptr != fn2);

    fn2 = nullptr;
    assert(fn2 == nullptr);

    printf("PASS\n");
}

// ============================================================================
// Test 24: Vector of Functions
// ============================================================================
void test_vector_of_functions() {
    printf("test_vector_of_functions: ");

    std::vector<Function<int(int)>> fns;

    fns.push_back([](int x) { return x + 1; });
    fns.push_back([](int x) { return x * 2; });
    fns.push_back([](int x) { return x * x; });

    assert(fns[0](5) == 6);
    assert(fns[1](5) == 10);
    assert(fns[2](5) == 25);

    printf("PASS\n");
}

// ============================================================================
// Test 25: Move Function in Vector
// ============================================================================
void test_move_in_vector() {
    printf("test_move_in_vector: ");

    auto ptr = std::make_unique<int>(42);
    Function<int()> fn = [p = std::move(ptr)]() { return *p; };

    std::vector<Function<int()>> vec;
    vec.push_back(std::move(fn));

    assert(!fn);  // fn is empty after move
    assert(vec[0]);
    assert(vec[0]() == 42);

    printf("PASS\n");
}

// ============================================================================
// Main
// ============================================================================
int main() {
    printf("========================================\n");
    printf("rusty::Function Test Suite\n");
    printf("========================================\n\n");

    test_function_pointer();
    test_lambda_no_capture();
    test_lambda_copy_capture();
    test_lambda_ref_capture();
    test_move_only_unique_ptr();
    test_move_only_box();
    test_move_only_arc();
    test_multiple_move_only();
    test_move_constructor();
    test_move_assignment();
    test_empty_function();
    test_assign_nullptr();
    test_sbo();
    test_heap_allocation();
    test_const_signature();
    test_void_return();
    test_multiple_args();
    test_string_args();
    test_swap();
    test_reassignment();
    test_functor();
    test_move_only_functor();
    test_comparison();
    test_vector_of_functions();
    test_move_in_vector();

    printf("\n========================================\n");
    printf("All 25 tests PASSED!\n");
    printf("========================================\n");

    return 0;
}
