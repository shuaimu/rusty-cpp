// Partial drain test — was broken before the rusty::Vec layout
// mismatch was fixed.
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
    CHECK(v.len() == 5, "initial len 5");

    {
        auto d = v.drain(rusty::range(static_cast<size_t>(0), static_cast<size_t>(2)));
        int count = 0;
        int expected[2] = {10, 20};
        while (true) {
            auto next = d.next();
            if (!next.is_some()) break;
            int val = next.unwrap();
            CHECK(count < 2, "drain yielded too many");
            CHECK(val == expected[count], "drain value");
            count++;
        }
        CHECK(count == 2, "drained exactly 2");
    }
    CHECK(v.len() == 3, "len 3 after partial drain");
    auto s = v.as_slice();
    CHECK(s[0] == 30, "v[0]=30 (tail shifted)");
    CHECK(s[1] == 40, "v[1]=40");
    CHECK(s[2] == 50, "v[2]=50");
    std::printf("ALL CHECKS PASSED\n");
    return 0;
}
