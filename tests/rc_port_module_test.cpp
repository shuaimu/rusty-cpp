// Smoke test for rc_port (Phase B/C bridge — see transpiled/rc_port/
// rc_port_stub.cppm). The transpiled body isn't reachable yet, so this
// only exercises the bridge surface (constructing an Rc, basic ref
// counting via the hand-written rusty::Rc).

import rc_port;

#include <rusty/rc.hpp>
#include <cassert>
#include <cstdio>

int main() {
    rc_port::Rc<int> p = rc_port::Rc<int>::make(42);
    assert(*p == 42);

    rc_port::Rc<int> p2 = p;  // clone
    assert(*p2 == 42);
    assert(*p == 42);

    std::printf("rc_port (stub bridge) smoke OK: Rc<int>(42) + clone\n");
    return 0;
}
