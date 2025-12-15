// Test: Member Reference Outlives Containing Object
// Status: NOT DETECTED (requires RAII tracking Phase 3)
//
// When you take a reference to an object's member, that reference
// must not outlive the containing object.

#include <string>
#include <memory>
#include <vector>

struct Container {
    std::string data;
    int value;
    std::vector<int> items;

    const std::string& get_data() const { return data; }
    int& get_value() { return value; }
    std::vector<int>& get_items() { return items; }
};

// =============================================================================
// NEGATIVE TESTS - Should produce errors after implementation
// =============================================================================

// @safe
const std::string& bad_return_member_ref() {
    Container c;
    c.data = "hello";
    return c.get_data();  // ERROR: c.data destroyed when c destroyed
}

// @safe
int& bad_return_member_value_ref() {
    Container c;
    c.value = 42;
    return c.get_value();  // ERROR: c.value destroyed when c destroyed
}

// @safe
void bad_store_member_ptr() {
    const std::string* ptr;
    {
        Container c;
        c.data = "hello";
        ptr = &c.data;  // ptr points to c.data
    }  // c destroyed, c.data destroyed

    // ERROR: ptr is dangling
    // @unsafe
    auto len = ptr->length();
}

// @safe
void bad_store_member_ref() {
    int* ref_ptr;
    {
        Container c;
        c.value = 42;
        ref_ptr = &c.value;
    }  // c destroyed

    // ERROR: ref_ptr is dangling
    // @unsafe
    *ref_ptr = 10;
}

// Nested member access
struct Outer {
    Container inner;
};

// @safe
const std::string& bad_nested_member_ref() {
    Outer o;
    o.inner.data = "nested";
    return o.inner.get_data();  // ERROR: o.inner.data destroyed when o destroyed
}

// Through unique_ptr
// @safe
void bad_unique_ptr_member() {
    int* raw;
    {
        auto ptr = std::make_unique<Container>();
        ptr->value = 42;
        raw = &ptr->value;
    }  // ptr destroyed, Container destroyed

    // ERROR: raw is dangling
    // @unsafe
    *raw = 10;
}

// Vector element reference
// @safe
void bad_vector_member_ref() {
    int* elem_ptr;
    {
        Container c;
        c.items = {1, 2, 3};
        elem_ptr = &c.items[0];
    }  // c destroyed, c.items destroyed

    // ERROR: elem_ptr is dangling
    // @unsafe
    *elem_ptr = 10;
}

// =============================================================================
// POSITIVE TESTS - Should NOT produce errors
// =============================================================================

// @safe
void good_member_access_in_scope() {
    Container c;
    c.data = "hello";
    const std::string& ref = c.get_data();
    auto len = ref.length();  // OK: ref and c have same scope
}

// @safe
void good_copy_member_value() {
    int val;
    {
        Container c;
        c.value = 42;
        val = c.value;  // Copy, not reference
    }
    int x = val;  // OK: val is a copy
}

// @safe
std::string good_return_member_by_value(Container& c) {
    return c.data;  // OK: returns copy
}

// @safe
const std::string& good_return_param_member(Container& c) {
    return c.get_data();  // OK: c is owned by caller
}
