// Smoke test for the transpiled ascii_port. Phase B level — proves
// the library links and `AsciiChar` enum + factories + manifest
// constants are usable from C++23 modules. The byte-method-bound API
// (to_uppercase / is_alphabetic / etc.) is stubbed by the patcher
// pending rusty::ascii::* helper additions; this test only exercises
// what's currently functional.

import ascii_port;

#include <cassert>
#include <cstdio>
#include <string_view>

int main() {
    using ::ascii_port::AsciiChar;

    // Enum variants compile and have stable values.
    static_assert(static_cast<int>(AsciiChar::Null) == 0);
    static_assert(static_cast<int>(AsciiChar::CapitalA) == 65);
    static_assert(static_cast<int>(AsciiChar::SmallA) == 97);
    static_assert(static_cast<int>(AsciiChar::Delete) == 127);

    // (AsciiChar_MIN / _MAX are emitted without `export` so they're not
    // visible from importers — pure inherent-impl constants.)

    // to_u8 / to_char round-trip.
    assert(::ascii_port::to_u8(AsciiChar::CapitalA) == 65);
    assert(::ascii_port::to_char(AsciiChar::CapitalA) == U'A');

    std::printf("ascii_port module smoke OK: enum + factories + to_u8/to_char\n");
    return 0;
}
