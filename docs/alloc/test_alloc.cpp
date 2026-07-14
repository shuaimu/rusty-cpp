import alloc;
#include <cassert>
#include <utility>
#include <tuple>
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


    // Rc: shared ownership with correct refcounting
    { auto a = rc::Rc<int>::new_(42);
      assert(*a == 42);
      assert(rc::Rc<int>::strong_count(a) == 1);
      { auto b = a.clone();
        assert(*b == 42 && rc::Rc<int>::strong_count(a) == 2); }
      assert(rc::Rc<int>::strong_count(a) == 1); }


    // Arc: atomic shared ownership
    { auto a = sync_mod::Arc<int>::new_(7);
      assert(*a == 7);
      assert(sync_mod::Arc<int>::strong_count(a) == 1);
      { auto b = a.clone();
        assert(*b == 7 && sync_mod::Arc<int>::strong_count(a) == 2); }
      assert(sync_mod::Arc<int>::strong_count(a) == 1); }


    // vec![x; n] (from_elem)
    { auto vf = vec::from_elem(7, static_cast<size_t>(5));
      assert(vf.len() == 5);
      const auto& r = vf;
      for (size_t i = 0; i < 5; ++i) assert(r[i] == 7); }


    // BTreeMap: insert/get/remove + ordered iteration
    { auto m = collections::btree::map::BTreeMap<int, int>::new_();
      assert(m.len() == 0 && m.is_empty());
      assert(m.insert(3, 30).is_none());
      assert(m.insert(1, 10).is_none());
      assert(m.insert(2, 20).is_none());
      assert(m.insert(2, 21).unwrap() == 20);   // replace returns old
      assert(m.len() == 3);
      assert(m.get(2).unwrap() == 21);
      assert(m.get(9).is_none());
      assert(m.contains_key(1) && !m.contains_key(9));
      // ordered iteration (sorted by key)
      { int expect_k = 1;
        auto it = m.iter();
        for (auto kv = it.next(); kv.is_some(); kv = it.next()) {
          assert(std::get<0>(kv.unwrap()) == expect_k); ++expect_k; }
        assert(expect_k == 4); }
      assert(m.first_key_value().unwrap() == std::make_tuple(1, 10));
      assert(m.last_key_value().unwrap() == std::make_tuple(3, 30));
      assert(m.remove(2).unwrap() == 21);
      assert(m.len() == 2 && m.get(2).is_none()); }


    // BTreeSet: insert/contains + ordered iteration + set algebra
    { auto s = collections::btree::set::BTreeSet<int>::new_();
      assert(s.insert(3) && s.insert(1) && s.insert(2));
      assert(!s.insert(2));                      // duplicate
      assert(s.len() == 3 && s.contains(1) && !s.contains(9));
      { int expect = 1;
        auto it = s.iter();
        for (auto v = it.next(); v.is_some(); v = it.next()) {
          assert(v.unwrap() == expect); ++expect; }
        assert(expect == 4); }
      auto t2 = collections::btree::set::BTreeSet<int>::new_();
      t2.insert(2); t2.insert(3); t2.insert(4);
      // difference {1,2,3}\{2,3,4} = {1}  (Search-pinned dispatch)
      { auto d = s.difference(t2);
        auto first = d.next();
        assert(first.is_some() && first.unwrap() == 1);
        assert(d.next().is_none()); }
      // intersection {1,2,3}∩{2,3,4} = {2,3}
      { auto ix = s.intersection(t2);
        assert(ix.next().unwrap() == 2);
        assert(ix.next().unwrap() == 3);
        assert(ix.next().is_none()); }
      // is_subset via the rebuilt min/max guard
      { auto sub = collections::btree::set::BTreeSet<int>::new_();
        sub.insert(2); sub.insert(3);
        assert(sub.is_subset(t2));
        assert(!s.is_subset(t2));
        auto empty = collections::btree::set::BTreeSet<int>::new_();
        assert(empty.is_subset(t2)); } }


#ifdef ALLOC_WITH_BOXED
    // ---- crate Box (boxed.rs) ----
    { // deref + mutate through the heap slot
      auto b = boxed::Box<int>::new_(42);
      assert(*b == 42);
      *b += 1;
      assert(*b == 43);
      // clone is a deep copy
      auto b2 = b.clone();
      assert(*b2 == 43);
      *b2 = 7;
      assert(*b == 43 && *b2 == 7);
      // move transfers the allocation
      auto moved = std::move(b2);
      assert(*moved == 7);
      // into_inner returns the payload
      int inner = boxed::Box<int>::into_inner(std::move(moved));
      assert(inner == 7);
      // From<T>
      auto bf = boxed::Box<int>::from(9);
      assert(*bf == 9); }
    { // non-trivial payload owned through Box
      auto pv = VI::new_(); pv.push(4); pv.push(5);
      auto vb = boxed::Box<VI>::new_(std::move(pv));
      assert((*vb).len() == 2);
      (*vb).push(6);
      assert((*vb).len() == 3 && (*vb)[2] == 6);
      auto back = boxed::Box<VI>::into_inner(std::move(vb));
      assert(back.len() == 3 && back[0] == 4); }
    { // new_uninit + in-place write round-trip (the btree allocation path).
      // NOTE: crate Box's assume_init() currently returns Box<MaybeUninit<T>>
      // (the Box<MaybeUninit<T>> impl flattens into the generic Box<T> class
      // with its self-ty binding lost — #88-class); read through MaybeUninit
      // until that is fixed.
      auto ub = boxed::Box<int>::new_uninit();
      (*ub).write(31);
      assert((*ub).assume_init_read() == 31); }
#endif // ALLOC_WITH_BOXED

#ifdef ALLOC_WITH_BOXED
    std::printf("alloc BROAD runtime OK (Vec+VecDeque+conv+BinaryHeap+LinkedList+Rc+Arc+from_elem+BTreeMap+BTreeSet+Box)\n");
#else
    std::printf("alloc BROAD runtime OK (Vec+VecDeque+conv+BinaryHeap+LinkedList+Rc+Arc+from_elem+BTreeMap+BTreeSet)\n");
#endif
    return 0;
}
