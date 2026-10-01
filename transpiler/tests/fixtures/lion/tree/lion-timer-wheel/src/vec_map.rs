use std::collections::HashMap;
use vstd::prelude::*;

verus! {

broadcast use vstd::std_specs::hash::group_hash_axioms;

// A map from u64 timer ids to values, stored as a std `HashMap`, so memory is
// proportional to the LIVE timers. Timer ids are resource ids, which are
// allocated by a monotone counter and never reused; a window indexed by id
// would grow with the ids ever allocated instead. The exec operations are std
// `HashMap` calls specified by vstd (`vstd::std_specs::hash`), whose
// `obeys_key_model::<u64>()` / `builds_valid_hashers::<RandomState>()`
// preconditions are discharged by the `group_hash_axioms` broadcast above.
// (The name is historical: this used to be a vector window.)
#[verifier::reject_recursive_types(V)]
pub struct VecMap<V: View> {
  pub inner: HashMap<u64, V>,
}

impl<V: View> VecMap<V> {
  // Kept under its historical name because the wheel's `wf` and the
  // `is_empty` precondition mention it. The exec entry count this predicate
  // used to couple to the spec occupancy now lives inside the std `HashMap`,
  // whose `is_empty` is specified directly against the view, so there is no
  // separate counter left to keep in sync and nothing for this to constrain.
  pub open spec fn count_wf(&self) -> bool {
    true
  }
}

impl<V: View> View for VecMap<V> {
  type V = Map<u64, V>;

  open spec fn view(&self) -> Map<u64, V> {
    self.inner@
  }
}

impl<V: View + Copy> VecMap<V> {

  pub exec fn new() -> (result: Self)
    ensures
      result@ == Map::<u64, V>::empty(),
      result.count_wf(),
  {
    VecMap { inner: HashMap::new() }
  }

  #[inline(always)]
  pub exec fn insert(&mut self, key: u64, value: V)
    ensures
      self@ == old(self)@.insert(key, value),
      old(self).count_wf() ==> self.count_wf(),
  {
    let _ = self.inner.insert(key, value);
  }

  #[inline(always)]
  pub exec fn get(&self, key: &u64) -> (result: Option<&V>)
    ensures
      result.is_some() <==> self@.contains_key(*key),
      result.is_some() ==> *result.unwrap() == self@[*key],
  {
    self.inner.get(key)
  }

  #[inline(always)]
  pub exec fn remove(&mut self, key: &u64) -> (result: Option<V>)
    ensures
      self@ == old(self)@.remove(*key),
      result.is_some() <==> old(self)@.contains_key(*key),
      result.is_some() ==> result.unwrap() == old(self)@[*key],
      old(self).count_wf() ==> self.count_wf(),
  {
    self.inner.remove(key)
  }

  #[inline(always)]
  pub exec fn contains_key(&self, key: &u64) -> (result: bool)
    ensures result == self@.contains_key(*key),
  {
    self.inner.contains_key(key)
  }

  // VERIFIED O(1) emptiness: std `HashMap::is_empty` is specified as
  // `res == m@.is_empty()` (an empty domain), which is equivalent to the view
  // being the empty map.
  #[inline]
  pub exec fn is_empty(&self) -> (result: bool)
    requires self.count_wf(),
    ensures result == (self@ == Map::<u64, V>::empty()),
  {
    let r = self.inner.is_empty();
    proof {
      if r {
        assert(self@.dom() =~= Set::<u64>::empty());
        assert(self@ =~= Map::<u64, V>::empty());
      } else {
        assert(self@.dom() != Set::<u64>::empty());
        assert(Map::<u64, V>::empty().dom() =~= Set::<u64>::empty());
      }
    }
    r
  }
}

}
