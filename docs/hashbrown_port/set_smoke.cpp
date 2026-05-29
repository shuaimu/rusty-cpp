// HashSet smoke test — exercises the facade in hashbrown_port.set.cppm.
// Same shape as smoke_test.cpp's HashMap coverage, adapted for the set
// API: insert returns bool (newly-inserted?), contains/remove for
// membership, plus growth via new_() at scale.

#include <cstdint>
#include <cstdio>
#include <rusty/rusty.hpp>

import hashbrown_port.set;
import hashbrown_port.map;
import hashbrown_port.hasher;

int main() {
    // Step 1: ctor + capacity
    auto s = HashSet<int>::with_capacity(64);
    if (s.len() != 0) { std::puts("FAIL: initial len"); return 1; }
    if (!s.is_empty()) { std::puts("FAIL: initial empty"); return 1; }
    std::printf("step 1: with_capacity(64) -> cap=%zu len=%zu\n", s.capacity(), s.len());

    // Step 2: insert + contains, duplicate semantics
    bool ins1 = s.insert(42);
    bool ins2 = s.insert(42);
    if (!ins1) { std::puts("FAIL: first insert(42) should return true"); return 1; }
    if (ins2)  { std::puts("FAIL: duplicate insert(42) should return false"); return 1; }
    if (!s.contains(42)) { std::puts("FAIL: contains(42)"); return 1; }
    if (s.contains(99))  { std::puts("FAIL: spurious contains(99)"); return 1; }
    if (s.len() != 1) { std::puts("FAIL: len after duplicate insert"); return 1; }
    std::printf("step 2: insert(42) twice -> len=%zu (expected 1)\n", s.len());

    // Step 3: bulk insert + verify
    for (int i = 0; i < 100; ++i) s.insert(i);
    if (s.len() != 100) { std::printf("FAIL: bulk len=%zu (expected 100)\n", s.len()); return 1; }
    int found = 0;
    for (int i = 0; i < 100; ++i) if (s.contains(i)) ++found;
    if (found != 100) { std::printf("FAIL: bulk contains found=%d/100\n", found); return 1; }
    std::printf("step 3: 100 inserts -> len=%zu found=%d\n", s.len(), found);

    // Step 4: remove
    if (!s.remove(50)) { std::puts("FAIL: remove(50)"); return 1; }
    if (s.contains(50)) { std::puts("FAIL: still contains(50) after remove"); return 1; }
    if (s.remove(50))  { std::puts("FAIL: second remove(50) should return false"); return 1; }
    if (s.len() != 99) { std::printf("FAIL: len after remove=%zu (expected 99)\n", s.len()); return 1; }
    std::printf("step 4: remove(50) -> len=%zu\n", s.len());

    // Step 5: clear
    s.clear();
    if (s.len() != 0)  { std::puts("FAIL: len after clear"); return 1; }
    if (!s.is_empty()) { std::puts("FAIL: is_empty after clear"); return 1; }
    if (s.contains(0)) { std::puts("FAIL: contains after clear"); return 1; }
    std::printf("step 5: clear -> len=%zu empty=%d\n", s.len(), (int)s.is_empty());

    // Step 6: growth via new_() — same path as HashMap step 6c.
    auto s2 = HashSet<int>::new_();
    for (int i = 0; i < 1000; ++i) s2.insert(i * 3);
    if (s2.len() != 1000) { std::printf("FAIL: growth len=%zu\n", s2.len()); return 1; }
    int hits = 0;
    for (int i = 0; i < 1000; ++i) if (s2.contains(i * 3)) ++hits;
    if (hits != 1000) { std::printf("FAIL: growth hits=%d/1000\n", hits); return 1; }
    std::printf("step 6: new_() + 1000 inserts -> len=%zu hits=%d\n", s2.len(), hits);

    std::puts("set_smoke passed");
    return 0;
}
