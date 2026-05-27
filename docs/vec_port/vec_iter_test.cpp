// Test Vec::into_iter() — call next() repeatedly to drain the vec.
import vec_port;
#include <cstdio>
#include <cstdlib>
#include <rusty/rusty.hpp>

#define CHECK(cond, msg) do { \
    if (!(cond)) { std::printf("FAIL: %s\n", msg); std::exit(1); } \
} while (0)

int main() {
    auto v = Vec<int, rusty::alloc::Global>::new_in(rusty::alloc::Global{});
    v.push(100); v.push(200); v.push(300);
    CHECK(v.len() == 3, "len 3 before iter");

    auto it = std::move(v).into_iter();
    int count = 0; int sum = 0;
    while (true) {
        auto next = it.next();
        if (next.is_none()) break;
        sum += next.unwrap();
        count++;
    }
    CHECK(count == 3, "iterated 3 elements");
    CHECK(sum == 600, "sum is 600");
    std::printf("iter result: count=%d sum=%d\n", count, sum);

    std::printf("ALL CHECKS PASSED\n");
    return 0;
}
