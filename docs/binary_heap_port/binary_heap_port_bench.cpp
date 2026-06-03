// BinaryHeap bench: transpiled binary_heap_port vs std::priority_queue.
//
// Three operations, each averaged over ROUNDS iterations:
//   1. PUSH: push N elements into a fresh heap.
//   2. POP:  drain N elements (full pop loop) from a pre-filled heap.
//   3. MIX:  N/2 pushes interleaved with N/2 pops, on a partially-filled heap.
//
// Each iteration uses the same shuffled int workload (same seed). N is
// kept modest (≤ 10000) so std::priority_queue's vector reallocs don't
// dominate; both contenders reserve via with_capacity_in/reserve when
// available so steady-state dominates.

#include <chrono>
#include <cstdint>
#include <cstdio>
#include <queue>
#include <random>
#include <vector>

#include <rusty/alloc.hpp>

import binary_heap_port;

using rusty::collections::BinaryHeap;

constexpr int N = 10000;
constexpr int ROUNDS = 200;

struct Workload {
    std::vector<int> values;
};

static Workload make_workload(int n, uint32_t seed) {
    std::mt19937 rng(seed);
    Workload w;
    w.values.reserve(n);
    for (int i = 0; i < n; ++i) {
        w.values.push_back(static_cast<int>(rng()));
    }
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
    auto w = make_workload(N, 42);

    // ------------------------------------------------------------------
    // 1. PUSH: build a fresh heap of N elements.
    // ------------------------------------------------------------------
    {
        double tot_bhp = 0, tot_std = 0;
        int64_t guard = 0;

        for (int r = 0; r < ROUNDS; ++r) {
            tot_bhp += time_ns([&] {
                auto h = BinaryHeap<int32_t, ::rusty::alloc::Global>::
                    with_capacity_in(N, ::rusty::alloc::Global{});
                for (int v : w.values) h.push(v);
                guard += static_cast<int64_t>(h.len());
            });
        }
        for (int r = 0; r < ROUNDS; ++r) {
            tot_std += time_ns([&] {
                std::vector<int> backing;
                backing.reserve(N);
                std::priority_queue<int> q(std::less<int>{}, std::move(backing));
                for (int v : w.values) q.push(v);
                guard += static_cast<int64_t>(q.size());
            });
        }

        double avg_bhp = tot_bhp / ROUNDS;
        double avg_std = tot_std / ROUNDS;
        std::printf("PUSH n=%d x %d rounds\n", N, ROUNDS);
        std::printf("  transpiled BinaryHeap : %9.0f ns/iter\n", avg_bhp);
        std::printf("  std::priority_queue   : %9.0f ns/iter\n", avg_std);
        std::printf("  ratio (bhp/std)       : %5.2fx\n", avg_bhp / avg_std);
        std::printf("  guard=%ld\n\n", static_cast<long>(guard));
    }

    // ------------------------------------------------------------------
    // 2. POP: drain a pre-filled heap of N elements.
    // ------------------------------------------------------------------
    {
        double tot_bhp = 0, tot_std = 0;
        int64_t guard = 0;

        for (int r = 0; r < ROUNDS; ++r) {
            // Rebuild fresh per round so we time only the pop loop.
            auto h = BinaryHeap<int32_t, ::rusty::alloc::Global>::
                with_capacity_in(N, ::rusty::alloc::Global{});
            for (int v : w.values) h.push(v);
            tot_bhp += time_ns([&] {
                int sum = 0;
                while (!h.is_empty()) sum += h.pop().unwrap();
                guard += sum;
            });
        }
        for (int r = 0; r < ROUNDS; ++r) {
            std::vector<int> backing;
            backing.reserve(N);
            std::priority_queue<int> q(std::less<int>{}, std::move(backing));
            for (int v : w.values) q.push(v);
            tot_std += time_ns([&] {
                int sum = 0;
                while (!q.empty()) { sum += q.top(); q.pop(); }
                guard += sum;
            });
        }

        double avg_bhp = tot_bhp / ROUNDS;
        double avg_std = tot_std / ROUNDS;
        std::printf("POP  n=%d x %d rounds\n", N, ROUNDS);
        std::printf("  transpiled BinaryHeap : %9.0f ns/iter\n", avg_bhp);
        std::printf("  std::priority_queue   : %9.0f ns/iter\n", avg_std);
        std::printf("  ratio (bhp/std)       : %5.2fx\n", avg_bhp / avg_std);
        std::printf("  guard=%ld\n\n", static_cast<long>(guard));
    }

    // ------------------------------------------------------------------
    // 3. MIX: N/2 pushes alternating with N/2 pops on a half-full heap.
    // ------------------------------------------------------------------
    {
        double tot_bhp = 0, tot_std = 0;
        int64_t guard = 0;
        const int HALF = N / 2;

        for (int r = 0; r < ROUNDS; ++r) {
            auto h = BinaryHeap<int32_t, ::rusty::alloc::Global>::
                with_capacity_in(N, ::rusty::alloc::Global{});
            // Pre-fill to half capacity so push/pop are both hot.
            for (int i = 0; i < HALF; ++i) h.push(w.values[i]);
            tot_bhp += time_ns([&] {
                int sum = 0;
                for (int i = 0; i < HALF; ++i) {
                    h.push(w.values[HALF + i]);
                    sum += h.pop().unwrap();
                }
                guard += sum;
            });
        }
        for (int r = 0; r < ROUNDS; ++r) {
            std::vector<int> backing;
            backing.reserve(N);
            std::priority_queue<int> q(std::less<int>{}, std::move(backing));
            for (int i = 0; i < HALF; ++i) q.push(w.values[i]);
            tot_std += time_ns([&] {
                int sum = 0;
                for (int i = 0; i < HALF; ++i) {
                    q.push(w.values[HALF + i]);
                    sum += q.top();
                    q.pop();
                }
                guard += sum;
            });
        }

        double avg_bhp = tot_bhp / ROUNDS;
        double avg_std = tot_std / ROUNDS;
        std::printf("MIX  n=%d x %d rounds (HALF=%d push/pop pairs)\n",
                    N, ROUNDS, HALF);
        std::printf("  transpiled BinaryHeap : %9.0f ns/iter\n", avg_bhp);
        std::printf("  std::priority_queue   : %9.0f ns/iter\n", avg_std);
        std::printf("  ratio (bhp/std)       : %5.2fx\n", avg_bhp / avg_std);
        std::printf("  guard=%ld\n\n", static_cast<long>(guard));
    }

    return 0;
}
