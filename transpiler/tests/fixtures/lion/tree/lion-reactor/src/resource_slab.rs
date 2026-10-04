use crate::resource_slot::ResourceSlot;
use crate::resource_slot_wrapper::ResourceSlotWrapper;
use crate::spec::types::{InstantView, ResourceIdView, ResourceSlotView, WakerView};
use crate::types::{Instant, ResourceId, TimerEntry, Waker};
use vstd::prelude::*;
use vstd::set::Set;

verus! {

pub closed spec fn derive_timer_map(m: Map<nat, ResourceSlotView>)
  -> Map<ResourceIdView, (InstantView, ResourceIdView, int)>
{
  Map::new(
    |k: nat| m.contains_key(k) && m[k].is_timer(),
    |k: nat| m[k].get_timer_entry(),
  )
}

pub closed spec fn derive_timer_set(m: Map<nat, ResourceSlotView>)
  -> Set<(InstantView, ResourceIdView, int)>
{
  Set::new(|e: (InstantView, ResourceIdView, int)|
    exists |k: nat| m.contains_key(k) && m[k].is_timer() && m[k].get_timer_entry() == e
  )
}

pub closed spec fn derive_timer_wakers(m: Map<nat, ResourceSlotView>)
  -> Map<ResourceIdView, WakerView>
{
  Map::new(
    |k: nat| m.contains_key(k) && m[k].is_timer(),
    |k: nat| m[k].get_timer_waker(),
  )
}

pub closed spec fn derive_read_wakers(m: Map<nat, ResourceSlotView>)
  -> Map<ResourceIdView, WakerView>
{
  Map::new(
    |k: nat| m.contains_key(k) && m[k].is_io() && m[k].get_read_waker().is_some(),
    |k: nat| m[k].get_read_waker().unwrap(),
  )
}

pub closed spec fn derive_write_wakers(m: Map<nat, ResourceSlotView>)
  -> Map<ResourceIdView, WakerView>
{
  Map::new(
    |k: nat| m.contains_key(k) && m[k].is_io() && m[k].get_write_waker().is_some(),
    |k: nat| m[k].get_write_waker().unwrap(),
  )
}

pub struct ResourceSlab {
  pub inner: lion_slab::Slab<ResourceSlotWrapper>,
}

impl View for ResourceSlab {
  type V = Map<nat, ResourceSlotView>;

  open spec fn view(&self) -> Map<nat, ResourceSlotView> {
    self.inner@
  }
}

impl ResourceSlab {
  pub open spec fn slab_wf(&self) -> bool {
    self.inner.wf()
  }

  pub proof fn timer_map_key_is_timer(&self, k: nat)
    requires self.timer_map_view().contains_key(k),
    ensures self@.contains_key(k), self@[k].is_timer(),
  {
    reveal(ResourceSlab::timer_map_view);
    reveal(derive_timer_map);
  }

  pub closed spec fn timer_set_view(&self) -> Set<(InstantView, ResourceIdView, int)> {
    derive_timer_set(self@)
  }

  pub closed spec fn timer_map_view(&self) -> Map<ResourceIdView, (InstantView, ResourceIdView, int)> {
    derive_timer_map(self@)
  }

  pub closed spec fn timer_wakers_view(&self) -> Map<ResourceIdView, WakerView> {
    derive_timer_wakers(self@)
  }

  pub closed spec fn read_wakers_view(&self) -> Map<ResourceIdView, WakerView> {
    derive_read_wakers(self@)
  }

  pub closed spec fn write_wakers_view(&self) -> Map<ResourceIdView, WakerView> {
    derive_write_wakers(self@)
  }

  pub fn new() -> (result: Self)
    ensures
      result.slab_wf(),
      result@ == Map::<nat, ResourceSlotView>::empty(),
      result.timer_set_view().finite(),
      result.timer_set_view() == Set::<(InstantView, ResourceIdView, int)>::empty(),
      result.timer_map_view() == Map::<ResourceIdView, (InstantView, ResourceIdView, int)>::empty(),
      result.timer_wakers_view() == Map::<ResourceIdView, WakerView>::empty(),
      result.read_wakers_view() == Map::<ResourceIdView, WakerView>::empty(),
      result.write_wakers_view() == Map::<ResourceIdView, WakerView>::empty(),
  {
    let inner = lion_slab::Slab::new();
    let result = ResourceSlab { inner };
    proof {
      reveal(ResourceSlab::timer_set_view);
      reveal(ResourceSlab::timer_map_view);
      reveal(ResourceSlab::timer_wakers_view);
      reveal(ResourceSlab::read_wakers_view);
      reveal(ResourceSlab::write_wakers_view);
      reveal(derive_timer_set);
      reveal(derive_timer_map);
      reveal(derive_timer_wakers);
      reveal(derive_read_wakers);
      reveal(derive_write_wakers);
      assert(result@ =~= Map::<nat, ResourceSlotView>::empty());
      assert(derive_timer_map(result@) =~= Map::<ResourceIdView, (InstantView, ResourceIdView, int)>::empty());
      assert(derive_timer_wakers(result@) =~= Map::<ResourceIdView, WakerView>::empty());
      assert(derive_read_wakers(result@) =~= Map::<ResourceIdView, WakerView>::empty());
      assert(derive_write_wakers(result@) =~= Map::<ResourceIdView, WakerView>::empty());
      assert(derive_timer_set(result@) =~= Set::<(InstantView, ResourceIdView, int)>::empty());
    }
    result
  }

  #[inline]
  pub fn p_insert_timer(&mut self, key: u64, entry: TimerEntry, waker: Waker)
    requires old(self).slab_wf(),
    ensures
      self.slab_wf(),
      self@ == old(self)@.insert(key as nat, ResourceSlotView::Timer {
        entry: entry@,
        waker: waker@,
      }),
  {
    let wrapper = ResourceSlotWrapper::new_timer(entry, waker);
    self.inner.insert(key, wrapper);
  }

  pub fn p_insert_io(&mut self, key: u64)
    requires old(self).slab_wf(),
    ensures
      self.slab_wf(),
      self@ == old(self)@.insert(key as nat, ResourceSlotView::Io {
        read_waker: None,
        write_waker: None,
      }),
  {
    let wrapper = ResourceSlotWrapper::new_io();
    self.inner.insert(key, wrapper);
  }

  #[inline]
  pub fn p_remove(&mut self, key: u64) -> (was_some: bool)
    requires old(self).slab_wf(),
    ensures
      self.slab_wf(),
      self@ == old(self)@.remove(key as nat),
      was_some <==> old(self)@.contains_key(key as nat),
  {
    let result = self.inner.remove(key);
    result.is_some()
  }

  pub fn p_set_read_waker(&mut self, key: u64, waker: Waker)
    requires
      old(self).slab_wf(),
      old(self)@.contains_key(key as nat),
      old(self)@[key as nat].is_io(),
    ensures
      self.slab_wf(),
      self@ == old(self)@.insert(key as nat, ResourceSlotView::Io {
        read_waker: Some(waker@),
        write_waker: old(self)@[key as nat].get_write_waker(),
      }),
  {
    let ghost old_view = self@;
    let ghost old_slot = self@[key as nat];
    let old_wrapper = self.inner.remove(key).unwrap();
    let new_wrapper = old_wrapper.with_read_waker(waker);
    self.inner.insert(key, new_wrapper);
    proof {
      assert(self@ =~= old_view.insert(key as nat, new_wrapper@));
    }
  }

  pub fn p_set_write_waker(&mut self, key: u64, waker: Waker)
    requires
      old(self).slab_wf(),
      old(self)@.contains_key(key as nat),
      old(self)@[key as nat].is_io(),
    ensures
      self.slab_wf(),
      self@ == old(self)@.insert(key as nat, ResourceSlotView::Io {
        read_waker: old(self)@[key as nat].get_read_waker(),
        write_waker: Some(waker@),
      }),
  {
    let ghost old_view = self@;
    let ghost old_slot = self@[key as nat];
    let old_wrapper = self.inner.remove(key).unwrap();
    let new_wrapper = old_wrapper.with_write_waker(waker);
    self.inner.insert(key, new_wrapper);
    proof {
      assert(self@ =~= old_view.insert(key as nat, new_wrapper@));
    }
  }

  pub fn p_get_timer_waker(&self, key: u64) -> (result: Option<Waker>)
    ensures
      (self@.contains_key(key as nat) && self@[key as nat].is_timer()) ==> {
        result.is_some() &&
        result.unwrap()@ == self@[key as nat].get_timer_waker()
      },
      !(self@.contains_key(key as nat) && self@[key as nat].is_timer()) ==>
        result.is_none(),
  {
    let wrapper_opt = self.inner.get(key);
    match wrapper_opt {
      Some(wrapper) => {
        if wrapper.is_timer() {
          Some(wrapper.clone_timer_waker())
        } else {
          None
        }
      }
      None => None,
    }
  }

  pub fn p_get_read_waker(&self, key: u64) -> (result: Option<Waker>)
    ensures
      (self@.contains_key(key as nat) && self@[key as nat].is_io()
        && self@[key as nat].get_read_waker().is_some()) ==> {
        result.is_some() &&
        result.unwrap()@ == self@[key as nat].get_read_waker().unwrap()
      },
      !(self@.contains_key(key as nat) && self@[key as nat].is_io()
        && self@[key as nat].get_read_waker().is_some()) ==>
        result.is_none(),
  {
    let wrapper_opt = self.inner.get(key);
    match wrapper_opt {
      Some(wrapper) => {
        if wrapper.is_io() {
          wrapper.clone_read_waker()
        } else {
          None
        }
      }
      None => None,
    }
  }

  pub fn p_get_write_waker(&self, key: u64) -> (result: Option<Waker>)
    ensures
      (self@.contains_key(key as nat) && self@[key as nat].is_io()
        && self@[key as nat].get_write_waker().is_some()) ==> {
        result.is_some() &&
        result.unwrap()@ == self@[key as nat].get_write_waker().unwrap()
      },
      !(self@.contains_key(key as nat) && self@[key as nat].is_io()
        && self@[key as nat].get_write_waker().is_some()) ==>
        result.is_none(),
  {
    let wrapper_opt = self.inner.get(key);
    match wrapper_opt {
      Some(wrapper) => {
        if wrapper.is_io() {
          wrapper.clone_write_waker()
        } else {
          None
        }
      }
      None => None,
    }
  }

  pub fn v_set_read_waker(&mut self, key: u64, waker: Waker)
    requires
      old(self).slab_wf(),
      old(self)@.contains_key(key as nat),
      old(self)@[key as nat].is_io(),
    ensures
      self.slab_wf(),
      self@ == old(self)@.insert(key as nat,
        ResourceSlotView::Io { read_waker: Some(waker@), write_waker: old(self)@[key as nat].get_write_waker() }),
      self.read_wakers_view() == old(self).read_wakers_view().insert(key as nat, waker@),
      self.write_wakers_view() == old(self).write_wakers_view(),
      self.timer_set_view() == old(self).timer_set_view(),
      self.timer_map_view() == old(self).timer_map_view(),
      self.timer_wakers_view() == old(self).timer_wakers_view(),
  {
    let ghost old_m = self@;
    let ghost k = key as nat;
    self.p_set_read_waker(key, waker);
    proof {
      reveal(ResourceSlab::timer_set_view);
      reveal(ResourceSlab::timer_map_view);
      reveal(ResourceSlab::timer_wakers_view);
      reveal(ResourceSlab::read_wakers_view);
      reveal(ResourceSlab::write_wakers_view);
      reveal(derive_timer_set);
      reveal(derive_timer_map);
      reveal(derive_timer_wakers);
      reveal(derive_read_wakers);
      reveal(derive_write_wakers);

      let new_m = self@;
      let new_slot = new_m[k];

      assert(derive_timer_map(new_m) =~= derive_timer_map(old_m));
      assert(derive_timer_wakers(new_m) =~= derive_timer_wakers(old_m));
      assert(derive_write_wakers(new_m) =~= derive_write_wakers(old_m));
      assert(derive_read_wakers(new_m) =~= derive_read_wakers(old_m).insert(k, waker@));
      assert(derive_timer_set(new_m) =~= derive_timer_set(old_m)) by {
        assert forall |e: (InstantView, ResourceIdView, int)|
          derive_timer_set(new_m).contains(e) <==> derive_timer_set(old_m).contains(e)
        by {
          if derive_timer_set(new_m).contains(e) {
            let witness = choose |w: nat| new_m.contains_key(w) && new_m[w].is_timer() && new_m[w].get_timer_entry() == e;
            assert(witness != k);
            assert(old_m.contains_key(witness));
            assert(old_m[witness] == new_m[witness]);
          }
          if derive_timer_set(old_m).contains(e) {
            let witness = choose |w: nat| old_m.contains_key(w) && old_m[w].is_timer() && old_m[w].get_timer_entry() == e;
            assert(witness != k);
            assert(new_m.contains_key(witness));
            assert(new_m[witness] == old_m[witness]);
          }
        };
      };
    }
  }

  pub fn v_set_write_waker(&mut self, key: u64, waker: Waker)
    requires
      old(self).slab_wf(),
      old(self)@.contains_key(key as nat),
      old(self)@[key as nat].is_io(),
    ensures
      self.slab_wf(),
      self@ == old(self)@.insert(key as nat,
        ResourceSlotView::Io { read_waker: old(self)@[key as nat].get_read_waker(), write_waker: Some(waker@) }),
      self.write_wakers_view() == old(self).write_wakers_view().insert(key as nat, waker@),
      self.read_wakers_view() == old(self).read_wakers_view(),
      self.timer_set_view() == old(self).timer_set_view(),
      self.timer_map_view() == old(self).timer_map_view(),
      self.timer_wakers_view() == old(self).timer_wakers_view(),
  {
    let ghost old_m = self@;
    let ghost k = key as nat;
    self.p_set_write_waker(key, waker);
    proof {
      reveal(ResourceSlab::timer_set_view);
      reveal(ResourceSlab::timer_map_view);
      reveal(ResourceSlab::timer_wakers_view);
      reveal(ResourceSlab::read_wakers_view);
      reveal(ResourceSlab::write_wakers_view);
      reveal(derive_timer_set);
      reveal(derive_timer_map);
      reveal(derive_timer_wakers);
      reveal(derive_read_wakers);
      reveal(derive_write_wakers);

      let new_m = self@;

      assert(derive_timer_map(new_m) =~= derive_timer_map(old_m));
      assert(derive_timer_wakers(new_m) =~= derive_timer_wakers(old_m));
      assert(derive_read_wakers(new_m) =~= derive_read_wakers(old_m));
      assert(derive_write_wakers(new_m) =~= derive_write_wakers(old_m).insert(k, waker@));
      assert(derive_timer_set(new_m) =~= derive_timer_set(old_m)) by {
        assert forall |e: (InstantView, ResourceIdView, int)|
          derive_timer_set(new_m).contains(e) <==> derive_timer_set(old_m).contains(e)
        by {
          if derive_timer_set(new_m).contains(e) {
            let witness = choose |w: nat| new_m.contains_key(w) && new_m[w].is_timer() && new_m[w].get_timer_entry() == e;
            assert(witness != k);
            assert(old_m.contains_key(witness));
          }
          if derive_timer_set(old_m).contains(e) {
            let witness = choose |w: nat| old_m.contains_key(w) && old_m[w].is_timer() && old_m[w].get_timer_entry() == e;
            assert(witness != k);
            assert(new_m.contains_key(witness));
            assert(new_m[witness] == old_m[witness]);
          }
        };
      };
    }
  }

  #[inline]
  pub fn v_insert_timer_slot(
    &mut self,
    key: u64,
    entry: TimerEntry,
    waker: Waker,
  )
    requires
      old(self).slab_wf(),
      !old(self)@.contains_key(key as nat),
      old(self).timer_set_view().finite(),
    ensures
      self.slab_wf(),
      self@ == old(self)@.insert(key as nat, ResourceSlotView::Timer { entry: entry@, waker: waker@ }),
      self.timer_set_view() == old(self).timer_set_view().insert(entry@),
      self.timer_set_view().finite(),
      self.timer_map_view() == old(self).timer_map_view().insert(key as nat, entry@),
      self.timer_wakers_view() == old(self).timer_wakers_view().insert(key as nat, waker@),
      self.read_wakers_view() == old(self).read_wakers_view(),
      self.write_wakers_view() == old(self).write_wakers_view(),
  {
    let ghost old_m = self@;
    self.p_insert_timer(key, entry, waker);
    proof {
      reveal(ResourceSlab::timer_set_view);
      reveal(ResourceSlab::timer_map_view);
      reveal(ResourceSlab::timer_wakers_view);
      reveal(ResourceSlab::read_wakers_view);
      reveal(ResourceSlab::write_wakers_view);
      reveal(derive_timer_set);
      reveal(derive_timer_map);
      reveal(derive_timer_wakers);
      reveal(derive_read_wakers);
      reveal(derive_write_wakers);

      let k = key as nat;
      let new_m = self@;
      let slot = ResourceSlotView::Timer { entry: entry@, waker: waker@ };

      assert(derive_timer_map(new_m) =~= derive_timer_map(old_m).insert(k, entry@));
      assert(derive_timer_wakers(new_m) =~= derive_timer_wakers(old_m).insert(k, waker@));
      assert(derive_read_wakers(new_m) =~= derive_read_wakers(old_m));
      assert(derive_write_wakers(new_m) =~= derive_write_wakers(old_m));

      assert(derive_timer_set(new_m) =~= derive_timer_set(old_m).insert(entry@)) by {
        assert forall |e: (InstantView, ResourceIdView, int)|
          derive_timer_set(new_m).contains(e) <==> derive_timer_set(old_m).insert(entry@).contains(e)
        by {
          if derive_timer_set(new_m).contains(e) {
            let witness = choose |w: nat| new_m.contains_key(w) && new_m[w].is_timer() && new_m[w].get_timer_entry() == e;
            if witness == k {
              assert(e == entry@);
            } else {
              assert(old_m.contains_key(witness));
              assert(old_m[witness] == new_m[witness]);
            }
          }
          if derive_timer_set(old_m).insert(entry@).contains(e) {
            if e == entry@ {
              assert(new_m.contains_key(k));
              assert(new_m[k].is_timer());
              assert(new_m[k].get_timer_entry() == entry@);
            } else {
              assert(derive_timer_set(old_m).contains(e));
              let witness = choose |w: nat| old_m.contains_key(w) && old_m[w].is_timer() && old_m[w].get_timer_entry() == e;
              assert(new_m.contains_key(witness));
              assert(new_m[witness] == old_m[witness]);
            }
          }
        };
      };
    }
  }

  pub fn v_insert_io_slot(&mut self, key: u64)
    requires
      old(self).slab_wf(),
      !old(self)@.contains_key(key as nat),
    ensures
      self.slab_wf(),
      self@ == old(self)@.insert(key as nat, ResourceSlotView::Io { read_waker: None, write_waker: None }),
      self.read_wakers_view() == old(self).read_wakers_view(),
      self.write_wakers_view() == old(self).write_wakers_view(),
      self.timer_set_view() == old(self).timer_set_view(),
      self.timer_map_view() == old(self).timer_map_view(),
      self.timer_wakers_view() == old(self).timer_wakers_view(),
  {
    let ghost old_m = self@;
    self.p_insert_io(key);
    proof {
      reveal(ResourceSlab::timer_set_view);
      reveal(ResourceSlab::timer_map_view);
      reveal(ResourceSlab::timer_wakers_view);
      reveal(ResourceSlab::read_wakers_view);
      reveal(ResourceSlab::write_wakers_view);
      reveal(derive_timer_set);
      reveal(derive_timer_map);
      reveal(derive_timer_wakers);
      reveal(derive_read_wakers);
      reveal(derive_write_wakers);

      let k = key as nat;
      let new_m = self@;

      assert(derive_timer_map(new_m) =~= derive_timer_map(old_m));
      assert(derive_timer_wakers(new_m) =~= derive_timer_wakers(old_m));
      assert(derive_read_wakers(new_m) =~= derive_read_wakers(old_m));
      assert(derive_write_wakers(new_m) =~= derive_write_wakers(old_m));
      assert(derive_timer_set(new_m) =~= derive_timer_set(old_m)) by {
        assert forall |e: (InstantView, ResourceIdView, int)|
          derive_timer_set(new_m).contains(e) <==> derive_timer_set(old_m).contains(e)
        by {
          if derive_timer_set(new_m).contains(e) {
            let witness = choose |w: nat| new_m.contains_key(w) && new_m[w].is_timer() && new_m[w].get_timer_entry() == e;
            if witness == k {
              assert(!new_m[k].is_timer());
              assert(false);
            }
            assert(old_m.contains_key(witness));
          }
          if derive_timer_set(old_m).contains(e) {
            let witness = choose |w: nat| old_m.contains_key(w) && old_m[w].is_timer() && old_m[w].get_timer_entry() == e;
            assert(witness != k);
            assert(new_m.contains_key(witness));
            assert(new_m[witness] == old_m[witness]);
          }
        };
      };
    }
  }

  #[inline]
  pub fn v_remove_timer_slot(&mut self, key: u64)
    requires
      old(self).slab_wf(),
      old(self).timer_set_view().finite(),
      old(self)@.contains_key(key as nat) ==> old(self)@[key as nat].is_timer(),
      forall |r: nat| #![auto] old(self).timer_map_view().contains_key(r) ==> old(self).timer_map_view()[r].1 == r,
    ensures
      self@ == old(self)@.remove(key as nat),
      old(self).timer_map_view().contains_key(key as nat) ==> {
        let entry = old(self).timer_map_view()[key as nat];
        self.timer_set_view() == old(self).timer_set_view().remove(entry) &&
        self.timer_map_view() == old(self).timer_map_view().remove(key as nat) &&
        self.timer_wakers_view() == old(self).timer_wakers_view().remove(key as nat)
      },
      !old(self).timer_map_view().contains_key(key as nat) ==> {
        self.timer_set_view() == old(self).timer_set_view() &&
        self.timer_map_view() == old(self).timer_map_view() &&
        self.timer_wakers_view() == old(self).timer_wakers_view()
      },
      self.slab_wf(),
      self.timer_set_view().finite(),
      self.read_wakers_view() == old(self).read_wakers_view(),
      self.write_wakers_view() == old(self).write_wakers_view(),
  {
    let ghost old_m = self@;
    let ghost k = key as nat;
    let _ = self.p_remove(key);
    proof {
      reveal(ResourceSlab::timer_set_view);
      reveal(ResourceSlab::timer_map_view);
      reveal(ResourceSlab::timer_wakers_view);
      reveal(ResourceSlab::read_wakers_view);
      reveal(ResourceSlab::write_wakers_view);
      reveal(derive_timer_set);
      reveal(derive_timer_map);
      reveal(derive_timer_wakers);
      reveal(derive_read_wakers);
      reveal(derive_write_wakers);

      let new_m = self@;

      assert(derive_read_wakers(new_m) =~= derive_read_wakers(old_m));
      assert(derive_write_wakers(new_m) =~= derive_write_wakers(old_m));

      if derive_timer_map(old_m).contains_key(k) {
        assert(old_m.contains_key(k) && old_m[k].is_timer());
        let old_entry = old_m[k].get_timer_entry();

        assert(derive_timer_map(new_m) =~= derive_timer_map(old_m).remove(k));
        assert(derive_timer_wakers(new_m) =~= derive_timer_wakers(old_m).remove(k));

        assert(derive_timer_set(new_m) =~= derive_timer_set(old_m).remove(old_entry)) by {
          reveal(ResourceSlab::timer_map_view);
          assert(derive_timer_map(old_m) =~= old(self).timer_map_view());
          assert forall |e: (InstantView, ResourceIdView, int)|
            derive_timer_set(new_m).contains(e) <==> derive_timer_set(old_m).remove(old_entry).contains(e)
          by {
            if derive_timer_set(new_m).contains(e) {
              let witness = choose |w: nat| new_m.contains_key(w) && new_m[w].is_timer() && new_m[w].get_timer_entry() == e;
              assert(witness != k);
              assert(old_m.contains_key(witness));
              assert(old_m[witness] == new_m[witness]);
              assert(derive_timer_set(old_m).contains(e));
              assert(derive_timer_map(old_m).contains_key(witness));
              assert(derive_timer_map(old_m)[witness].1 == witness);
              assert(derive_timer_map(old_m).contains_key(k));
              assert(derive_timer_map(old_m)[k].1 == k);
              assert(e.1 == witness);
              assert(old_entry.1 == k);
              assert(e.1 != old_entry.1);
              assert(e != old_entry);
            }
            if derive_timer_set(old_m).remove(old_entry).contains(e) {
              assert(derive_timer_set(old_m).contains(e));
              assert(e != old_entry);
              let witness = choose |w: nat| old_m.contains_key(w) && old_m[w].is_timer() && old_m[w].get_timer_entry() == e;
              assert(witness != k);
              assert(new_m.contains_key(witness));
              assert(new_m[witness] == old_m[witness]);
            }
          };
        };
      } else {
        assert(derive_timer_map(new_m) =~= derive_timer_map(old_m));
        assert(derive_timer_wakers(new_m) =~= derive_timer_wakers(old_m));
        assert(derive_timer_set(new_m) =~= derive_timer_set(old_m)) by {
          assert forall |e: (InstantView, ResourceIdView, int)|
            derive_timer_set(new_m).contains(e) <==> derive_timer_set(old_m).contains(e)
          by {
            if derive_timer_set(new_m).contains(e) {
              let witness = choose |w: nat| new_m.contains_key(w) && new_m[w].is_timer() && new_m[w].get_timer_entry() == e;
              assert(witness != k);
              assert(old_m.contains_key(witness));
            }
            if derive_timer_set(old_m).contains(e) {
              let witness = choose |w: nat| old_m.contains_key(w) && old_m[w].is_timer() && old_m[w].get_timer_entry() == e;
              if witness == k {
                assert(derive_timer_map(old_m).contains_key(k));
              }
              assert(witness != k);
              assert(new_m.contains_key(witness));
              assert(new_m[witness] == old_m[witness]);
            }
          };
        };
      }
    }
  }

  pub fn v_remove_io_slot(&mut self, key: u64)
    requires
      old(self).slab_wf(),
      old(self).timer_set_view().finite(),
      old(self)@.contains_key(key as nat) ==> old(self)@[key as nat].is_io(),
    ensures
      self.slab_wf(),
      self@ == old(self)@.remove(key as nat),
      self.read_wakers_view() == old(self).read_wakers_view().remove(key as nat),
      self.write_wakers_view() == old(self).write_wakers_view().remove(key as nat),
      self.timer_set_view() == old(self).timer_set_view(),
      self.timer_map_view() == old(self).timer_map_view(),
      self.timer_wakers_view() == old(self).timer_wakers_view(),
  {
    let ghost old_m = self@;
    let ghost k = key as nat;
    let _ = self.p_remove(key);
    proof {
      reveal(ResourceSlab::timer_set_view);
      reveal(ResourceSlab::timer_map_view);
      reveal(ResourceSlab::timer_wakers_view);
      reveal(ResourceSlab::read_wakers_view);
      reveal(ResourceSlab::write_wakers_view);
      reveal(derive_timer_set);
      reveal(derive_timer_map);
      reveal(derive_timer_wakers);
      reveal(derive_read_wakers);
      reveal(derive_write_wakers);

      let new_m = self@;

      assert(derive_read_wakers(new_m) =~= derive_read_wakers(old_m).remove(k));
      assert(derive_write_wakers(new_m) =~= derive_write_wakers(old_m).remove(k));
      assert(derive_timer_map(new_m) =~= derive_timer_map(old_m));
      assert(derive_timer_wakers(new_m) =~= derive_timer_wakers(old_m));
      assert(derive_timer_set(new_m) =~= derive_timer_set(old_m)) by {
        assert forall |e: (InstantView, ResourceIdView, int)|
          derive_timer_set(new_m).contains(e) <==> derive_timer_set(old_m).contains(e)
        by {
          if derive_timer_set(new_m).contains(e) {
            let witness = choose |w: nat| new_m.contains_key(w) && new_m[w].is_timer() && new_m[w].get_timer_entry() == e;
            assert(witness != k);
            assert(old_m.contains_key(witness));
          }
          if derive_timer_set(old_m).contains(e) {
            let witness = choose |w: nat| old_m.contains_key(w) && old_m[w].is_timer() && old_m[w].get_timer_entry() == e;
            assert(witness != k);
            assert(new_m.contains_key(witness));
            assert(new_m[witness] == old_m[witness]);
          }
        };
      };
    }
  }

  #[inline]
  pub fn v_take_timer_waker(&self, key: u64) -> (result: Option<Waker>)
    ensures
      self.timer_wakers_view().contains_key(key as nat) ==> {
        result.is_some() &&
        result.unwrap()@ == self.timer_wakers_view()[key as nat]
      },
      !self.timer_wakers_view().contains_key(key as nat) ==> result.is_none(),
  {
    let result = self.p_get_timer_waker(key);
    proof {
      reveal(ResourceSlab::timer_wakers_view);
      reveal(derive_timer_wakers);
    }
    result
  }

  pub fn v_get_read_waker(&self, key: u64) -> (result: Option<Waker>)
    ensures
      self.read_wakers_view().contains_key(key as nat) ==> {
        result.is_some() &&
        result.unwrap()@ == self.read_wakers_view()[key as nat]
      },
      !self.read_wakers_view().contains_key(key as nat) ==> result.is_none(),
  {
    let result = self.p_get_read_waker(key);
    proof {
      reveal(ResourceSlab::read_wakers_view);
      reveal(derive_read_wakers);
    }
    result
  }

  pub fn v_get_write_waker(&self, key: u64) -> (result: Option<Waker>)
    ensures
      self.write_wakers_view().contains_key(key as nat) ==> {
        result.is_some() &&
        result.unwrap()@ == self.write_wakers_view()[key as nat]
      },
      !self.write_wakers_view().contains_key(key as nat) ==> result.is_none(),
  {
    let result = self.p_get_write_waker(key);
    proof {
      reveal(ResourceSlab::write_wakers_view);
      reveal(derive_write_wakers);
    }
    result
  }
}

}

