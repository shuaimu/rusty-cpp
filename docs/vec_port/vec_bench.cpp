// Quick bench: transpiled Vec<int> push vs std::vector<int> push.
import vec_port;
#include <chrono>
#include <cstdio>
#include <vector>
#include <rusty/rusty.hpp>

static constexpr size_t N = 10'000'000;
static constexpr int REPEATS = 5;

double bench_vec_push() {
    using clock = std::chrono::steady_clock;
    auto t0 = clock::now();
    auto v = Vec<int, rusty::alloc::Global>::new_in(rusty::alloc::Global{});
    for (size_t i = 0; i < N; ++i) v.push(static_cast<int>(i));
    auto t1 = clock::now();
    return std::chrono::duration<double, std::milli>(t1 - t0).count();
}

double bench_std_vector_push() {
    using clock = std::chrono::steady_clock;
    auto t0 = clock::now();
    std::vector<int> v;
    for (size_t i = 0; i < N; ++i) v.push_back(static_cast<int>(i));
    auto t1 = clock::now();
    return std::chrono::duration<double, std::milli>(t1 - t0).count();
}

double bench_vec_push_reserved() {
    using clock = std::chrono::steady_clock;
    auto t0 = clock::now();
    auto v = Vec<int, rusty::alloc::Global>::with_capacity_in(N, rusty::alloc::Global{});
    for (size_t i = 0; i < N; ++i) v.push(static_cast<int>(i));
    auto t1 = clock::now();
    return std::chrono::duration<double, std::milli>(t1 - t0).count();
}

double bench_std_vector_push_reserved() {
    using clock = std::chrono::steady_clock;
    auto t0 = clock::now();
    std::vector<int> v;
    v.reserve(N);
    for (size_t i = 0; i < N; ++i) v.push_back(static_cast<int>(i));
    auto t1 = clock::now();
    return std::chrono::duration<double, std::milli>(t1 - t0).count();
}

int main() {
    std::printf("Benchmark: %zu int pushes, %d trials each\n\n", N, REPEATS);

    double vec_total = 0, std_total = 0, vec_r_total = 0, std_r_total = 0;
    for (int i = 0; i < REPEATS; ++i) {
        vec_total += bench_vec_push();
        std_total += bench_std_vector_push();
        vec_r_total += bench_vec_push_reserved();
        std_r_total += bench_std_vector_push_reserved();
    }

    std::printf("Vec::push (grow path):                   avg %.2f ms\n", vec_total / REPEATS);
    std::printf("std::vector::push_back (grow path):      avg %.2f ms\n", std_total / REPEATS);
    std::printf("Vec::push (reserved):                    avg %.2f ms\n", vec_r_total / REPEATS);
    std::printf("std::vector::push_back (reserved):       avg %.2f ms\n", std_r_total / REPEATS);

    double r1 = (vec_total / std_total - 1.0) * 100.0;
    double r2 = (vec_r_total / std_r_total - 1.0) * 100.0;
    std::printf("\nVec vs std::vector overhead (grow):     %+.1f%%\n", r1);
    std::printf("Vec vs std::vector overhead (reserved): %+.1f%%\n", r2);
    return 0;
}
