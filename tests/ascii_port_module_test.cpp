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

    // Case folding now delegates to rusty::to_ascii_X helpers.
    assert(::ascii_port::to_uppercase(AsciiChar::SmallA) == AsciiChar::CapitalA);
    assert(::ascii_port::to_lowercase(AsciiChar::CapitalA) == AsciiChar::SmallA);
    assert(::ascii_port::to_uppercase(AsciiChar::Digit5) == AsciiChar::Digit5);
    AsciiChar c = AsciiChar::SmallZ;
    ::ascii_port::make_uppercase(c);
    assert(c == AsciiChar::CapitalZ);

    // Predicates delegate to rusty::is_ascii_X.
    assert(::ascii_port::is_alphabetic(AsciiChar::CapitalA));
    assert(::ascii_port::is_alphabetic(AsciiChar::SmallZ));
    assert(!::ascii_port::is_alphabetic(AsciiChar::Digit0));
    assert(::ascii_port::is_uppercase(AsciiChar::CapitalA));
    assert(!::ascii_port::is_uppercase(AsciiChar::SmallA));
    assert(::ascii_port::is_lowercase(AsciiChar::SmallA));
    assert(::ascii_port::is_digit(AsciiChar::Digit5));
    assert(::ascii_port::is_hexdigit(AsciiChar::SmallF));
    assert(!::ascii_port::is_hexdigit(AsciiChar::SmallG));
    assert(::ascii_port::is_whitespace(AsciiChar::Space));
    assert(::ascii_port::is_control(AsciiChar::Null));
    assert(!::ascii_port::is_control(AsciiChar::CapitalA));
    assert(::ascii_port::is_graphic(AsciiChar::Tilde));
    assert(::ascii_port::eq_ignore_case(AsciiChar::CapitalA, AsciiChar::SmallA));

    std::printf("ascii_port module smoke OK: enum + factories + to_u8/to_char\n"
                "                          + case folding + predicates (rusty::ascii::*)\n");
    return 0;
}
