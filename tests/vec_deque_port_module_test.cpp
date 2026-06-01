// Smoke test for vec_deque_port (full transpiled body — vendored
// rustc `library/alloc/src/collections/vec_deque/` via collapse +
// post_transpile_patch). Exercises new_/push_back/len through the
// real ring-buffer code path.
import vec_deque_port;

#include <rusty/rusty.hpp>
#include <cassert>
#include <cstdio>

int main() {
    auto q = vec_deque_port::VecDeque<int>::new_();
    q.push_back(1);
    q.push_back(2);
    q.push_back(3);
    assert(q.len() == 3);
    std::printf("vec_deque_port (transpiled) smoke OK: len=%zu\n",
                static_cast<size_t>(q.len()));
    return 0;
}