impl ResourceSlab {
  #[inline]
  fn get_slot(&self, key: u64) -> Option<&ResourceSlot> {
    self.inner.get(key).map(|w| &w.inner)
  }

  #[inline]
  fn get_slot_mut(&mut self, key: u64) -> Option<&mut ResourceSlot> {
    self.inner.get_mut(key).map(|w| &mut w.inner)
  }

  #[inline]
  pub fn contains(&self, key: u64) -> bool {
    self.inner.get(key).is_some()
  }

  pub fn get_timer_entry(&self, key: u64) -> Option<TimerEntry> {
    match self.get_slot(key) {
      Some(ResourceSlot::Timer { entry, .. }) => Some(*entry),
      _ => None,
    }
  }

  pub fn take_timer_waker(&mut self, key: u64) -> Option<Waker> {
    match self.get_slot_mut(key) {
      Some(ResourceSlot::Timer { waker, .. }) => {
        let w = waker.clone();
        Some(w)
      }
      _ => None,
    }
  }

  pub fn get_read_waker(&self, key: u64) -> Option<&Waker> {
    match self.get_slot(key) {
      Some(ResourceSlot::Io { read_waker, .. }) => read_waker.as_ref(),
      _ => None,
    }
  }

  pub fn get_write_waker(&self, key: u64) -> Option<&Waker> {
    match self.get_slot(key) {
      Some(ResourceSlot::Io { write_waker, .. }) => write_waker.as_ref(),
      _ => None,
    }
  }

  pub fn set_read_waker(&mut self, key: u64, waker: Waker) {
    if let Some(ResourceSlot::Io { read_waker, .. }) = self.get_slot_mut(key) {
      *read_waker = Some(waker);
    }
  }

  pub fn set_write_waker(&mut self, key: u64, waker: Waker) {
    if let Some(ResourceSlot::Io { write_waker, .. }) = self.get_slot_mut(key) {
      *write_waker = Some(waker);
    }
  }

  #[inline]
  pub fn replace_timer_waker(&mut self, key: u64, new_waker: Waker) -> bool {
    match self.inner.get_mut(key) {
      Some(ResourceSlotWrapper { inner: ResourceSlot::Timer { waker, .. } }) => {
        *waker = new_waker;
        true
      }
      _ => false,
    }
  }

  pub fn is_empty_timers(&self) -> bool {
    !self.inner.values().any(|w| matches!(w.inner, ResourceSlot::Timer { .. }))
  }
}
