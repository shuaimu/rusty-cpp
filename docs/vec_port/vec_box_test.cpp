// Vec<Box<int>>: tests Drop chain through Vec destructor for non-trivial T
import vec_port;
#include <cstdio>
#include <rusty/rusty.hpp>

#define CHECK(cond, msg) do { \
    if (!(cond)) { std::printf("FAIL: %s\n", msg); std::exit(1); } \
} while (0)

int main() {
    {
        auto v = Vec<rusty::Box<int>, rusty::alloc::Global>::new_in(rusty::alloc::Global{});
        v.push(rusty::Box<int>::make(10));
        v.push(rusty::Box<int>::make(20));
        v.push(rusty::Box<int>::make(30));
        CHECK(v.len() == 3, "len");
        // Read values through as_slice + deref
        auto s = v.as_slice();
        CHECK(*s[0] == 10, "[0] deref == 10");
        CHECK(*s[1] == 20, "[1] deref == 20");
        CHECK(*s[2] == 30, "[2] deref == 30");
        std::printf("Vec<Box<int>>: 3 boxes pushed, values read OK\n");
        // Destructor of v runs here — each Box should free its int
    }
    std::printf("ALL CHECKS PASSED\n");
    return 0;
}
