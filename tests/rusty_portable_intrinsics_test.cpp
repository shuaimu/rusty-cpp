// Smoke test: rusty-cpp must build and behave correctly when the
// consumer opts into the portable-intrinsics path that avoids
// pulling x86 SIMD headers (`<immintrin.h>`) into the rusty-cpp
// public surface. See `include/rusty/hashmap.hpp` for the
// motivation — clang21+ rejects duplicate static-inline SIMD
// intrinsic definitions when `<immintrin.h>` is reachable through
// both a C++23 named-module GMF and a direct include in the
// importer.
//
// This test compiles with `RUSTY_PORTABLE_INTRINSICS` defined
// before any rusty-cpp header. `<immintrin.h>` must therefore not
// appear in any rusty-cpp surface used by this TU. We then run a
// quick functional test of `HashMap` (which uses the scalar
// fallback for `Group::match_byte` and
// `Group::match_empty_or_deleted`) and of `mpsc_lockfree`'s
// `CPU_RELAX` macro (which falls back to
// `__builtin_ia32_pause()` on clang/gcc x86).

#define RUSTY_PORTABLE_INTRINSICS 1

#include "../include/rusty/hashmap.hpp"
#include "../include/rusty/sync/mpsc_lockfree.hpp"

#include <cassert>
#include <iostream>
#include <string>

int main() {
    // HashMap functional check — exercises the scalar Group probe.
    rusty::HashMap<int, std::string> m;
    assert(m.is_empty());
    for (int i = 0; i < 64; ++i) {
        m.insert(i, std::string("v") + std::to_string(i));
    }
    assert(m.len() == 64);
    for (int i = 0; i < 64; ++i) {
        auto opt = m.get(i);
        assert(opt.is_some());
        assert(*opt.unwrap() == std::string("v") + std::to_string(i));
    }
    // Negative lookup must still go through the scalar match-empty
    // path without false positives.
    auto missing = m.get(64);
    assert(missing.is_none());

    for (int i = 0; i < 32; ++i) {
        m.remove(i);
    }
    assert(m.len() == 32);
    for (int i = 32; i < 64; ++i) {
        auto opt = m.get(i);
        assert(opt.is_some());
    }

    // CPU_RELAX must be defined and callable when the macro path is
    // active. The body does nothing observable; we just need it to
    // link.
    for (int i = 0; i < 8; ++i) {
        CPU_RELAX();
    }

    std::cout << "rusty_portable_intrinsics_test: OK\n";
    return 0;
}
