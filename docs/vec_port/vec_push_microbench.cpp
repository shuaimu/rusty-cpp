// Single-purpose push microbench — for callgrind.
import vec_port;
#include <cstdio>
#include <rusty/rusty.hpp>

int main() {
    constexpr size_t N = 1'000'000;
    auto v = Vec<int, rusty::alloc::Global>::with_capacity_in(N, rusty::alloc::Global{});
    for (size_t i = 0; i < N; ++i) v.push(static_cast<int>(i));
    std::printf("len=%zu\n", v.len());
    return 0;
}
