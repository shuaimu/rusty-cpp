use std::collections::HashMap;
use std::collections::hash_map::Values;
use vstd::prelude::*;

verus! {

broadcast use vstd::std_specs::hash::group_hash_axioms;

pub const SLAB_CAPACITY: usize = 65536;

// A map from u64 ids to values, stored as a std `HashMap`, so memory is
// proportional to the LIVE entries. Ids are allocated by monotone counters and
// never reused (see lion-reactor `alloc_resource_id`); a window indexed by id
// would grow with the ids ever allocated instead. The exec operations are
// std `HashMap` calls specified by vstd (`vstd::std_specs::hash`), whose
// `obeys_key_model::<u64>()` / `builds_valid_hashers::<RandomState>()`
// preconditions are discharged by the `group_hash_axioms` broadcast above.
#[verifier::reject_recursive_types(V)]
pub struct Slab<V: View> {
  pub inner: HashMap<u64, V>,
}

impl<V: View> View for Slab<V> {
  type V = Map<nat, V::V>;

  open spec fn view(&self) -> Map<nat, V::V> {
    Map::new(
      |k: nat| self.spec_contains(k),
      |k: nat| self.inner@[k as u64]@,
    )
  }
}

impl<V: View> Slab<V> {
  pub open spec fn spec_contains(&self, k: nat) -> bool {
    &&& k <= u64::MAX as nat
    &&& self.inner@.contains_key(k as u64)
  }

  pub open spec fn wf(&self) -> bool {
    true
  }

  pub fn new() -> (result: Self)
    ensures
      result@ == Map::<nat, V::V>::empty(),
      result.wf(),
  {
    let result = Slab { inner: HashMap::new() };
    proof {
      assert(result@ =~= Map::<nat, V::V>::empty());
    }
    result
  }

  #[inline(always)]
  pub fn insert(&mut self, key: u64, value: V)
    requires
      old(self).wf(),
    ensures
      self@ == old(self)@.insert(key as nat, value@),
      self.wf(),
  {
    let ghost old_view = old(self)@;
    let ghost v = value;
    let _ = self.inner.insert(key, value);
    proof {
      let kn = key as nat;
      assert(self.inner@ == old(self).inner@.insert(key, v));
      assert forall |k: nat| #[trigger] self.spec_contains(k) <==>
        old_view.insert(kn, v@).contains_key(k)
      by {
        if k <= u64::MAX as nat {
          assert((k as u64) as nat == k);
        }
      };
      assert forall |k: nat| #[trigger] self.spec_contains(k) implies
        self@[k] == old_view.insert(kn, v@)[k]
      by {
        assert((k as u64) as nat == k);
        if k != kn {
          assert(k as u64 != key);
        }
      };
      assert(self@ =~= old_view.insert(kn, v@));
    }
  }

  #[inline(always)]
  pub fn get(&self, key: u64) -> (result: Option<&V>)
    ensures
      result.is_some() <==> self@.contains_key(key as nat),
      result.is_some() ==> result.unwrap()@ == self@[key as nat],
  {
    self.inner.get(&key)
  }

  // external (not external_body): Verus cannot express `&mut` returns, so this
  // stays outside verification — but the body is safe code mirroring `get`
  // above (a std `HashMap::get_mut`; no unsafe anywhere in this crate).
  #[inline(always)]
  #[verifier::external]
  pub fn get_mut(&mut self, key: u64) -> Option<&mut V> {
    self.inner.get_mut(&key)
  }

  // external: iteration over the live values, for unverified callers only
  // (lion-reactor `ResourceSlab::is_empty_timers`). A std `HashMap::values`;
  // no verified code calls it, so no proof depends on its behavior.
  #[inline(always)]
  #[verifier::external]
  pub fn values(&self) -> Values<'_, u64, V> {
    self.inner.values()
  }

  #[inline(always)]
  pub fn remove(&mut self, key: u64) -> (result: Option<V>)
    requires old(self).wf(),
    ensures
      self@ == old(self)@.remove(key as nat),
      result.is_some() <==> old(self)@.contains_key(key as nat),
      result.is_some() ==> result.unwrap()@ == old(self)@[key as nat],
      self.wf(),
  {
    let result = self.inner.remove(&key);
    proof {
      let kn = key as nat;
      assert(self.inner@ == old(self).inner@.remove(key));
      assert forall |k: nat| #[trigger] self.spec_contains(k) <==>
        old(self)@.remove(kn).contains_key(k)
      by {
        if k <= u64::MAX as nat {
          assert((k as u64) as nat == k);
        }
      };
      assert forall |k: nat| #[trigger] self.spec_contains(k) implies
        self@[k] == old(self)@.remove(kn)[k]
      by {
        assert((k as u64) as nat == k);
      };
      assert(self@ =~= old(self)@.remove(kn));
    }
    result
  }

}

}
