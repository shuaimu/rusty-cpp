// Test: Partial moves - moving individual struct fields
// Expected: Rust supports this, RustyCpp may not

#include <string>
#include <utility>

struct Pair {
    std::string first;
    std::string second;
};

// @safe
void test_partial_move_basic() {
    Pair p;
    p.first = "hello";
    p.second = "world";

    // Move only p.first
    std::string x = std::move(p.first);

    // In Rust: p.second is still valid, p.first is moved
    // Question: Does RustyCpp track this at field level?
    std::string y = std::move(p.second);  // Should be OK - p.second wasn't moved
}

// @safe
void test_partial_move_use_after_field_move() {
    Pair p;
    p.first = "hello";
    p.second = "world";

    std::string x = std::move(p.first);

    // This should be an error - p.first was moved
    std::string z = std::move(p.first);  // ERROR: use after move of p.first
}

// @safe
void test_use_unmoved_field_after_partial_move() {
    Pair p;
    p.first = "hello";
    p.second = "world";

    std::string x = std::move(p.first);

    // This should be OK - only p.first was moved, not p.second
    int len = p.second.length();  // Should be OK
}

// @safe
void test_whole_struct_move_after_partial() {
    Pair p;
    p.first = "hello";
    p.second = "world";

    std::string x = std::move(p.first);

    // In Rust: Cannot move whole struct after partial move
    // Pair p2 = std::move(p);  // Should be ERROR in Rust
}

int main() {
    test_partial_move_basic();
    return 0;
}
