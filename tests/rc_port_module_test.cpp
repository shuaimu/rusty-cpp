// Smoke test for rc_port (full transpiled body from
// library/alloc/src/rc.rs via the docs/rc_port/ pipeline).
// Exercises Rc<int>::new_(42), strong_count, and clone.
import rc_port;

#include <rusty/rusty.hpp>
#include <cassert>
#include <cstdio>

int main() {
    auto p = rc_port::Rc<int>::new_(42);
    assert(rc_port::Rc<int>::strong_count(p) == 1);
    {
        auto p2 = p.clone();
        assert(rc_port::Rc<int>::strong_count(p) == 2);
        assert(rc_port::Rc<int>::strong_count(p2) == 2);
        (void)p2;
    }
    assert(rc_port::Rc<int>::strong_count(p) == 1);
    std::printf("rc_port (transpiled) smoke OK: Rc<int>::new_(42) + clone\n");
    return 0;
}
