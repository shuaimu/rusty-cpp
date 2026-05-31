// Smoke test for vec_deque_port (Phase B/C bridge module).
import vec_deque_port;

#include <rusty/vecdeque.hpp>
#include <cassert>
#include <cstdio>

int main() {
    vec_deque_port::VecDeque<int> q;
    q.push_back(1);
    q.push_back(2);
    q.push_back(3);
    assert(q.size() == 3);
    std::printf("vec_deque_port (stub bridge) smoke OK: size=%zu\n",
                static_cast<size_t>(q.size()));
    return 0;
}
