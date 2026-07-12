import alloc;
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

    // Vec::into_iter + IntoIter::next (consuming iteration)
    auto v3 = VI::new_(); v3.push(1); v3.push(2); v3.push(3);
    { long sum=0; int cnt=0; auto it=v3.into_iter();
      while(true){auto n=it.next(); if(!n.is_some())break; sum+=n.unwrap(); ++cnt;}
      assert(cnt==3 && sum==6); }

    // Vec range-for + std algorithms (needs begin/end members)
    { auto vr = VI::new_(); vr.push(3); vr.push(7); vr.push(9);
      long s2=0; for (const auto& x : vr) s2 += x; assert(s2==19); }

    // BinaryHeap: max-order pop
    { auto h = collections::binary_heap::BinaryHeap<int>::with_capacity(4);
      h.push(3); h.push(9); h.push(1); h.push(7);
      assert(h.pop().unwrap()==9 && h.pop().unwrap()==7 && h.pop().unwrap()==3 && h.pop().unwrap()==1);
      assert(!h.pop().is_some()); }

    // LinkedList: push/pop both ends, order preserved
    { auto l = collections::linked_list::LinkedList<int>::new_();
      l.push_back(2); l.push_front(1); l.push_back(3);
      assert(l.len()==3);
      assert(l.pop_front().unwrap()==1);
      assert(l.pop_back().unwrap()==3);
      assert(l.pop_front().unwrap()==2); }

    std::printf("alloc BROAD runtime OK (Vec+VecDeque+conv+BinaryHeap+LinkedList)\n");
    return 0;
}
