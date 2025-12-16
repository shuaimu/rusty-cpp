// Test: Partial borrow tracking
// Goal: Track borrows of individual fields separately from whole-struct borrows

#include <string>

struct Pair {
    std::string first;
    std::string second;
};

// TEST 1: Borrow different fields mutably - should be OK (Rust allows this)
// @safe
void test_different_fields_mutable_borrow() {
    Pair p;
    p.first = "hello";
    p.second = "world";

    std::string& r1 = p.first;   // Mutable borrow of p.first
    std::string& r2 = p.second;  // OK: p.second is separate field

    r1 = "modified";
    r2 = "also modified";
}

// TEST 2: Double mutable borrow of same field - should ERROR
// @safe
void test_same_field_double_mutable_borrow() {
    Pair p;
    p.first = "hello";

    std::string& r1 = p.first;   // Mutable borrow of p.first
    std::string& r2 = p.first;   // ERROR: p.first already mutably borrowed

    r1 = "modified";
}

// TEST 3: Mutable and immutable borrow of same field - should ERROR
// @safe
void test_same_field_mixed_borrow() {
    Pair p;
    p.first = "hello";

    const std::string& r1 = p.first;  // Immutable borrow of p.first
    std::string& r2 = p.first;        // ERROR: p.first already borrowed

    r2 = "modified";
}

// TEST 4: Immutable borrow field, mutable borrow different field - should be OK
// @safe
void test_mixed_borrows_different_fields() {
    Pair p;
    p.first = "hello";
    p.second = "world";

    const std::string& r1 = p.first;  // Immutable borrow of p.first
    std::string& r2 = p.second;       // OK: p.second is separate

    r2 = "modified";
    // r1 still valid for reading
}

// TEST 5: Multiple immutable borrows of same field - should be OK
// @safe
void test_multiple_immutable_same_field() {
    Pair p;
    p.first = "hello";

    const std::string& r1 = p.first;  // Immutable borrow of p.first
    const std::string& r2 = p.first;  // OK: multiple immutable borrows allowed

    // Both r1 and r2 valid for reading
}

// TEST 6: Whole struct borrow conflicts with field borrow
// @safe
void test_whole_struct_vs_field_borrow() {
    Pair p;
    p.first = "hello";

    std::string& r1 = p.first;  // Mutable borrow of p.first
    Pair& r2 = p;               // ERROR: Cannot borrow whole p while p.first borrowed
}

// TEST 7: Field borrow after whole struct borrow - should ERROR
// @safe
void test_field_borrow_after_whole() {
    Pair p;
    p.first = "hello";

    Pair& r1 = p;               // Mutable borrow of whole p
    std::string& r2 = p.first;  // ERROR: Cannot borrow p.first while p borrowed
}

int main() { return 0; }
