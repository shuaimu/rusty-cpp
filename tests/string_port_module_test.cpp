// Smoke test for string_port (Phase B/C bridge module).
import string_port;

#include <rusty/string.hpp>
#include <cassert>
#include <cstdio>

int main() {
    rusty::string::String s = rusty::string::String::from("hello");
    assert(s.len() == 5);
    std::printf("string_port (stub bridge) smoke OK: len=%zu\n", s.len());
    return 0;
}
