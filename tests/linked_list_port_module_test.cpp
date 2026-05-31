// Smoke test for linked_list_port (Phase B/C bridge — wraps std::list).
import linked_list_port;

#include <list>
#include <cassert>
#include <cstdio>

int main() {
    linked_list_port::LinkedList<int> ll;
    ll.push_back(1);
    ll.push_back(2);
    ll.push_front(0);
    assert(ll.size() == 3);
    assert(ll.front() == 0);
    assert(ll.back() == 2);
    std::printf("linked_list_port (stub bridge) smoke OK: front=%d back=%d size=%zu\n",
                ll.front(), ll.back(), ll.size());
    return 0;
}
