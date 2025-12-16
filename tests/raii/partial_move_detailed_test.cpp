// Detailed test: What partial move features does RustyCpp support?

#include <string>
#include <utility>

struct Pair {
    std::string first;
    std::string second;
};

// TEST 1: Double move of same field - should ERROR
// @safe
void test_double_move_same_field() {
    Pair p;
    p.first = "hello";
    std::string x = std::move(p.first);
    std::string y = std::move(p.first);  // ERROR expected: p.first already moved
}

// TEST 2: Move different fields - should be OK
// @safe
void test_move_different_fields() {
    Pair p;
    p.first = "hello";
    p.second = "world";
    std::string x = std::move(p.first);   // Move p.first
    std::string y = std::move(p.second);  // Should be OK: p.second not moved yet
}

// TEST 3: Use field after moving different field - should be OK
// @safe
void test_use_unmoved_field() {
    Pair p;
    p.first = "hello";
    p.second = "world";
    std::string x = std::move(p.first);   // Move p.first
    int len = p.second.length();          // Should be OK: p.second not moved
}

// TEST 4: Use same field after move - should ERROR
// @safe
void test_use_moved_field() {
    Pair p;
    p.first = "hello";
    std::string x = std::move(p.first);   // Move p.first
    int len = p.first.length();           // ERROR expected: p.first was moved
}

int main() { return 0; }
