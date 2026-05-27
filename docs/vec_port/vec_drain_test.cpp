// Test Vec::drain(range)
import vec_port;
#include <cstdio>
#include <cstdlib>
#include <rusty/rusty.hpp>

#define CHECK(cond, msg) do { \
    if (!(cond)) { std::printf("FAIL: %s\n", msg); std::exit(1); } \
} while (0)

int main() {
    auto v = Vec<int, rusty::alloc::Global>::new_in(rusty::alloc::Global{});
    for (int i = 1; i <= 5; ++i) v.push(i * 10);
    CHECK(v.len() == 5, "len 5 before drain");

    // drain(0..) should drain all elements
    {
        auto d = v.drain(rusty::range_from(static_cast<size_t>(0)));
        int sum = 0; int count = 0;
        while (true) {
            auto next = d.next();
            if (next.is_none()) break;
            sum += next.unwrap();
            count++;
        }
        CHECK(count == 5, "drained 5");
        CHECK(sum == 150, "sum 150 (10+20+30+40+50)");
        std::printf("drain(0..) yielded %d elements, sum=%d\n", count, sum);
    }
    // After drain dropped, vec should be empty
    CHECK(v.len() == 0, "vec empty after drain");

    std::printf("ALL CHECKS PASSED\n");
    return 0;
}
