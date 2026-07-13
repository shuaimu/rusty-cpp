import vec_port;

#include <cstdio>
#include <cstdlib>
#include <rusty/rusty.hpp>

#define CHECK(cond, msg)                                                      \
    do {                                                                      \
        if (!(cond)) {                                                        \
            std::printf("FAIL: %s\n", msg);                                  \
            std::exit(1);                                                     \
        }                                                                     \
    } while (0)

int main() {
    auto v = Vec<int, rusty::alloc::Global>::new_in(rusty::alloc::Global{});
    for (int i = 1; i <= 6; ++i) {
        v.push(i);
    }

    {
        auto ei = v.extract_if(rusty::range_from(static_cast<size_t>(0)),
                               [](int& x) { return x % 2 == 0; });
        int count = 0;
        int expected[3] = {2, 4, 6};
        while (true) {
            auto next = ei.next();
            if (next.is_none()) {
                break;
            }
            int val = next.unwrap();
            CHECK(count < 3, "extract_if yielded too many values");
            CHECK(val == expected[count], "extract_if yielded expected value");
            ++count;
        }
        CHECK(count == 3, "extract_if extracted exactly three values");
    }

    auto s = v.as_slice();
    CHECK(s.size() == 3, "extract_if left three values behind");
    CHECK(s[0] == 1 && s[1] == 3 && s[2] == 5,
          "extract_if kept the non-matching values");

    std::printf("ALL CHECKS PASSED\n");
    return 0;
}
