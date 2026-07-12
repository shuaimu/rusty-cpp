import alloc_port;
#include <cassert>
#include <utility>
#include <cstdio>
using VI = vec::Vec<int>;
using DQ = collections::vec_deque::VecDeque<int>;
int main() {
    // Vec core (known green)
    auto v = VI::new_(); v.push(1); v.push(2); v.push(3);
    assert(v.len()==3);
    // VecDeque core (newly green)
    auto dq = DQ::with_capacity(4);
    dq.push_back(10); dq.push_front(5); dq.push_back(20);
    { const auto& r=dq; assert(r[0]==5 && r[1]==10 && r[2]==20); }
    assert(dq.pop_front().unwrap()==5);
    assert(dq.pop_back().unwrap()==20);
    assert(dq.len()==1);
    // VecDeque iteration
    { auto it = dq.iter(); (void)it; }
    // cross-conversion: VecDeque -> Vec
    auto dq2 = DQ::with_capacity(3); dq2.push_back(7); dq2.push_back(8); dq2.push_back(9);
    auto fromdq = VI::from(std::move(dq2));
    { const auto& r=fromdq; assert(r[0]==7 && r[1]==8 && r[2]==9); }
    // cross-conversion: Vec -> VecDeque
    auto v2 = VI::new_(); v2.push(100); v2.push(200);
    auto fromv = DQ::from(std::move(v2));
    assert(fromv.len()==2);
    { const auto& r=fromv; assert(r[0]==100 && r[1]==200); }
    std::printf("alloc_port BROAD runtime OK\n");
    return 0;
}
