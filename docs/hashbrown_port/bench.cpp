// HashMap bench: transpiled hashbrown vs std::unordered_map.
// All ports use pre-allocated capacity (resize bug worked around).

#include <chrono>
#include <cstdint>
#include <cstdio>
#include <random>
#include <unordered_map>
#include <vector>
#include <rusty/rusty.hpp>

import hashbrown_port.raw;
import hashbrown_port.map;
import hashbrown_port.hasher;

constexpr int N = 100;  // inserts per iteration; small to fit growth_left

// generate a workload of N unique int keys + values once
struct Workload {
    std::vector<int> keys;
    std::vector<int> vals;
};

static Workload make_workload(int n, uint32_t seed) {
    std::mt19937 rng(seed);
    Workload w;
    w.keys.reserve(n);
    w.vals.reserve(n);
    for (int i = 0; i < n; ++i) {
        w.keys.push_back(i);
        w.vals.push_back(static_cast<int>(rng()));
    }
    // shuffle for non-monotone insert order
    std::shuffle(w.keys.begin(), w.keys.end(), rng);
    std::shuffle(w.vals.begin(), w.vals.end(), rng);
    return w;
}

template <typename F>
static double time_ns(F&& fn) {
    auto t0 = std::chrono::steady_clock::now();
    fn();
    auto t1 = std::chrono::steady_clock::now();
    return std::chrono::duration<double, std::nano>(t1 - t0).count();
}

int main() {
    constexpr int ROUNDS = 1000;
    auto w = make_workload(N, 42);

    // 1. Insert benchmark — preallocated capacity, no resize during loop.
    {
        double tot_hash = 0, tot_std = 0;
        int64_t guard = 0;

        for (int r = 0; r < ROUNDS; ++r) {
            // transpiled hashbrown
            tot_hash += time_ns([&] {
                auto m = HashMap<int, int>::with_capacity(N * 4);
                for (int i = 0; i < N; ++i) m.insert(w.keys[i], w.vals[i]);
                guard += static_cast<int64_t>(rusty::len(m));
            });
        }
        for (int r = 0; r < ROUNDS; ++r) {
            // std::unordered_map
            tot_std += time_ns([&] {
                std::unordered_map<int, int> m;
                m.reserve(N * 4);
                for (int i = 0; i < N; ++i) m.emplace(w.keys[i], w.vals[i]);
                guard += static_cast<int64_t>(m.size());
            });
        }

        double avg_hash = tot_hash / ROUNDS;
        double avg_std  = tot_std  / ROUNDS;
        std::printf("INSERT n=%d × %d rounds\n", N, ROUNDS);
        std::printf("  transpiled hashbrown : %8.0f ns/iter\n", avg_hash);
        std::printf("  std::unordered_map   : %8.0f ns/iter\n", avg_std);
        std::printf("  ratio (hb/std)       : %5.2fx\n", avg_hash / avg_std);
        std::printf("  guard=%ld\n", (long)guard);
    }

    // 2. Lookup benchmark — hot map, then N lookups.
    {
        double tot_hash = 0, tot_std = 0;
        int64_t guard = 0;

        // Prep
        auto m_hash = HashMap<int, int>::with_capacity(N * 4);
        std::unordered_map<int, int> m_std;
        m_std.reserve(N * 4);
        for (int i = 0; i < N; ++i) {
            m_hash.insert(w.keys[i], w.vals[i]);
            m_std.emplace(w.keys[i], w.vals[i]);
        }

        for (int r = 0; r < ROUNDS; ++r) {
            tot_hash += time_ns([&] {
                int64_t sum = 0;
                for (int i = 0; i < N; ++i) {
                    auto h = ::make_hash<int, DefaultHasher>(
                        m_hash.hash_builder, w.keys[i]);
                    auto b = m_hash.table.find(h, [&](const auto& kv) {
                        return std::get<0>(kv) == w.keys[i];
                    });
                    if (b.is_some()) sum += std::get<1>(b.unwrap().as_ref());
                }
                guard += sum;
            });
        }
        for (int r = 0; r < ROUNDS; ++r) {
            tot_std += time_ns([&] {
                int64_t sum = 0;
                for (int i = 0; i < N; ++i) {
                    auto it = m_std.find(w.keys[i]);
                    if (it != m_std.end()) sum += it->second;
                }
                guard += sum;
            });
        }

        double avg_hash = tot_hash / ROUNDS;
        double avg_std  = tot_std  / ROUNDS;
        std::printf("LOOKUP n=%d × %d rounds\n", N, ROUNDS);
        std::printf("  transpiled hashbrown : %8.0f ns/iter\n", avg_hash);
        std::printf("  std::unordered_map   : %8.0f ns/iter\n", avg_std);
        std::printf("  ratio (hb/std)       : %5.2fx\n", avg_hash / avg_std);
        std::printf("  guard=%ld\n", (long)guard);
    }

    return 0;
}
