// IntoIter on Vec<Box<int>>: partial drain then destruct must clean up the rest.
import vec_port;
#include <cstdio>
#include <cstdlib>
#include <rusty/rusty.hpp>

#define CHECK(cond, msg) do { \
    if (!(cond)) { std::printf("FAIL: %s\n", msg); std::exit(1); } \
} while (0)

int main() {
    {
        auto v = Vec<rusty::Box<int>, rusty::alloc::Global>::new_in(rusty::alloc::Global{});
        v.push(rusty::Box<int>::make(11));
        v.push(rusty::Box<int>::make(22));
        v.push(rusty::Box<int>::make(33));
        v.push(rusty::Box<int>::make(44));
        v.push(rusty::Box<int>::make(55));

        auto it = std::move(v).into_iter();
        // Drain only the first 2 — then let ~IntoIter clean up the remaining 3.
        auto a = it.next();
        auto b = it.next();
        CHECK(a.is_some() && *a.unwrap() == 11, "first value");
        CHECK(b.is_some() && *b.unwrap() == 22, "second value");
        std::printf("partial drain OK; ~IntoIter must free remaining 3 boxes\n");
        // IntoIter goes out of scope here — its destructor must run drop_in_place
        // on the remaining range [ptr, end) which is 3 Box<int>s.
    }
    std::printf("ALL CHECKS PASSED\n");
    return 0;
}
