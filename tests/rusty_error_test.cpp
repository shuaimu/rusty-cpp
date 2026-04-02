// Tests for rusty::error helpers used by transpiled expanded output.
#include <rusty/error.hpp>

#include <cassert>
#include <iostream>
#include <string>
#include <string_view>

namespace {

struct HasDescriptionView {
    std::string_view description() const { return "hello"; }
};

struct HasDescriptionString {
    std::string description() const { return "world"; }
};

struct HasDescriptionCString {
    const char* description() const { return "cstr"; }
};

struct NoDescription {};

void test_description_dispatch_uses_member_when_available() {
    const HasDescriptionView v;
    const HasDescriptionCString c;
    assert(rusty::error::description(v) == "hello");
    assert(rusty::error::description(c) == "cstr");
    std::cout << "  test_description_dispatch_uses_member_when_available PASSED\n";
}

void test_description_dispatch_falls_back_to_empty_for_non_error_types() {
    const NoDescription n;
    const HasDescriptionString s;
    assert(rusty::error::description(n).empty());
    assert(rusty::error::description(s).empty());
    std::cout << "  test_description_dispatch_falls_back_to_empty_for_non_error_types PASSED\n";
}

} // namespace

int main() {
    std::cout << "Running rusty::error tests...\n";
    test_description_dispatch_uses_member_when_available();
    test_description_dispatch_falls_back_to_empty_for_non_error_types();
    std::cout << "All rusty::error tests PASSED\n";
    return 0;
}
