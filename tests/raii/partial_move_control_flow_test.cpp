// Test: Partial moves with control flow (loops and branches)
// Goal: Verify field moves work correctly across control flow

#include <string>
#include <utility>

struct Pair {
    std::string first;
    std::string second;
};

// ============================================================================
// BRANCH TESTS (if/else)
// ============================================================================

// TEST 1: Move field in one branch - moved after if (conservative)
// @safe
void test_branch_move_one_side(bool condition) {
    Pair p;
    p.first = "hello";

    if (condition) {
        std::string x = std::move(p.first);  // Move in then branch only
    }
    // After if: p.first considered moved (conservative - moved in ANY path)

    std::string y = std::move(p.first);  // ERROR: may have been moved
}

// TEST 2: Move same field in both branches - definitely moved
// @safe
void test_branch_move_both_sides(bool condition) {
    Pair p;
    p.first = "hello";

    if (condition) {
        std::string x = std::move(p.first);  // Move in then
    } else {
        std::string y = std::move(p.first);  // Move in else
    }
    // After if: p.first definitely moved

    std::string z = std::move(p.first);  // ERROR: already moved
}

// TEST 3: Move different fields in branches - both considered moved
// @safe
void test_branch_move_different_fields(bool condition) {
    Pair p;
    p.first = "hello";
    p.second = "world";

    if (condition) {
        std::string x = std::move(p.first);   // Move first in then
    } else {
        std::string y = std::move(p.second);  // Move second in else
    }
    // After if: Conservative analysis - both may be moved

    // These may error depending on analysis
    std::string a = std::move(p.first);   // May error - moved in then branch
    std::string b = std::move(p.second);  // May error - moved in else branch
}

// TEST 4: Use field after branch where it was moved - ERROR
// @safe
void test_use_after_branch_move(bool condition) {
    Pair p;
    p.first = "hello";

    if (condition) {
        std::string x = std::move(p.first);
    }

    int len = p.first.length();  // ERROR: p.first may have been moved
}

// ============================================================================
// LOOP TESTS
// ============================================================================

// TEST 5: Move in first iteration, use in second - ERROR
// @safe
void test_loop_move_and_use() {
    Pair p;
    p.first = "hello";

    for (int i = 0; i < 2; i++) {
        std::string x = std::move(p.first);  // ERROR on second iteration
    }
}

// TEST 6: Move field before loop, use in loop - ERROR
// @safe
void test_move_before_loop_use_in() {
    Pair p;
    p.first = "hello";

    std::string x = std::move(p.first);  // Move before loop

    for (int i = 0; i < 2; i++) {
        int len = p.first.length();  // ERROR: p.first already moved
    }
}

// TEST 7: Move different fields in loop - tracks each separately
// @safe
void test_loop_move_different_fields() {
    Pair p1, p2;
    p1.first = "hello";
    p2.first = "world";

    // This is OK - moving different objects
    std::string x = std::move(p1.first);
    std::string y = std::move(p2.first);
}

// ============================================================================
// SCOPE AND REASSIGNMENT TESTS
// ============================================================================

// TEST 8: Move in scope, reassign after scope - field valid again
// @safe
void test_reassign_after_scope_move() {
    Pair p;
    p.first = "hello";

    {
        std::string x = std::move(p.first);  // Move in inner scope
    }

    p.first = "reassigned";  // Reassign - p.first valid again

    std::string y = std::move(p.first);  // OK - was reassigned
}

// TEST 9: Move field, reassign, move again - OK
// @safe
void test_move_reassign_move_again() {
    Pair p;
    p.first = "hello";

    std::string x = std::move(p.first);  // First move
    p.first = "new value";               // Reassign
    std::string y = std::move(p.first);  // OK - was reassigned
}

// TEST 10: Move one field, other still usable
// @safe
void test_partial_move_other_field_ok() {
    Pair p;
    p.first = "hello";
    p.second = "world";

    std::string x = std::move(p.first);  // Move first

    std::string y = std::move(p.second);  // OK - second not moved
}

// TEST 11: Move both fields, whole struct unusable
// @safe
void test_move_both_fields() {
    Pair p;
    p.first = "hello";
    p.second = "world";

    std::string x = std::move(p.first);   // Move first
    std::string y = std::move(p.second);  // Move second

    Pair p2 = std::move(p);  // ERROR: p fully moved (both fields)
}

// ============================================================================
// NESTED STRUCT CONTROL FLOW
// ============================================================================

struct Inner {
    std::string data;
};

struct Outer {
    Inner inner;
    std::string name;
};

// TEST 12: Move nested field in branch
// @safe
void test_nested_branch_move(bool condition) {
    Outer o;
    o.inner.data = "hello";

    if (condition) {
        std::string x = std::move(o.inner.data);  // Move in then
    }

    std::string y = std::move(o.inner.data);  // ERROR: may have been moved
}

// TEST 13: Move nested in loop - ERROR on second iteration
// @safe
void test_nested_loop_move() {
    Outer o;
    o.inner.data = "hello";

    for (int i = 0; i < 2; i++) {
        std::string x = std::move(o.inner.data);  // ERROR on second iteration
    }
}

int main() { return 0; }
