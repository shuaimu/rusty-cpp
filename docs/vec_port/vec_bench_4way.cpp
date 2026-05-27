// 4-way Vec bench: vec_port::Vec (transpiled) vs rusty::VecLegacy
// (hand-written) vs std::vector. The Rust std::Vec measurement
// lives in a sibling crate.
//
// Workload: push (grow), push (reserved), iterate, index.
// Trials: 5 per workload per runtime. Reports mean ms.

import vec_port;

#include <chrono>
#include <cstdint>
#include <cstdio>
#include <vector>

#include <rusty/rusty.hpp>
#include <rusty/vec.hpp>

static constexpr size_t N = 10'000'000;
static constexpr int REPEATS = 5;

using clock_t_ = std::chrono::steady_clock;
static inline double ms_since(clock_t_::time_point t0) {
    return std::chrono::duration<double, std::milli>(clock_t_::now() - t0).count();
}

// --- push (grow from empty) ----------------------------------------

double bench_transpiled_push_grow() {
    auto t0 = clock_t_::now();
    auto v = Vec<int, rusty::alloc::Global>::new_in(rusty::alloc::Global{});
    for (size_t i = 0; i < N; ++i) v.push(static_cast<int>(i));
    return ms_since(t0);
}
double bench_legacy_push_grow() {
    auto t0 = clock_t_::now();
    auto v = rusty::VecLegacy<int>::new_();
    for (size_t i = 0; i < N; ++i) v.push(static_cast<int>(i));
    return ms_since(t0);
}
double bench_std_push_grow() {
    auto t0 = clock_t_::now();
    std::vector<int> v;
    for (size_t i = 0; i < N; ++i) v.push_back(static_cast<int>(i));
    return ms_since(t0);
}

// --- push (reserved) ----------------------------------------------

double bench_transpiled_push_reserved() {
    auto t0 = clock_t_::now();
    auto v = Vec<int, rusty::alloc::Global>::with_capacity_in(N, rusty::alloc::Global{});
    for (size_t i = 0; i < N; ++i) v.push(static_cast<int>(i));
    return ms_since(t0);
}
double bench_legacy_push_reserved() {
    auto t0 = clock_t_::now();
    auto v = rusty::VecLegacy<int>::with_capacity(N);
    for (size_t i = 0; i < N; ++i) v.push(static_cast<int>(i));
    return ms_since(t0);
}
double bench_std_push_reserved() {
    auto t0 = clock_t_::now();
    std::vector<int> v;
    v.reserve(N);
    for (size_t i = 0; i < N; ++i) v.push_back(static_cast<int>(i));
    return ms_since(t0);
}

// --- iterate (sum) -------------------------------------------------

double bench_transpiled_iter(volatile int64_t& sink) {
    auto v = Vec<int, rusty::alloc::Global>::with_capacity_in(N, rusty::alloc::Global{});
    for (size_t i = 0; i < N; ++i) v.push(static_cast<int>(i));
    auto t0 = clock_t_::now();
    int64_t s = 0;
    auto sp = v.as_slice();
    for (size_t i = 0; i < sp.size(); ++i) s += sp[i];
    auto ms = ms_since(t0);
    sink = s;
    return ms;
}
double bench_legacy_iter(volatile int64_t& sink) {
    auto v = rusty::VecLegacy<int>::with_capacity(N);
    for (size_t i = 0; i < N; ++i) v.push(static_cast<int>(i));
    auto t0 = clock_t_::now();
    int64_t s = 0;
    for (size_t i = 0; i < v.size(); ++i) s += v[i];
    auto ms = ms_since(t0);
    sink = s;
    return ms;
}
double bench_std_iter(volatile int64_t& sink) {
    std::vector<int> v;
    v.reserve(N);
    for (size_t i = 0; i < N; ++i) v.push_back(static_cast<int>(i));
    auto t0 = clock_t_::now();
    int64_t s = 0;
    for (size_t i = 0; i < v.size(); ++i) s += v[i];
    auto ms = ms_since(t0);
    sink = s;
    return ms;
}

// --- index (sum via operator[]) -----------------------------------
// Same shape as iter, but exercises a different access path on each
// runtime (operator[] for Vec/std::vector, as_slice()[i] vs sp[i] on
// transpiled). For std::vector and VecLegacy this is identical to
// iter; for vec_port::Vec it differs because indexing goes through
// operator[]/index instead of as_slice indexing.

double bench_transpiled_index(volatile int64_t& sink) {
    auto v = Vec<int, rusty::alloc::Global>::with_capacity_in(N, rusty::alloc::Global{});
    for (size_t i = 0; i < N; ++i) v.push(static_cast<int>(i));
    auto t0 = clock_t_::now();
    int64_t s = 0;
    for (size_t i = 0; i < N; ++i) s += v[i];
    auto ms = ms_since(t0);
    sink = s;
    return ms;
}
double bench_legacy_index(volatile int64_t& sink) {
    auto v = rusty::VecLegacy<int>::with_capacity(N);
    for (size_t i = 0; i < N; ++i) v.push(static_cast<int>(i));
    auto t0 = clock_t_::now();
    int64_t s = 0;
    for (size_t i = 0; i < N; ++i) s += v[i];
    auto ms = ms_since(t0);
    sink = s;
    return ms;
}
double bench_std_index(volatile int64_t& sink) {
    std::vector<int> v;
    v.reserve(N);
    for (size_t i = 0; i < N; ++i) v.push_back(static_cast<int>(i));
    auto t0 = clock_t_::now();
    int64_t s = 0;
    for (size_t i = 0; i < N; ++i) s += v[i];
    auto ms = ms_since(t0);
    sink = s;
    return ms;
}

// --- driver --------------------------------------------------------

struct Row {
    const char* name;
    double t_pg, t_pr, t_it, t_ix;
};

int main() {
    std::printf("4-way Vec bench: N=%zu int pushes, %d trials each\n", N, REPEATS);
    std::printf("(workload-side measurement; sibling Rust binary covers std::Vec)\n\n");

    volatile int64_t sink = 0;
    Row rows[3]{
        {"vec_port::Vec   (transpiled)", 0, 0, 0, 0},
        {"rusty::VecLegacy (hand-written)", 0, 0, 0, 0},
        {"std::vector      (libstdc++)", 0, 0, 0, 0},
    };
    for (int trial = 0; trial < REPEATS; ++trial) {
        rows[0].t_pg += bench_transpiled_push_grow();
        rows[1].t_pg += bench_legacy_push_grow();
        rows[2].t_pg += bench_std_push_grow();

        rows[0].t_pr += bench_transpiled_push_reserved();
        rows[1].t_pr += bench_legacy_push_reserved();
        rows[2].t_pr += bench_std_push_reserved();

        rows[0].t_it += bench_transpiled_iter(sink);
        rows[1].t_it += bench_legacy_iter(sink);
        rows[2].t_it += bench_std_iter(sink);

        rows[0].t_ix += bench_transpiled_index(sink);
        rows[1].t_ix += bench_legacy_index(sink);
        rows[2].t_ix += bench_std_index(sink);
    }

    std::printf("                                  push-grow  push-reserved   iterate     index\n");
    for (auto& r : rows) {
        std::printf("%-32s  %8.2f ms   %8.2f ms %8.2f ms %8.2f ms\n",
            r.name,
            r.t_pg / REPEATS, r.t_pr / REPEATS,
            r.t_it / REPEATS, r.t_ix / REPEATS);
    }
    std::printf("\n(sink=%lld — defeats DCE)\n", (long long)sink);
    return 0;
}
