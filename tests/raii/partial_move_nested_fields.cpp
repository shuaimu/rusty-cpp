// Test: Nested field tracking for partial moves
// Goal: Track moves of nested struct fields like p.inner.field

#include <string>
#include <utility>

struct Inner {
    std::string data;
    int value;
};

struct Outer {
    Inner inner;
    std::string name;
};

// TEST 1: Double move of nested field - should ERROR
// @safe
void test_nested_double_move() {
    Outer o;
    o.inner.data = "hello";

    std::string x = std::move(o.inner.data);  // Move o.inner.data
    std::string y = std::move(o.inner.data);  // ERROR: o.inner.data already moved
}

// TEST 2: Move different nested fields - should be OK
// @safe
void test_nested_different_fields() {
    Outer o;
    o.inner.data = "hello";
    o.name = "world";

    std::string x = std::move(o.inner.data);  // Move o.inner.data
    std::string y = std::move(o.name);        // OK: o.name not moved
}

// TEST 3: Use nested field after move - should ERROR
// @safe
void test_nested_use_after_move() {
    Outer o;
    o.inner.data = "hello";

    std::string x = std::move(o.inner.data);  // Move o.inner.data
    int len = o.inner.data.length();          // ERROR: o.inner.data was moved
}

// TEST 4: Move parent after moving nested field - should ERROR
// @safe
void test_move_parent_after_nested_move() {
    Outer o;
    o.inner.data = "hello";

    std::string x = std::move(o.inner.data);  // Move o.inner.data
    Inner i = std::move(o.inner);             // ERROR: o.inner partially moved
}

// TEST 5: Move whole struct after nested field move - should ERROR
// @safe
void test_move_whole_after_nested_move() {
    Outer o;
    o.inner.data = "hello";

    std::string x = std::move(o.inner.data);  // Move o.inner.data
    Outer o2 = std::move(o);                  // ERROR: o partially moved (via o.inner.data)
}

// TEST 6: Use sibling nested field after move - should be OK
// @safe
void test_sibling_nested_field_ok() {
    Outer o;
    o.inner.data = "hello";
    o.inner.value = 42;

    std::string x = std::move(o.inner.data);  // Move o.inner.data
    int v = o.inner.value;                    // OK: o.inner.value not moved
}

int main() { return 0; }
