// Smoke test for arc_port (Phase B/C bridge module).
import arc_port;

#include <rusty/arc.hpp>
#include <cassert>
#include <cstdio>

int main() {
    arc_port::Arc<int> p = arc_port::Arc<int>::make(7);
    assert(*p == 7);
    arc_port::Arc<int> p2 = p;
    assert(*p2 == 7);
    std::printf("arc_port (stub bridge) smoke OK: Arc<int>(7) + clone\n");
    return 0;
}
