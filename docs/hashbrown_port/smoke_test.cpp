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
import hashbrown_port.hasher;

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

    // Step 3: insert(42, 100) — exercises hash + ctrl-tag write path.
    try {
        auto m3 = HashMap<int, int>::with_capacity(16);
        m3.insert(42, 100);
        std::puts("smoke step 3: insert(42, 100) — completed");
    } catch (const std::exception& e) {
        std::printf("smoke step 3: FAILED with: %s\n", e.what());
        return 1;
    }

    // Step 4: insert + len — verify table tracks size.
    try {
        auto m4 = HashMap<int, int>::with_capacity(16);
        m4.insert(1, 10);
        m4.insert(2, 20);
        m4.insert(3, 30);
        size_t len = rusty::len(m4);
        std::printf("smoke step 4: 3 inserts; len=%zu\n", len);
        if (len != 3) { std::puts("FAIL: expected len=3"); return 1; }
    } catch (const std::exception& e) {
        std::printf("smoke step 4: FAILED with: %s\n", e.what());
        return 1;
    }

    // Step 5: insert + lookup roundtrip via table.find directly.
    // (HashMap doesn't expose a clean .get(K); we use the raw table API
    // since that's what the bench / production callers would use.)
    try {
        auto m5 = HashMap<int, int>::with_capacity(16);
        m5.insert(42, 1000);
        m5.insert(99, 2000);
        auto hash42 = ::make_hash<int, DefaultHasher>(m5.hash_builder, 42);
        auto bucket = m5.table.find(hash42, [](const auto& kv) {
            return std::get<0>(kv) == 42;
        });
        if (!bucket.is_some()) { std::puts("FAIL: bucket missing"); return 1; }
        auto& kv = bucket.unwrap().as_ref();
        std::printf("smoke step 5: find(42) → %d\n", std::get<1>(kv));
        if (std::get<1>(kv) != 1000) { std::puts("FAIL: wrong value"); return 1; }
    } catch (const std::exception& e) {
        std::printf("smoke step 5: FAILED with: %s\n", e.what());
        return 1;
    }

    // Step 6: growth — insert past initial capacity, force resize.
    // Start small to isolate behavior.
    try {
        auto m6a = HashMap<int, int>::with_capacity(64);
        for (int i = 0; i < 20; ++i) {
            m6a.insert(i, i * 7);
        }
        size_t len_a = rusty::len(m6a);
        int found_a = 0;
        for (int i = 0; i < 20; ++i) {
            auto h = ::make_hash<int, DefaultHasher>(m6a.hash_builder, i);
            auto b = m6a.table.find(h, [i](const auto& kv) {
                return std::get<0>(kv) == i;
            });
            if (b.is_some()) found_a++;
        }
        std::printf("smoke step 6a: 20 inserts cap=64; len=%zu found=%d\n",
                    len_a, found_a);
        if (len_a != 20 || found_a != 20) {
            std::puts("FAIL: small batch lost entries");
            return 1;
        }

        // 6b: 1000 inserts with sufficient pre-allocated capacity.
        {
            auto m6b = HashMap<int, int>::with_capacity(2048);
            for (int i = 0; i < 1000; ++i) m6b.insert(i, i * 7);
            size_t len_b = rusty::len(m6b);
            std::printf("smoke step 6b: 1000 inserts cap=2048; len=%zu\n", len_b);
            if (len_b != 1000) { std::puts("FAIL: len_b != 1000"); return 1; }
        }
        // 6c: 1000 inserts with new_() (triggers multiple resizes).
        auto m6 = HashMap<int, int>::new_();
        for (int i = 0; i < 1000; ++i) {
            m6.insert(i, i * 7);
        }
        size_t len = rusty::len(m6);
        std::printf("smoke step 6: 1000 inserts; len=%zu\n", len);
        if (len != 1000) { std::puts("FAIL: expected len=1000"); return 1; }

        // Verify a sample of values.
        int misses = 0;
        for (int i = 0; i < 1000; i += 37) {
            auto h = ::make_hash<int, DefaultHasher>(m6.hash_builder, i);
            auto b = m6.table.find(h, [i](const auto& kv) {
                return std::get<0>(kv) == i;
            });
            if (!b.is_some() || std::get<1>(b.unwrap().as_ref()) != i * 7) {
                misses++;
            }
        }
        std::printf("smoke step 6: lookup-sample misses=%d (of 28)\n", misses);
        if (misses != 0) { std::puts("FAIL: lookup miss after growth"); return 1; }
    } catch (const std::exception& e) {
        std::printf("smoke step 6: FAILED with: %s\n", e.what());
        return 1;
    }

    std::puts("smoke test passed");
    return 0;
}
