#include <cstdio>
#include <vector>
int main() {
    constexpr size_t N = 1'000'000;
    std::vector<int> v;
    v.reserve(N);
    for (size_t i = 0; i < N; ++i) v.push_back(static_cast<int>(i));
    std::printf("len=%zu\n", v.size());
    return 0;
}
