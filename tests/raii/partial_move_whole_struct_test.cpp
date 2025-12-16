// Test: Moving whole struct after partial move
// This should be detected as an error

#include <string>
#include <utility>

struct Pair {
    std::string first;
    std::string second;
};

// TEST: Move whole struct after moving a field - should ERROR
// @safe
void test_whole_struct_move_after_partial() {
    Pair p;
    p.first = "hello";
    p.second = "world";

    std::string x = std::move(p.first);  // Move just p.first

    // This should be an error - p is partially moved
    Pair p2 = std::move(p);  // ERROR: Cannot move p because p.first already moved
}

int main() { return 0; }
