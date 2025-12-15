// Test: Temporary Object Lifetime
// Status: NOT DETECTED (requires RAII tracking Phase 4)
//
// C++ temporaries have complex lifetime rules. References to temporaries
// can dangle if the temporary is destroyed before the reference is used.

#include <string>
#include <vector>

std::string get_string() {
    return std::string("temporary");
}

struct Result {
    std::string value;
    const std::string& get() const { return value; }
};

Result get_result() {
    return Result{"result"};
}

// =============================================================================
// NEGATIVE TESTS - Should produce errors after implementation
// =============================================================================

// @safe
const std::string& bad_return_temporary() {
    return std::string("temp");  // ERROR: temporary destroyed at semicolon
}

// @safe
const std::string& bad_return_temporary_from_call() {
    return get_string();  // ERROR: returned temporary destroyed immediately
}

// @safe
void bad_ref_to_temp_member() {
    const std::string& ref = get_result().get();
    // ERROR: Result temporary destroyed, ref dangles
    // @unsafe
    auto len = ref.length();
}

// @safe
void bad_pointer_to_temporary() {
    // @unsafe
    const std::string* ptr = &get_string();
    // ERROR: temporary destroyed, ptr dangles
    // @unsafe
    auto len = ptr->length();
}

// Ternary with temporary
// @safe
void bad_ternary_temporary(bool condition) {
    std::string a = "a";
    const std::string& ref = condition ? a : std::string("temp");
    // ERROR if condition==false: temporary destroyed, ref dangles
    // @unsafe
    auto len = ref.length();
}

// Chained method calls with temporary
// @safe
void bad_chained_temp() {
    // @unsafe
    const char* ptr = get_string().c_str();
    // ERROR: temporary string destroyed, ptr dangles
    // @unsafe
    char c = ptr[0];
}

// Temporary in range-for
// @safe
void bad_temp_in_range_for() {
    // This is actually OK in C++ due to lifetime extension in range-for
    // But this variant is NOT:
    // @unsafe
    auto& items_ref = std::vector<int>{1, 2, 3};
    // Temporary vector destroyed here, but ref survives
    for (int x : items_ref) {  // ERROR: items_ref is dangling
        // ...
    }
}

// =============================================================================
// POSITIVE TESTS - Should NOT produce errors (lifetime extension applies)
// =============================================================================

// @safe
void good_const_ref_extends_lifetime() {
    const std::string& ref = get_string();
    // OK: const ref extends temporary's lifetime to ref's scope
    auto len = ref.length();
}

// @safe
void good_rvalue_ref_extends_lifetime() {
    std::string&& rref = get_string();
    // OK: rvalue ref extends temporary's lifetime
    auto len = rref.length();
}

// @safe
void good_copy_from_temporary() {
    std::string s = get_string();
    // OK: s is a copy (or move), owns the data
    auto len = s.length();
}

// @safe
void good_temp_in_expression() {
    auto len = get_string().length();
    // OK: temporary lives until end of full expression
}

// @safe
void good_range_for_with_temp() {
    for (int x : std::vector<int>{1, 2, 3}) {
        // OK: temporary lifetime extended for range-for
        int y = x;
    }
}

// @safe
void good_pass_temp_to_function() {
    // Assuming process takes const std::string&
    // process(get_string());
    // OK: temporary lives until function returns
}

// @safe
int good_immediate_use_of_temp() {
    return get_result().value.length();
    // OK: all temporaries live until end of full expression
}
