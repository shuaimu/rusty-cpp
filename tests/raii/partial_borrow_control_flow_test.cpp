// Test: Partial borrows with control flow (loops and branches)
// Goal: Verify field borrows work correctly across control flow

#include <string>

struct Pair {
    std::string first;
    std::string second;
};

// ============================================================================
// BRANCH TESTS (if/else)
// ============================================================================

// TEST 1: Borrow field in one branch only - conservative (borrow cleared)
// @safe
void test_branch_borrow_one_side(bool condition) {
    Pair p;
    p.first = "hello";

    if (condition) {
        std::string& r = p.first;  // Borrow in then branch only
        r = "modified";
    }
    // After if: borrow should be cleared (conservative)

    std::string& r2 = p.first;  // Should be OK - borrow cleared
    r2 = "again";
}

// TEST 2: Borrow same field in both branches - borrow persists
// @safe
void test_branch_borrow_both_sides(bool condition) {
    Pair p;
    p.first = "hello";
    std::string* ptr = nullptr;

    if (condition) {
        std::string& r = p.first;  // Borrow in then
        ptr = &r;
    } else {
        std::string& r = p.first;  // Borrow in else
        ptr = &r;
    }
    // After if: depends on implementation - may or may not persist
}

// TEST 3: Borrow different fields in branches - both accessible after
// @safe
void test_branch_different_fields(bool condition) {
    Pair p;
    p.first = "hello";
    p.second = "world";

    if (condition) {
        std::string& r = p.first;  // Borrow first
        r = "modified first";
    } else {
        std::string& r = p.second;  // Borrow second
        r = "modified second";
    }
    // After if: neither borrow persists (different in each branch)

    std::string& r1 = p.first;   // Should be OK
    std::string& r2 = p.second;  // Should be OK
}

// TEST 4: Borrow before if, use in both branches - should work
// @safe
void test_borrow_before_branch(bool condition) {
    Pair p;
    p.first = "hello";
    p.second = "world";

    std::string& r = p.first;  // Borrow before branch

    if (condition) {
        r = "modified in then";
    } else {
        r = "modified in else";
    }
    // r still valid after if
    r = "final";
}

// ============================================================================
// LOOP TESTS
// ============================================================================

// TEST 5: Borrow field inside loop - cleared each iteration
// @safe
void test_loop_borrow_inside() {
    Pair p;
    p.first = "hello";

    for (int i = 0; i < 3; i++) {
        std::string& r = p.first;  // New borrow each iteration
        r = "modified";
        // r goes out of scope, borrow cleared
    }
    // After loop: no active borrows

    std::string& r2 = p.first;  // Should be OK
    r2 = "final";
}

// TEST 6: Borrow before loop, use inside - valid throughout
// @safe
void test_borrow_before_loop() {
    Pair p;
    p.first = "hello";

    std::string& r = p.first;  // Borrow before loop

    for (int i = 0; i < 3; i++) {
        r = "modified";  // Use borrow inside loop
    }
    // r still valid
    r = "final";
}

// TEST 7: While loop with borrows
// @safe
void test_while_loop_borrow() {
    Pair p;
    p.first = "hello";
    int count = 0;

    while (count < 3) {
        std::string& r = p.first;  // Borrow each iteration
        r = "modified";
        count++;
    }
    // After while: borrow cleared

    std::string& r2 = p.first;  // Should be OK
}

// ============================================================================
// SCOPE CLEANUP TESTS
// ============================================================================

// TEST 8: Nested scopes - inner borrow cleared on scope exit
// @safe
void test_nested_scope_cleanup() {
    Pair p;
    p.first = "hello";

    {
        std::string& r1 = p.first;  // Borrow in inner scope
        r1 = "inner modified";
    }  // r1 out of scope, borrow cleared

    std::string& r2 = p.first;  // Should be OK - previous borrow cleared
    r2 = "outer modified";
}

// TEST 9: Sequential borrows in separate scopes - should work
// @safe
void test_sequential_scope_borrows() {
    Pair p;
    p.first = "hello";

    {
        std::string& r = p.first;
        r = "first scope";
    }

    {
        std::string& r = p.first;  // Same name, new borrow - OK
        r = "second scope";
    }

    {
        std::string& r = p.first;  // Third time - OK
        r = "third scope";
    }
}

// TEST 10: Parallel borrows of different fields - OK even in same scope
// @safe
void test_parallel_field_borrows() {
    Pair p;
    p.first = "hello";
    p.second = "world";

    std::string& r1 = p.first;   // Borrow first
    std::string& r2 = p.second;  // Borrow second - OK, different fields

    r1 = "modified first";
    r2 = "modified second";
}

// TEST 11: Multiple sequential borrows of same field in same scope - ERROR
// @safe
void test_sequential_same_scope_error() {
    Pair p;
    p.first = "hello";

    std::string& r1 = p.first;  // First borrow
    std::string& r2 = p.first;  // ERROR: r1 still active in same scope

    r1 = "modified";
}

int main() { return 0; }
