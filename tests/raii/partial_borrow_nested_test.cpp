// Test: Nested field borrow tracking
// Goal: Track borrows of nested fields like p.inner.field

#include <string>

struct Inner {
    std::string data;
    int value;
};

struct Outer {
    Inner inner;
    std::string name;
};

// TEST 1: Borrow different nested fields mutably - should be OK
// @safe
void test_nested_different_fields_mutable() {
    Outer o;
    o.inner.data = "hello";
    o.name = "world";

    std::string& r1 = o.inner.data;  // Mutable borrow of o.inner.data
    std::string& r2 = o.name;        // OK: o.name is separate from o.inner.data

    r1 = "modified";
    r2 = "also modified";
}

// TEST 2: Double mutable borrow of same nested field - should ERROR
// @safe
void test_nested_same_field_double_mutable() {
    Outer o;
    o.inner.data = "hello";

    std::string& r1 = o.inner.data;  // Mutable borrow of o.inner.data
    std::string& r2 = o.inner.data;  // ERROR: o.inner.data already mutably borrowed

    r1 = "modified";
}

// TEST 3: Borrow sibling nested fields - should be OK
// @safe
void test_nested_sibling_fields() {
    Outer o;
    o.inner.data = "hello";
    o.inner.value = 42;

    std::string& r1 = o.inner.data;  // Mutable borrow of o.inner.data
    int& r2 = o.inner.value;         // OK: o.inner.value is separate

    r1 = "modified";
    r2 = 100;
}

// TEST 4: Borrow parent after borrowing nested - should ERROR
// @safe
void test_borrow_parent_after_nested() {
    Outer o;
    o.inner.data = "hello";

    std::string& r1 = o.inner.data;  // Mutable borrow of o.inner.data
    Inner& r2 = o.inner;             // ERROR: Cannot borrow o.inner while o.inner.data borrowed

    r1 = "modified";
}

// TEST 5: Borrow nested after borrowing parent - should ERROR
// @safe
void test_borrow_nested_after_parent() {
    Outer o;
    o.inner.data = "hello";

    Inner& r1 = o.inner;             // Mutable borrow of whole o.inner
    std::string& r2 = o.inner.data;  // ERROR: Cannot borrow o.inner.data while o.inner borrowed

    r2 = "modified";
}

// TEST 6: Multiple immutable borrows of nested field - should be OK
// @safe
void test_nested_multiple_immutable() {
    Outer o;
    o.inner.data = "hello";

    const std::string& r1 = o.inner.data;  // Immutable borrow
    const std::string& r2 = o.inner.data;  // OK: multiple immutable allowed

    // Both r1 and r2 valid for reading
}

// TEST 7: Immutable + mutable borrow of different nested fields - should be OK
// @safe
void test_nested_mixed_different_fields() {
    Outer o;
    o.inner.data = "hello";
    o.inner.value = 42;

    const std::string& r1 = o.inner.data;  // Immutable borrow of data
    int& r2 = o.inner.value;               // OK: value is separate field

    r2 = 100;
}

// TEST 8: Deep nesting (3+ levels) borrow - should be OK for different paths
struct Level3 { std::string data; };
struct Level2 { Level3 level3; int x; };
struct Level1 { Level2 level2; };
struct Root { Level1 level1; std::string name; };

// @safe
void test_deep_nested_different_paths() {
    Root r;
    r.level1.level2.level3.data = "deep";
    r.name = "root";

    std::string& r1 = r.level1.level2.level3.data;  // Deep nested borrow
    std::string& r2 = r.name;                        // OK: different path

    r1 = "modified deep";
    r2 = "modified root";
}

// TEST 9: Deep nesting double borrow same field - should ERROR
// @safe
void test_deep_nested_same_field() {
    Root r;
    r.level1.level2.level3.data = "deep";

    std::string& r1 = r.level1.level2.level3.data;  // Deep nested borrow
    std::string& r2 = r.level1.level2.level3.data;  // ERROR: already borrowed

    r1 = "modified";
}

int main() { return 0; }
