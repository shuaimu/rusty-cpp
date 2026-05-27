// Vec<rusty::String>: tests Drop chain through Vec destructor for the
// other non-trivial RAII type in the rusty library.
import vec_port;
#include <cstdio>
#include <rusty/rusty.hpp>

#define CHECK(cond, msg) do { \
    if (!(cond)) { std::printf("FAIL: %s\n", msg); std::exit(1); } \
} while (0)

int main() {
    {
        auto v = Vec<rusty::String, rusty::alloc::Global>::new_in(rusty::alloc::Global{});
        v.push(rusty::String("hello"));
        v.push(rusty::String("world"));
        v.push(rusty::String("from rusty vec"));
        CHECK(v.len() == 3, "len");
        auto s = v.as_slice();
        // String's c_str() / as_str() — try both
        std::printf("  [0] = %s\n", s[0].c_str());
        std::printf("  [1] = %s\n", s[1].c_str());
        std::printf("  [2] = %s\n", s[2].c_str());

        // Force a realloc by pushing more
        for (int i = 0; i < 10; ++i) {
            char buf[32];
            std::snprintf(buf, sizeof(buf), "elem_%d", i);
            v.push(rusty::String(buf));
        }
        CHECK(v.len() == 13, "len after realloc");
        std::printf("  [last] = %s\n", v.as_slice()[12].c_str());
    }
    std::printf("ALL CHECKS PASSED\n");
    return 0;
}
