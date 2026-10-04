use crate::reactor::Reactor;
use crate::types::{Instant, IoResult, ResourceId, TimerEntry, Waker};
use crate::spec::types::{InstantView, ResourceIdView, WakerView};
use crate::invariants::*;
use crate::invariants::data_inv::*;
use crate::spec::log::*;
use crate::spec::predicates::*;
use crate::proof::preservation::*;
use crate::proof::safety_preservation::*;
use crate::proof::preservation_ext::*;
use crate::proof::park_safety::*;
use crate::resource_slab::ResourceSlab;
use vstd::prelude::*;

verus! {

impl Reactor {
  #[inline]
  pub fn register_timer(&mut self, deadline: Instant, waker: Waker)
    -> (result: IoResult<ResourceId>)
    requires
      old(self).wheel.full_wf(),
      old(self).resources.slab_wf(),
      reactor_wf(old(self).log@, old(self).next_resource_id as nat),
      timer_impl_inv(old(self).resources.timer_set_view(), old(self).resources.timer_map_view(), old(self).next_resource_id as nat),
      slab_alloc_inv(old(self).resources@, old(self).next_resource_id as nat),
      free_rids_wf(old(self).free_rids@, old(self).log@, old(self).resources@, old(self).next_resource_id as nat),
      data_inv(
        old(self).resources.timer_set_view(), old(self).resources.timer_map_view(),
        old(self).resources.timer_wakers_view(), old(self).resources.read_wakers_view(), old(self).resources.write_wakers_view(),
        old(self).log@,
      ),
      deadline@ > max_timestamp_up_to(old(self).log@, old(self).log@.len() as int),
      wheel_slab_consistent(old(self).wheel@, old(self).resources.timer_map_view()),
      old(self).wheel.pending@.len() == 0,
      deadline.inner > old(self).wheel.elapsed,
    ensures
      self.wheel.full_wf(),
      self.wheel.pending@.len() == 0,
      self.wheel.elapsed == old(self).wheel.elapsed,
      self.resources.slab_wf(),
      reactor_wf(self.log@, self.next_resource_id as nat),
      timer_impl_inv(self.resources.timer_set_view(), self.resources.timer_map_view(), self.next_resource_id as nat),
      slab_alloc_inv(self.resources@, self.next_resource_id as nat),
      free_rids_wf(self.free_rids@, self.log@, self.resources@, self.next_resource_id as nat),
      data_inv(
        self.resources.timer_set_view(), self.resources.timer_map_view(),
        self.resources.timer_wakers_view(), self.resources.read_wakers_view(), self.resources.write_wakers_view(),
        self.log@,
      ),
      wheel_slab_consistent(self.wheel@, self.resources.timer_map_view()),
  {
    let ghost old_log = self.log@;
    let ghost old_ts = self.resources.timer_set_view();
    let ghost old_tm = self.resources.timer_map_view();
    let ghost old_tw = self.resources.timer_wakers_view();
    let ghost old_rw = self.resources.read_wakers_view();
    let ghost old_ww = self.resources.write_wakers_view();
    let ghost old_next_rid = self.next_resource_id as nat;

    self.register_timer_begin_action(deadline, &waker);

    let ghost log_mid = self.log@;

    proof {
      let begin_event = log_mid[old_log.len() as int];
      assert(log_mid == old_log.push(begin_event));
      data_inv_preserved_by_harmless_event(old_ts, old_tm, old_tw, old_rw, old_ww, old_log, begin_event);
      reactor_inv_preserved_by_non_trigger(old_log, begin_event);
      reactor_ext_inv_preserved_by_non_trigger(old_log, begin_event);
      alloc_inv_preserved_by_non_registration(old_log, begin_event, old_next_rid);
      free_rids_wf_preserved_by_append(old(self).free_rids@, old_log, begin_event, self.resources@, old_next_rid);
    }

    let alloc_result = self.alloc_resource_id();

    let ghost new_next_rid = self.next_resource_id as nat;

    let result = match alloc_result {
      IoResult::Ok(resource_id) => {
        let ghost rid = resource_id@;
        let log_index: Ghost<int> = Ghost(self.log@.len() as int);
        let entry = TimerEntry { deadline, resource_id, log_index };

        proof {
          assert(!self.resources@.contains_key(rid));
          if old_tm.contains_key(rid) {
            self.resources.timer_map_key_is_timer(rid);
            assert(false);
          }
          assert(!old_tm.contains_key(rid));
        }

        self.resources.v_insert_timer_slot(resource_id.0, entry, waker.clone());
        self.wheel.insert(resource_id.0, deadline.inner);

        proof {
          free_rids_wf_preserved_by_resource_insert(self.free_rids@, log_mid, old(self).resources@, new_next_rid, rid, self.resources@[rid]);
        }

        IoResult::Ok(resource_id)
      }
      IoResult::Err(e) => IoResult::Err(e),
    };

    let ghost pre_end_ts = self.resources.timer_set_view();
    let ghost pre_end_tm = self.resources.timer_map_view();
    let ghost pre_end_tw = self.resources.timer_wakers_view();
    let ghost pre_end_free_rids = self.free_rids@;
    let ghost pre_end_resources = self.resources@;

    self.register_timer_end_action(deadline, &waker, &result);

    let ghost log_final = self.log@;

    proof {
      let end_event = log_final[log_mid.len() as int];
      assert(log_final == log_mid.push(end_event));

      match result {
        IoResult::Ok(resource_id) => {
          let rid = resource_id@;
          let deadline_val = deadline@;
          let waker_val = waker@;
          let log_idx = log_mid.len() as int;
          let new_entry = (deadline_val, rid, log_idx);

          assert(!old_tm.contains_key(rid));

          reactor_inv_preserved_by_succ_register_timer(log_mid, end_event);

          max_timestamp_up_to_push(old_log, log_mid[old_log.len() as int], old_log.len() as int);
          reactor_ext_inv_preserved_by_succ_register_timer(log_mid, end_event);

          alloc_inv_preserved_by_registration(log_mid, end_event, new_next_rid);

          free_rids_wf_preserved_by_register_timer(pre_end_free_rids, log_mid, end_event, pre_end_resources, new_next_rid, rid);

          data_inv_preserved_by_fresh_register_timer(
            old_ts, old_tm, old_tw, old_rw, old_ww,
            log_mid, end_event,
            rid, deadline_val, waker_val,
          );

          assert(pre_end_ts == old_ts.insert(new_entry));
          assert(pre_end_tm == old_tm.insert(rid, new_entry));
          assert(pre_end_tw == old_tw.insert(rid, waker_val));

          assert(pre_end_ts.finite()) by {
            assert(old_ts.finite());
          }

          assert forall |r: nat| pre_end_tm.contains_key(r)
            implies pre_end_ts.contains(pre_end_tm[r]) && pre_end_tm[r].1 == r && 1 <= r && r < new_next_rid
          by {
            if r == rid {
              assert(pre_end_tm[r] == new_entry);
              assert(pre_end_ts.contains(new_entry));
              assert(new_entry.1 == rid);
              assert(1 <= rid && rid < new_next_rid);
            } else {
              assert(old_tm.contains_key(r));
              assert(pre_end_tm[r] == old_tm[r]);
              assert(old_ts.contains(old_tm[r]));
              assert(pre_end_ts.contains(old_tm[r]));
              assert(old_tm[r].1 == r);
              assert(1 <= r && r < old_next_rid);
              assert(r < new_next_rid);
            }
          }

          assert forall |d: InstantView, r: ResourceIdView, i: int|
            pre_end_ts.contains((d, r, i)) implies
            pre_end_tm.contains_key(r) && pre_end_tm[r] == (d, r, i)
          by {
            if (d, r, i) == new_entry {
              assert(r == rid);
              assert(pre_end_tm.contains_key(rid));
              assert(pre_end_tm[rid] == new_entry);
            } else {
              assert(old_ts.contains((d, r, i)));
              assert(old_tm.contains_key(r) && old_tm[r] == (d, r, i));
              assert(r != rid);
              assert(pre_end_tm.contains_key(r));
              assert(pre_end_tm[r] == old_tm[r]);
            }
          }

          assert(wheel_slab_consistent(self.wheel@, pre_end_tm)) by {
            let new_wheel = self.wheel@;
            assert(new_wheel =~= old(self).wheel@.insert(rid, deadline@ as int));
            assert(pre_end_tm =~= old_tm.insert(rid, new_entry));
            assert forall |r: nat| #![auto] new_wheel.contains_key(r) <==> pre_end_tm.contains_key(r) by {
              if r == rid {
              } else {
                assert(new_wheel.contains_key(r) <==> old(self).wheel@.contains_key(r));
                assert(pre_end_tm.contains_key(r) <==> old_tm.contains_key(r));
              }
            };
            assert forall |r: nat| #![auto] new_wheel.contains_key(r) implies new_wheel[r] == pre_end_tm[r].0 by {
              if r == rid {
                assert(new_wheel[r] == deadline@ as int);
                assert(pre_end_tm[r] == new_entry);
                assert(new_entry.0 == deadline_val);
              } else {
                assert(new_wheel[r] == old(self).wheel@[r]);
                assert(pre_end_tm[r] == old_tm[r]);
                assert(old(self).wheel@[r] == old_tm[r].0);
              }
            };
          };
        }
        IoResult::Err(_) => {
          data_inv_preserved_by_harmless_event(old_ts, old_tm, old_tw, old_rw, old_ww, log_mid, end_event);
          reactor_inv_preserved_by_non_trigger(log_mid, end_event);
          reactor_ext_inv_preserved_by_non_trigger(log_mid, end_event);
          alloc_inv_preserved_by_non_registration(log_mid, end_event, new_next_rid);
          free_rids_wf_preserved_by_append(pre_end_free_rids, log_mid, end_event, pre_end_resources, new_next_rid);

          assert(pre_end_ts == old_ts);
          assert(pre_end_tm == old_tm);
          assert(pre_end_tw == old_tw);
        }
      }
    }
    result
  }

  #[inline]
  pub fn deregister_timer(&mut self, resource_id: ResourceId)
    requires
      old(self).wheel.full_wf(),
      old(self).resources.slab_wf(),
      reactor_wf(old(self).log@, old(self).next_resource_id as nat),
      timer_impl_inv(old(self).resources.timer_set_view(), old(self).resources.timer_map_view(), old(self).next_resource_id as nat),
      slab_alloc_inv(old(self).resources@, old(self).next_resource_id as nat),
      free_rids_wf(old(self).free_rids@, old(self).log@, old(self).resources@, old(self).next_resource_id as nat),
      data_inv(
        old(self).resources.timer_set_view(), old(self).resources.timer_map_view(),
        old(self).resources.timer_wakers_view(), old(self).resources.read_wakers_view(), old(self).resources.write_wakers_view(),
        old(self).log@,
      ),
      old(self).resources.timer_map_view().contains_key(resource_id@),
      wheel_slab_consistent(old(self).wheel@, old(self).resources.timer_map_view()),
      old(self).wheel.pending@.len() == 0,
    ensures
      self.wheel.full_wf(),
      self.wheel.pending@.len() == 0,
      self.wheel.elapsed == old(self).wheel.elapsed,
      self.resources.slab_wf(),
      reactor_wf(self.log@, self.next_resource_id as nat),
      timer_impl_inv(self.resources.timer_set_view(), self.resources.timer_map_view(), self.next_resource_id as nat),
      slab_alloc_inv(self.resources@, self.next_resource_id as nat),
      free_rids_wf(self.free_rids@, self.log@, self.resources@, self.next_resource_id as nat),
      data_inv(
        self.resources.timer_set_view(), self.resources.timer_map_view(),
        self.resources.timer_wakers_view(), self.resources.read_wakers_view(), self.resources.write_wakers_view(),
        self.log@,
      ),
      wheel_slab_consistent(self.wheel@, self.resources.timer_map_view()),
  {
    let ghost old_log = self.log@;
    let ghost old_ts = self.resources.timer_set_view();
    let ghost old_tm = self.resources.timer_map_view();
    let ghost old_tw = self.resources.timer_wakers_view();
    let ghost old_rw = self.resources.read_wakers_view();
    let ghost old_ww = self.resources.write_wakers_view();
    let ghost rid = resource_id@;

    self.deregister_timer_begin_action(resource_id);

    let ghost log_after = self.log@;

    self.wheel.remove(resource_id.0);
    proof {
      assert(self.resources.timer_map_view() == old_tm);
      self.resources.timer_map_key_is_timer(rid);
    }
    self.resources.v_remove_timer_slot(resource_id.0);
    // [RID-REUSE DISABLED — restore when generational ids are added]
    // self.free_rids.push(resource_id.0);

    let ghost new_ts = self.resources.timer_set_view();
    let ghost new_tm = self.resources.timer_map_view();
    let ghost new_tw = self.resources.timer_wakers_view();

    proof {
      let begin_event = log_after[old_log.len() as int];
      assert(log_after == old_log.push(begin_event));

      reactor_inv_preserved_by_non_trigger(old_log, begin_event);
      reactor_ext_inv_preserved_by_non_trigger(old_log, begin_event);
      alloc_inv_preserved_by_non_registration(old_log, begin_event, self.next_resource_id as nat);

      if old_tm.contains_key(rid) {
        let entry = old_tm[rid];
        assert(new_ts == old_ts.remove(entry));
        assert(new_tm == old_tm.remove(rid));
        assert(new_tw == old_tw.remove(rid));

        assert forall |d: InstantView, r: ResourceIdView, i: int|
          new_ts.contains((d, r, i)) implies old_ts.contains((d, r, i)) && r != rid
        by {
          assert(old_ts.contains((d, r, i)));
          assert((d, r, i) != entry);
          assert(old_tm.contains_key(r) && old_tm[r] == (d, r, i));
          if r == rid {
            assert(old_tm[rid] == entry);
            assert((d, r, i) == entry);
            assert(false);
          }
        }

        assert forall |d: InstantView, r: ResourceIdView, i: int|
          old_ts.contains((d, r, i)) && r != rid implies new_ts.contains((d, r, i))
        by {
          assert(old_tm.contains_key(r) && old_tm[r] == (d, r, i));
          assert((d, r, i) != entry);
        }

        data_inv_preserved_by_deregister_timer(
          old_ts, new_ts, old_tm, old_tw, old_rw, old_ww,
          old_log, begin_event, rid,
        );

        assert(new_ts.finite());
        assert forall |r2: nat| new_tm.contains_key(r2)
          implies new_ts.contains(new_tm[r2]) && new_tm[r2].1 == r2 && 1 <= r2 && r2 < self.next_resource_id as nat
        by {
          assert(r2 != rid);
          assert(old_tm.contains_key(r2));
          assert(old_ts.contains(old_tm[r2]));
          assert(old_tm[r2].1 == r2);
          assert(new_tm[r2] == old_tm[r2]);
          assert(r2 != rid);
          assert(new_ts.contains(old_tm[r2]));
        }
        assert forall |d: InstantView, r: ResourceIdView, i: int|
          new_ts.contains((d, r, i)) implies
          new_tm.contains_key(r) && new_tm[r] == (d, r, i)
        by {
          assert(old_ts.contains((d, r, i)));
          assert(r != rid);
          assert(old_tm.contains_key(r) && old_tm[r] == (d, r, i));
          assert(new_tm.contains_key(r));
          assert(new_tm[r] == old_tm[r]);
        }
      } else {
        assert(new_ts == old_ts);
        assert(new_tm == old_tm);
        assert(new_tw == old_tw);

        assert forall |d: InstantView, r: ResourceIdView, i: int|
          new_ts.contains((d, r, i)) implies old_ts.contains((d, r, i)) && r != rid
        by {
          assert(old_ts.contains((d, r, i)));
          assert(old_tm.contains_key(r) && old_tm[r] == (d, r, i));
          assert(r != rid);
        }

        data_inv_preserved_by_deregister_timer(
          old_ts, new_ts, old_tm, old_tw, old_rw, old_ww,
          old_log, begin_event, rid,
        );

        assert(old_tm.remove(rid) =~= old_tm);
        assert(!old_tw.contains_key(rid));
        assert(old_tw.remove(rid) =~= old_tw);
      }

      assert(wheel_slab_consistent(self.wheel@, new_tm)) by {
        let new_wheel = self.wheel@;
        assert(new_wheel =~= old(self).wheel@.remove(rid));
        assert forall |r: nat| #![auto] new_wheel.contains_key(r) <==> new_tm.contains_key(r) by {
          assert(new_wheel.contains_key(r) <==> (r != rid && old(self).wheel@.contains_key(r)));
          assert(new_tm.contains_key(r) <==> (r != rid && old_tm.contains_key(r)));
        };
        assert forall |r: nat| #![auto] new_wheel.contains_key(r) implies new_wheel[r] == new_tm[r].0 by {
          assert(r != rid);
          assert(new_wheel[r] == old(self).wheel@[r]);
          assert(new_tm[r] == old_tm[r]);
          assert(old(self).wheel@[r] == old_tm[r].0);
        };
      };

      assert(old_tm.contains_key(rid));
      let entry = old_tm[rid];
      assert(old_ts.contains(entry));
      assert(old_ts.contains((entry.0, entry.1, entry.2)));
      let timer_log_idx = entry.2;
      assert(timer_heap_entries_valid(old_ts, old_log));
      assert(timer_awaiting_wake(old_log, timer_log_idx));
      assert(is_succ_register_timer_at(old_log, timer_log_idx));
      assert(get_register_timer_rid(old_log[timer_log_idx]) == rid);
      assert(timer_active_at(old_log, timer_log_idx, old_log.len() as int));

      reactor_inv_split(old_log);
      timer_rid_has_no_io_registration(old_log, rid, timer_log_idx);

      // [RID-REUSE DISABLED — restore when generational ids are added]
      // Push removed: free_rids unchanged. Re-derive free_rids_wf for the UNCHANGED
      // seq across the deregister_timer event append + timer-slot removal (the push
      // lemma's first two internal steps, minus the final push).
      free_rids_wf_preserved_by_append(
        old(self).free_rids@, old_log, begin_event,
        old(self).resources@, self.next_resource_id as nat,
      );
      free_rids_wf_preserved_by_resource_remove(
        old(self).free_rids@, log_after, old(self).resources@,
        self.next_resource_id as nat, rid,
      );
    }
  }

  #[verifier::external_body]
  pub fn next_deadline(&self) -> (result: Option<Instant>)
  {
    match self.wheel.next_deadline() {
      Some(d) => Some(Instant { inner: d }),
      None => None,
    }
  }

  #[verifier::external_body]
  #[inline]
  pub fn flush_pending_deregister(&mut self)
  {
    if let Some((rid, _)) = self.pending_deregister.take() {
      self.wheel.remove(rid);
      self.resources.v_remove_timer_slot(rid);
      // [RID-REUSE DISABLED — restore when generational ids are added]
      // self.free_rids.push(rid);
    }
  }
}

}
