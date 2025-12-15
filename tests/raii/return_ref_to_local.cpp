// Test: Return Reference to Local
// Status: NOT DETECTED (requires RAII tracking Phase 1)
//
// This is one of the most common memory safety bugs in C++.
// Rust prevents this at compile time; we should too.

#include <string>

// =============================================================================
// NEGATIVE TESTS - Should produce errors after implementation
// =============================================================================

// @safe
std::string& bad_return_ref() {
    std::string s = "hello";
    return s;  // ERROR: returning reference to local 's' that will be destroyed
}

// @safe
const std::string& bad_return_const_ref() {
    std::string s = "world";
    return s;  // ERROR: returning const reference to local
}

// @safe
int& bad_return_int_ref() {
    int x = 42;
    return x;  // ERROR: returning reference to local int
}

// @safe
int* bad_return_ptr_to_local() {
    int x = 42;
    // @unsafe
    return &x;  // ERROR: returning pointer to local (even in unsafe, should warn)
}

// @safe
std::string& bad_conditional_return(bool condition) {
    std::string a = "a";
    std::string b = "b";
    if (condition) {
        return a;  // ERROR: 'a' is local
    } else {
        return b;  // ERROR: 'b' is local
    }
}

// @safe
const char* bad_return_c_str() {
    std::string s = "temporary";
    return s.c_str();  // ERROR: s destroyed, c_str() points to freed memory
}

// More subtle: returning reference through a chain
struct Wrapper {
    std::string value;
    std::string& get() { return value; }
};

// @safe
std::string& bad_return_through_member() {
    Wrapper w;
    w.value = "test";
    return w.get();  // ERROR: w destroyed, w.value destroyed
}

// =============================================================================
// POSITIVE TESTS - Should NOT produce errors
// =============================================================================

// @safe
std::string good_return_by_value() {
    std::string s = "hello";
    return s;  // OK: return by value (move semantics)
}

// @safe
std::string good_return_literal() {
    return "hello";  // OK: string literal has static lifetime
}

// @safe
int good_return_copy() {
    int x = 42;
    return x;  // OK: return by value (copy)
}

// Static storage - OK to return reference
static std::string global_string = "global";

// @safe
std::string& good_return_static_ref() {
    return global_string;  // OK: global has static lifetime
}

// Parameter reference - OK if caller owns it
// @safe
std::string& good_return_param_ref(std::string& input) {
    return input;  // OK: caller owns the string
}

// @safe
const std::string& good_return_param_const_ref(const std::string& input) {
    return input;  // OK: just forwarding the reference
}
