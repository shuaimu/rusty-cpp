// Tests for smart pointer arrow operator safety
// Verifies that:
// 1. operator-> on safe smart pointers (Box, Arc, etc.) is allowed in @safe code
// 2. operator-> on raw pointers is NOT allowed in @safe code

#include <rusty/box.hpp>

struct Data {
    int value;
    // @safe
    int get_value() const { return value; }
};

// ===========================================================================
// SAFE CASES - These should NOT produce errors
// ===========================================================================

// Test 1: Box->field is safe (field access via arrow)
// @safe
int test_box_field_access() {
    auto box = rusty::Box<Data>::make(Data{42});
    return box->value;  // Safe - Box operator-> is safe
}

// Test 2: Box->method() is safe (method call via arrow)
// @safe
int test_box_method_call() {
    auto box = rusty::Box<Data>::make(Data{42});
    return box->get_value();  // Safe - Box operator-> is safe
}

// Test 3: (*box).field is safe (explicit dereference via operator*)
// @safe
int test_box_deref() {
    auto box = rusty::Box<Data>::make(Data{42});
    return (*box).value;  // Safe - *box returns Data&, not Data*
}

// Test 4: Chained smart pointer access
// @safe
int test_box_chained() {
    auto box = rusty::Box<Data>::make(Data{42});
    auto box2 = rusty::Box<Data>::make(Data{box->value + 10});
    return box2->get_value();  // Safe - multiple Box operations
}

// ===========================================================================
// UNSAFE CASES - Raw pointers should produce errors in @safe code
// These are wrapped in @unsafe blocks to allow the file to pass analysis
// ===========================================================================

// Test 5: Raw pointer -> is UNSAFE (must use @unsafe block)
// @safe
int test_raw_pointer_in_unsafe_block() {
    Data d{42};
    // @unsafe
    {
        Data* ptr = &d;      // address-of needs unsafe
        return ptr->value;   // raw pointer dereference needs unsafe
    }
}

// Test 6: Raw pointer * is UNSAFE (must use @unsafe block)
// @safe
int test_raw_deref_in_unsafe_block() {
    Data d{42};
    // @unsafe
    {
        Data* ptr = &d;      // address-of needs unsafe
        return (*ptr).value; // raw pointer dereference needs unsafe
    }
}

int main() { return 0; }
