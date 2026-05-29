// Phase B/C smoke test for hashbrown_port::HashMap.
// Build: configured by CMakeLists.txt patch via post_transpile_patch.py.
//
// Minimal first iteration — just instantiate HashMap<int, int> and
// let the default constructor + destructor run. Catches the most
// common compile + RAII issues without touching the full insert
// / lookup paths.

#include <cstdint>
#include <cstdio>
#include <rusty/rusty.hpp>

import hashbrown_port.raw;
import hashbrown_port.map;

int main() {
    // Step 1: default-construct an empty HashMap.
    auto m = HashMap<int, int>::new_();
    std::puts("smoke step 1: HashMap<int, int>::new_() — constructed");
    (void)m;

    // Step 2: with_capacity(16) — exercises the alloc path.
    try {
        auto m2 = HashMap<int, int>::with_capacity(16);
        std::puts("smoke step 2: with_capacity(16) — constructed");
        (void)m2;
    } catch (const std::exception& e) {
        std::printf("smoke step 2: FAILED with: %s\n", e.what());
        return 1;
    }

    std::puts("smoke test passed");
    return 0;
}
