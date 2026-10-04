use crate::reactor::Reactor;
use crate::readiness;
use crate::types::{Duration, Instant, IoEvent, IoMode, IoResult, ResourceId, Waker};
use crate::invariants::*;
use crate::invariants::data_inv::*;
use crate::spec::predicates::*;
use crate::spec::log::*;
use crate::spec::types::*;
use crate::proof::preservation::*;
use crate::proof::safety_preservation::*;
use crate::proof::liveness_preservation::*;
use crate::proof::preservation_ext::*;
use crate::proof::park_safety::*;
use crate::invariants::park_has_timestamp::*;
use crate::invariants::park_poll_once::*;
use crate::invariants::io_ready_in_park::*;
use crate::resource_slab::ResourceSlab;
use vstd::prelude::*;

verus! {

#[verifier::external_body]
fn mark_io_readable(rid: ResourceId) {
  readiness::mark_readable(rid);
}

#[verifier::external_body]
fn mark_io_writable(rid: ResourceId) {
  readiness::mark_writable(rid);
}

impl Reactor {
  fn try_pop_expired_timer(&mut self, now: Instant)
    -> (result: Option<(ResourceId, Ghost<int>, Ghost<InstantView>)>)
    requires
      old(self).wheel.full_wf(),
      old(self).resources.slab_wf(),
      wheel_slab_consistent(old(self).wheel@, old(self).resources.timer_map_view()),
      timer_impl_inv(old(self).resources.timer_set_view(), old(self).resources.timer_map_view(), old(self).next_resource_id as nat),
      now.inner >= old(self).wheel.elapsed,
    ensures
      self.wheel.full_wf(),
      self.resources.slab_wf(),
      self.resources@ == old(self).resources@,
      self.resources.timer_set_view() == old(self).resources.timer_set_view(),
      self.resources.timer_map_view() == old(self).resources.timer_map_view(),
      self.resources.timer_wakers_view() == old(self).resources.timer_wakers_view(),
      self.resources.read_wakers_view() == old(self).resources.read_wakers_view(),
      self.resources.write_wakers_view() == old(self).resources.write_wakers_view(),
      self.next_resource_id == old(self).next_resource_id,
      self.log@ == old(self).log@,
      self.free_rids == old(self).free_rids,
      self.wheel.elapsed <= now.inner,
      result.is_none() ==> self.wheel.pending@.len() == 0,
      result.is_some() ==> {
        let (rid, log_idx, deadline) = result.unwrap();
        old(self).resources.timer_map_view().contains_key(rid@) &&
        old(self).resources.timer_map_view()[rid@] == (deadline@, rid@, log_idx@) &&
        deadline@ <= now@ &&
        self.wheel@ == old(self).wheel@.remove(rid@)
      },
      result.is_none() ==> (
        forall |d: InstantView, r: ResourceIdView, log_idx: int|
          #![auto] old(self).resources.timer_set_view().contains((d, r, log_idx)) ==> d > now@
      ),
      result.is_none() ==> self.wheel@ == old(self).wheel@,
  {
    let ghost old_ts = self.resources.timer_set_view();
    let ghost old_tm = self.resources.timer_map_view();
    let ghost old_wv = self.wheel@;
    let ghost old_nrid = self.next_resource_id as nat;

    proof {
      assert(old_ts == old(self).resources.timer_set_view());
      assert(old_tm == old(self).resources.timer_map_view());
      assert(old_wv == old(self).wheel@);
      assert(old_nrid == old(self).next_resource_id as nat);
      assert(timer_impl_inv(old_ts, old_tm, old_nrid));
      assert(wheel_slab_consistent(old_wv, old_tm));
    }

    match self.wheel.try_pop_expired(now.inner) {
      None => {
        proof {
          assert forall |d: InstantView, r: ResourceIdView, log_idx: int|
            #![auto] old_ts.contains((d, r, log_idx)) implies d > now@
          by {
            assert(old_wv.contains_key(r));
            assert(old_wv[r] > now.inner as int);
            assert(old_wv[r] == old_tm[r].0);
          };
        }
        None
      }
      Some(rid_u64) => {
        let resource_id = ResourceId(rid_u64);
        let ghost rid: nat = rid_u64 as nat;
        proof {
          assert(old_wv.contains_key(rid));
          assert(old_tm.contains_key(rid));
          assert(old_tm[rid].1 == rid);
        }
        let ghost entry = old_tm[rid];
        let log_idx: Ghost<int> = Ghost(entry.2);
        let deadline: Ghost<InstantView> = Ghost(entry.0);
        proof {
          assert(old_wv[rid] <= now.inner as int);
          assert(old_wv[rid] == old_tm[rid].0);
          assert(entry.0 <= now@);
          assert(old_tm[rid] == (deadline@, rid as nat, log_idx@));
        }
        Some((resource_id, log_idx, deadline))
      }
    }
  }

  fn wake_expired_timers(&mut self, now: Instant)
    requires
      old(self).wheel.full_wf(),
      old(self).resources.slab_wf(),
      reactor_safety_inv(old(self).log@),
      reactor_ext_inv(old(self).log@),
      alloc_inv(old(self).log@, old(self).next_resource_id as nat),
      timer_impl_inv(old(self).resources.timer_set_view(), old(self).resources.timer_map_view(), old(self).next_resource_id as nat),
      slab_alloc_inv(old(self).resources@, old(self).next_resource_id as nat),
      free_rids_wf(old(self).free_rids@, old(self).log@, old(self).resources@, old(self).next_resource_id as nat),
      data_inv(
        old(self).resources.timer_set_view(), old(self).resources.timer_map_view(),
        old(self).resources.timer_wakers_view(), old(self).resources.read_wakers_view(), old(self).resources.write_wakers_view(),
        old(self).log@,
      ),
      wheel_slab_consistent(old(self).wheel@, old(self).resources.timer_map_view()),
      now.inner >= old(self).wheel.elapsed,
    ensures
      self.resources.slab_wf(),
      reactor_safety_inv(self.log@),
      reactor_ext_inv(self.log@),
      alloc_inv(self.log@, self.next_resource_id as nat),
      timer_impl_inv(self.resources.timer_set_view(), self.resources.timer_map_view(), self.next_resource_id as nat),
      slab_alloc_inv(self.resources@, self.next_resource_id as nat),
      free_rids_wf(self.free_rids@, self.log@, self.resources@, self.next_resource_id as nat),
      data_inv(
        self.resources.timer_set_view(), self.resources.timer_map_view(),
        self.resources.timer_wakers_view(), self.resources.read_wakers_view(), self.resources.write_wakers_view(),
        self.log@,
      ),
      self.wheel.full_wf(),
      self.wheel.pending@.len() == 0,
      self.wheel.elapsed <= now.inner,
      wheel_slab_consistent(self.wheel@, self.resources.timer_map_view()),
      self.next_resource_id == old(self).next_resource_id,
      self.log@.len() >= old(self).log@.len(),
      forall |k: int| #![auto] 0 <= k < old(self).log@.len() ==> self.log@[k] == old(self).log@[k],
      forall |k: int| #![auto] old(self).log@.len() as int <= k < self.log@.len() ==>
        !is_succ_register_timer_at(self.log@, k) &&
        !io_api_registered_at(self.log@, k) &&
        !is_deregister_timer_at(self.log@, k) &&
        !io_api_deregistered_at(self.log@, k) &&
        !is_succ_set_waker_at(self.log@, k) &&
        !is_io_event_ready_at(self.log@, k) &&
        !is_park_begin_at(self.log@, k) &&
        !is_park_end_at(self.log@, k) &&
        !is_get_current_time_at(self.log@, k) &&
        !is_poll_events_at(self.log@, k),
      forall |d: InstantView, r: ResourceIdView, log_idx: int|
        #![auto] self.resources.timer_set_view().contains((d, r, log_idx)) ==> d > now@,
      self.resources.read_wakers_view() == old(self).resources.read_wakers_view(),
      self.resources.write_wakers_view() == old(self).resources.write_wakers_view(),
  {
    let ghost old_log = self.log@;
    let ghost old_rw = self.resources.read_wakers_view();
    let ghost old_ww = self.resources.write_wakers_view();
    let ghost old_next_rid = self.next_resource_id as nat;
    let ghost old_next_rid_u64 = self.next_resource_id;

    proof {
      assert(old_next_rid_u64 == old(self).next_resource_id);
      assert(old_log == old(self).log@);
      assert(old_rw == old(self).resources.read_wakers_view());
      assert(old_ww == old(self).resources.write_wakers_view());
    }

    loop
      invariant
        self.wheel.full_wf(),
        now.inner >= self.wheel.elapsed,
        self.resources.slab_wf(),
        reactor_safety_inv(self.log@),
        reactor_ext_inv(self.log@),
        alloc_inv(self.log@, self.next_resource_id as nat),
        timer_impl_inv(self.resources.timer_set_view(), self.resources.timer_map_view(), self.next_resource_id as nat),
        slab_alloc_inv(self.resources@, self.next_resource_id as nat),
        free_rids_wf(self.free_rids@, self.log@, self.resources@, self.next_resource_id as nat),
        data_inv(
          self.resources.timer_set_view(), self.resources.timer_map_view(),
          self.resources.timer_wakers_view(), self.resources.read_wakers_view(), self.resources.write_wakers_view(),
          self.log@,
        ),
        wheel_slab_consistent(self.wheel@, self.resources.timer_map_view()),
        self.next_resource_id == old_next_rid_u64,
        self.next_resource_id as nat == old_next_rid,
        self.log@.len() >= old_log.len(),
        forall |k: int| #![auto] 0 <= k < old_log.len() ==> self.log@[k] == old_log[k],
        forall |k: int| #![auto] old_log.len() as int <= k < self.log@.len() ==>
          !is_succ_register_timer_at(self.log@, k) &&
          !io_api_registered_at(self.log@, k) &&
          !is_deregister_timer_at(self.log@, k) &&
          !io_api_deregistered_at(self.log@, k) &&
          !is_succ_set_waker_at(self.log@, k) &&
          !is_io_event_ready_at(self.log@, k) &&
          !is_park_begin_at(self.log@, k) &&
          !is_park_end_at(self.log@, k) &&
          !is_get_current_time_at(self.log@, k) &&
          !is_poll_events_at(self.log@, k),
        self.resources.read_wakers_view() == old_rw,
        self.resources.write_wakers_view() == old_ww,
        old_next_rid_u64 == old(self).next_resource_id,
        old_log =~= old(self).log@,
        old_rw == old(self).resources.read_wakers_view(),
        old_ww == old(self).resources.write_wakers_view(),
      decreases self.resources.timer_set_view().len(),
    {
      let ghost pre_ts = self.resources.timer_set_view();
      let ghost pre_tm = self.resources.timer_map_view();
      let ghost pre_tw = self.resources.timer_wakers_view();
      let ghost pre_log = self.log@;
      let ghost pre_wheel = self.wheel@;
      let ghost pre_free_rids = self.free_rids@;
      let ghost pre_resources = self.resources@;

      match self.try_pop_expired_timer(now) {
        None => {
          return;
        }
        Some((rid, log_idx_ghost, deadline_ghost)) => {
          proof {
            assert(timer_heap_has_wakers(pre_tw, pre_tm));
            assert(pre_tm.contains_key(rid@));
            assert(pre_tw.contains_key(rid@));
          }

          let waker_opt = self.resources.v_take_timer_waker(rid.0);

          match waker_opt {
            Some(waker) => {
              self.wake_task_action(&waker, rid);
              let ghost log_wake = self.log@;

              proof {
                assert(self.resources.timer_map_view() == pre_tm);
                self.resources.timer_map_key_is_timer(rid@);
              }
              self.resources.v_remove_timer_slot(rid.0);
              // [RID-REUSE DISABLED — restore when generational ids are added]
              // self.free_rids.push(rid.0);

              proof {
                let e = log_wake[pre_log.len() as int];
                assert(log_wake == pre_log.push(e));

                timer_wake_remove_step(
                  pre_log, e,
                  pre_ts, pre_tm, pre_tw,
                  old_rw, old_ww,
                  old_next_rid,
                  rid@,
                );

                assert forall |k: int| #![auto] old_log.len() as int <= k < log_wake.len() implies
                  !is_succ_register_timer_at(log_wake, k) &&
                  !io_api_registered_at(log_wake, k) &&
                  !is_deregister_timer_at(log_wake, k) &&
                  !io_api_deregistered_at(log_wake, k) &&
                  !is_succ_set_waker_at(log_wake, k) &&
                  !is_io_event_ready_at(log_wake, k) &&
                  !is_park_begin_at(log_wake, k) &&
                  !is_park_end_at(log_wake, k) &&
                  !is_get_current_time_at(log_wake, k) &&
                  !is_poll_events_at(log_wake, k)
                by {
                  if k < pre_log.len() as int {
                    assert(log_wake[k] == pre_log[k]);
                    assert(!is_succ_register_timer_at(pre_log, k));
                    assert(!io_api_registered_at(pre_log, k));
                    assert(!is_deregister_timer_at(pre_log, k));
                    assert(!io_api_deregistered_at(pre_log, k));
                    assert(!is_succ_set_waker_at(pre_log, k));
                    assert(!is_io_event_ready_at(pre_log, k));
                    assert(!is_park_begin_at(pre_log, k));
                    assert(!is_park_end_at(pre_log, k));
                    assert(!is_get_current_time_at(pre_log, k));
                    assert(!is_poll_events_at(pre_log, k));
                  }
                };

                let entry = pre_tm[rid@];
                assert(pre_ts.contains(entry));
                assert(self.resources.timer_set_view() == pre_ts.remove(entry));
                assert(pre_ts.finite());

                assert(slab_alloc_inv(self.resources@, self.next_resource_id as nat)) by {
                  assert forall |k: nat| #![auto] self.resources@.contains_key(k) implies
                    1 <= k && k < self.next_resource_id as nat
                  by {
                    assert(self.resources@.contains_key(k));
                  }
                };

                assert(wheel_slab_consistent(self.wheel@, self.resources.timer_map_view())) by {
                  let new_wheel = self.wheel@;
                  let new_tm = self.resources.timer_map_view();
                  assert(new_tm =~= pre_tm.remove(rid@));
                  assert(new_wheel =~= pre_wheel.remove(rid@));
                  assert forall |r: nat| #![auto] new_wheel.contains_key(r) <==> new_tm.contains_key(r) by {
                    assert(new_wheel.contains_key(r) <==> (r != rid@ && pre_wheel.contains_key(r)));
                    assert(new_tm.contains_key(r) <==> (r != rid@ && pre_tm.contains_key(r)));
                    assert(pre_wheel.contains_key(r) <==> pre_tm.contains_key(r));
                  };
                  assert forall |r: nat| #![auto] new_wheel.contains_key(r) implies new_wheel[r] == new_tm[r].0 by {
                    assert(r != rid@);
                    assert(new_wheel[r] == pre_wheel[r]);
                    assert(new_tm[r] == pre_tm[r]);
                    assert(pre_wheel[r] == pre_tm[r].0);
                  };
                };

                let timer_entry = pre_tm[rid@];
                assert(pre_ts.contains(timer_entry));
                assert(pre_ts.contains((timer_entry.0, timer_entry.1, timer_entry.2)));
                assert(timer_heap_entries_valid(pre_ts, pre_log));
                assert(timer_awaiting_wake(pre_log, timer_entry.2));
                assert(is_succ_register_timer_at(pre_log, timer_entry.2));
                assert(get_register_timer_rid(pre_log[timer_entry.2]) == rid@);
                assert(timer_active_at(pre_log, timer_entry.2, pre_log.len() as int));

                timer_rid_has_no_io_registration(pre_log, rid@, timer_entry.2);

                assert(pre_resources.contains_key(rid@));
                assert(rid@ >= 1 && rid@ < old_next_rid);

                assert(free_rids_wf(pre_free_rids, pre_log, pre_resources, old_next_rid));
                free_rids_wf_preserved_by_append(pre_free_rids, pre_log, e, pre_resources, old_next_rid);
                assert(free_rids_wf(pre_free_rids, log_wake, pre_resources, old_next_rid));
                free_rids_wf_preserved_by_resource_remove(pre_free_rids, log_wake, pre_resources, old_next_rid, rid@);
                assert(free_rids_wf(pre_free_rids, log_wake, self.resources@, old_next_rid));
                // [RID-REUSE DISABLED — restore when generational ids are added]
                // Push removed: free_rids stays == pre_free_rids. The UNCHANGED-seq
                // free_rids_wf at (log_wake, self.resources@) is already established above
                // (free_rids_wf_preserved_by_append + _by_resource_remove); the freed-rid
                // push apparatus below is disabled.
                // free_rids_wf_disjoint_from_key(pre_free_rids, pre_log, pre_resources, old_next_rid, rid@);
                // no_prior_timer_reg_after_wake(pre_log, e, rid@);
                // no_prior_io_api_reg_preserved(pre_log, e, rid@, pre_log.len() as int);
                // no_io_api_with_rid_preserved(pre_log, e, rid@, pre_log.len() as int);
                // assert(!self.resources@.contains_key(rid@));
                // free_rids_wf_push_deregistered_timer(pre_free_rids, log_wake, self.resources@, old_next_rid, rid.0);
                assert(self.free_rids@ == pre_free_rids);
                assert(self.log@ == log_wake);
                assert(self.next_resource_id as nat == old_next_rid);
                assert(free_rids_wf(self.free_rids@, self.log@, self.resources@, self.next_resource_id as nat));
              }
            }
            None => {
              proof {
                assert(pre_tw.contains_key(rid@));
                assert(self.resources.timer_wakers_view() == pre_tw);
                assert(self.resources.timer_wakers_view().contains_key(rid.0 as nat));
                assert(false);
              }
            }
          }
        }
      }
    }
  }

  #[verifier::rlimit(200)]
  fn process_io_events(&mut self, io_events: &Vec<IoEvent>)
    requires
      old(self).wheel.full_wf(),
      old(self).resources.slab_wf(),
      reactor_safety_inv(old(self).log@),
      reactor_ext_inv(old(self).log@),
      alloc_inv(old(self).log@, old(self).next_resource_id as nat),
      timer_impl_inv(old(self).resources.timer_set_view(), old(self).resources.timer_map_view(), old(self).next_resource_id as nat),
      slab_alloc_inv(old(self).resources@, old(self).next_resource_id as nat),
      free_rids_wf(old(self).free_rids@, old(self).log@, old(self).resources@, old(self).next_resource_id as nat),
      data_inv(
        old(self).resources.timer_set_view(), old(self).resources.timer_map_view(),
        old(self).resources.timer_wakers_view(), old(self).resources.read_wakers_view(), old(self).resources.write_wakers_view(),
        old(self).log@,
      ),
      current_park_start(old(self).log@, old(self).log@.len() as int) >= 0,
      wheel_slab_consistent(old(self).wheel@, old(self).resources.timer_map_view()),
    ensures
      self.wheel.full_wf(),
      self.wheel.elapsed == old(self).wheel.elapsed,
      self.wheel.pending@ == old(self).wheel.pending@,
      self.resources.slab_wf(),
      reactor_safety_inv(self.log@),
      reactor_ext_inv(self.log@),
      alloc_inv(self.log@, self.next_resource_id as nat),
      timer_impl_inv(self.resources.timer_set_view(), self.resources.timer_map_view(), self.next_resource_id as nat),
      slab_alloc_inv(self.resources@, self.next_resource_id as nat),
      free_rids_wf(self.free_rids@, self.log@, self.resources@, self.next_resource_id as nat),
      data_inv(
        self.resources.timer_set_view(), self.resources.timer_map_view(),
        self.resources.timer_wakers_view(), self.resources.read_wakers_view(), self.resources.write_wakers_view(),
        self.log@,
      ),
      wheel_slab_consistent(self.wheel@, self.resources.timer_map_view()),
      self.next_resource_id == old(self).next_resource_id,
      self.resources.timer_set_view() == old(self).resources.timer_set_view(),
      self.resources.timer_map_view() == old(self).resources.timer_map_view(),
      self.resources.timer_wakers_view() == old(self).resources.timer_wakers_view(),
      self.resources.read_wakers_view() == old(self).resources.read_wakers_view(),
      self.resources.write_wakers_view() == old(self).resources.write_wakers_view(),
      self.log@.len() >= old(self).log@.len(),
      forall |k: int| #![auto] 0 <= k < old(self).log@.len() ==> self.log@[k] == old(self).log@[k],
      forall |k: int| #![auto] old(self).log@.len() as int <= k < self.log@.len() ==>
        !is_succ_register_timer_at(self.log@, k) &&
        !io_api_registered_at(self.log@, k) &&
        !is_deregister_timer_at(self.log@, k) &&
        !io_api_deregistered_at(self.log@, k) &&
        !is_succ_set_waker_at(self.log@, k) &&
        !is_park_begin_at(self.log@, k) &&
        !is_park_end_at(self.log@, k) &&
        !is_get_current_time_at(self.log@, k) &&
        !is_poll_events_at(self.log@, k),
      forall |p: int| #![auto] old(self).log@.len() as int <= p < self.log@.len() &&
        is_io_event_ready_at(self.log@, p) &&
        get_io_event(self.log@[p]).readable &&
        old(self).resources.read_wakers_view().contains_key(get_io_event(self.log@[p]).resource_id) ==> {
          let rid = get_io_event(self.log@[p]).resource_id;
          p + 1 < self.log@.len() as int &&
          is_wake_task_at(self.log@, p + 1) &&
          get_wake_task_source_rid(self.log@[p + 1]) == rid &&
          get_wake_task_waker(self.log@[p + 1]) == old(self).resources.read_wakers_view()[rid]
        },
      forall |p: int| #![auto] old(self).log@.len() as int <= p < self.log@.len() &&
        is_io_event_ready_at(self.log@, p) &&
        get_io_event(self.log@[p]).writable &&
        old(self).resources.write_wakers_view().contains_key(get_io_event(self.log@[p]).resource_id) ==> {
          let rid = get_io_event(self.log@[p]).resource_id;
          p + 1 < self.log@.len() as int &&
          is_wake_task_at(self.log@, p + 1) &&
          get_wake_task_source_rid(self.log@[p + 1]) == rid &&
          get_wake_task_waker(self.log@[p + 1]) == old(self).resources.write_wakers_view()[rid]
        },
  {
    let ghost old_log = self.log@;
    let ghost old_ts = self.resources.timer_set_view();
    let ghost old_tm = self.resources.timer_map_view();
    let ghost old_tw = self.resources.timer_wakers_view();
    let ghost old_rw = self.resources.read_wakers_view();
    let ghost old_ww = self.resources.write_wakers_view();
    let ghost old_next_rid = self.next_resource_id as nat;
    let ghost old_wheel_view = self.wheel@;

    let num_events = io_events.len();
    let mut i: usize = 0;
    while i < num_events
      invariant
        0 <= i <= num_events,
        num_events == io_events@.len(),
        self.wheel.full_wf(),
        self.resources.slab_wf(),
        reactor_safety_inv(self.log@),
        reactor_ext_inv(self.log@),
        alloc_inv(self.log@, self.next_resource_id as nat),
        timer_impl_inv(self.resources.timer_set_view(), self.resources.timer_map_view(), self.next_resource_id as nat),
        slab_alloc_inv(self.resources@, self.next_resource_id as nat),
        free_rids_wf(self.free_rids@, self.log@, self.resources@, self.next_resource_id as nat),
        data_inv(
          self.resources.timer_set_view(), self.resources.timer_map_view(),
          self.resources.timer_wakers_view(), self.resources.read_wakers_view(), self.resources.write_wakers_view(),
          self.log@,
        ),
        self.wheel@ == old_wheel_view,
        self.wheel.elapsed == old(self).wheel.elapsed,
        self.wheel.pending@ == old(self).wheel.pending@,
        self.next_resource_id as nat == old_next_rid,
        self.resources.timer_set_view() == old_ts,
        self.resources.timer_map_view() == old_tm,
        self.resources.timer_wakers_view() == old_tw,
        self.resources.read_wakers_view() == old_rw,
        self.resources.write_wakers_view() == old_ww,
        self.log@.len() >= old_log.len(),
        forall |k: int| #![auto] 0 <= k < old_log.len() ==> self.log@[k] == old_log[k],
        forall |k: int| #![auto] old_log.len() as int <= k < self.log@.len() ==>
          !is_succ_register_timer_at(self.log@, k) &&
          !io_api_registered_at(self.log@, k) &&
          !is_deregister_timer_at(self.log@, k) &&
          !io_api_deregistered_at(self.log@, k) &&
          !is_succ_set_waker_at(self.log@, k) &&
          !is_park_begin_at(self.log@, k) &&
          !is_park_end_at(self.log@, k) &&
          !is_get_current_time_at(self.log@, k) &&
          !is_poll_events_at(self.log@, k),
        current_park_start(self.log@, self.log@.len() as int) >= 0,
        forall |p: int| #![auto] old_log.len() as int <= p < self.log@.len() &&
          is_io_event_ready_at(self.log@, p) &&
          get_io_event(self.log@[p]).readable &&
          old_rw.contains_key(get_io_event(self.log@[p]).resource_id) ==> {
            let rid = get_io_event(self.log@[p]).resource_id;
            p + 1 < self.log@.len() as int &&
            is_wake_task_at(self.log@, p + 1) &&
            get_wake_task_source_rid(self.log@[p + 1]) == rid &&
            get_wake_task_waker(self.log@[p + 1]) == old_rw[rid]
          },
        forall |p: int| #![auto] old_log.len() as int <= p < self.log@.len() &&
          is_io_event_ready_at(self.log@, p) &&
          get_io_event(self.log@[p]).writable &&
          old_ww.contains_key(get_io_event(self.log@[p]).resource_id) ==> {
            let rid = get_io_event(self.log@[p]).resource_id;
            p + 1 < self.log@.len() as int &&
            is_wake_task_at(self.log@, p + 1) &&
            get_wake_task_source_rid(self.log@[p + 1]) == rid &&
            get_wake_task_waker(self.log@[p + 1]) == old_ww[rid]
          },
      decreases num_events - i,
    {
      let event = &io_events[i];
      let ghost pre_log = self.log@;

      self.io_event_ready_action(event);
      let ghost log_ier = self.log@;
      let rid = event.resource_id;

      proof {
        let e1 = log_ier[pre_log.len() as int];
        assert(log_ier == pre_log.push(e1));
        io_event_ready_step_inv(pre_log, e1, old_ts, old_tm, old_tw, old_rw, old_ww, old_next_rid);
        free_rids_wf_preserved_by_append(self.free_rids@, pre_log, e1, self.resources@, old_next_rid);
        let ier_pos = pre_log.len() as int;
        assert(!is_succ_register_timer_at(log_ier, ier_pos));
        assert(!io_api_registered_at(log_ier, ier_pos));
        assert(!is_deregister_timer_at(log_ier, ier_pos));
        assert(!io_api_deregistered_at(log_ier, ier_pos));
        assert(!is_succ_set_waker_at(log_ier, ier_pos));
        assert(!is_park_begin_at(log_ier, ier_pos));
        assert(!is_park_end_at(log_ier, ier_pos));
        assert(!is_get_current_time_at(log_ier, ier_pos));
        assert(!is_poll_events_at(log_ier, ier_pos));
      }

      match event.mode {
        IoMode::Readable => {
          mark_io_readable(rid);
          let waker_opt = self.resources.v_get_read_waker(rid.0);
          match waker_opt {
            Some(waker) => {
              self.wake_task_action(&waker, rid);
              let ghost log_wake = self.log@;

              proof {
                let e2 = log_wake[log_ier.len() as int];
                assert(log_wake == log_ier.push(e2));
                free_rids_wf_preserved_by_append(self.free_rids@, log_ier, e2, self.resources@, old_next_rid);

                let ghost ev_rid: ResourceIdView = rid@;
                assert(old_rw.contains_key(ev_rid));
                assert(waker@ == old_rw[ev_rid]);
                assert(get_wake_task_waker(log_wake[log_ier.len() as int]) == old_rw[ev_rid]);

                assert(read_wakers_valid(old_rw, log_ier));
                assert(io_currently_active(log_ier, ev_rid));

                io_wake_step_inv(
                  log_ier, e2, old_ts, old_tm, old_tw, old_rw, old_ww, old_next_rid, ev_rid, true,
                );

                let wt_pos = log_ier.len() as int;
                assert(!is_succ_register_timer_at(log_wake, wt_pos));
                assert(!io_api_registered_at(log_wake, wt_pos));
                assert(!is_deregister_timer_at(log_wake, wt_pos));
                assert(!io_api_deregistered_at(log_wake, wt_pos));
                assert(!is_succ_set_waker_at(log_wake, wt_pos));
                assert(!is_park_begin_at(log_wake, wt_pos));
                assert(!is_park_end_at(log_wake, wt_pos));
                assert(!is_get_current_time_at(log_wake, wt_pos));
                assert(!is_poll_events_at(log_wake, wt_pos));
                assert(!is_io_event_ready_at(log_wake, wt_pos));

                assert forall |p: int| #![auto] old_log.len() as int <= p < log_wake.len() &&
                  is_io_event_ready_at(log_wake, p) &&
                  get_io_event(log_wake[p]).readable &&
                  old_rw.contains_key(get_io_event(log_wake[p]).resource_id) implies {
                    let r = get_io_event(log_wake[p]).resource_id;
                    p + 1 < log_wake.len() as int &&
                    is_wake_task_at(log_wake, p + 1) &&
                    get_wake_task_source_rid(log_wake[p + 1]) == r &&
                    get_wake_task_waker(log_wake[p + 1]) == old_rw[r]
                  }
                by {
                  if p < pre_log.len() as int {
                    assert(log_wake[p] == pre_log[p]);
                    assert(is_io_event_ready_at(pre_log, p));
                    assert(get_io_event(pre_log[p]).readable);
                    assert(old_rw.contains_key(get_io_event(pre_log[p]).resource_id));
                    assert(p + 1 < pre_log.len() as int);
                    assert(log_wake[p + 1] == pre_log[p + 1]);
                  } else if p == pre_log.len() as int {
                    assert(p + 1 == log_ier.len() as int);
                    assert(p + 1 < log_wake.len() as int);
                  } else {
                    assert(!is_io_event_ready_at(log_wake, p));
                  }
                };

                assert forall |p: int| #![auto] old_log.len() as int <= p < log_wake.len() &&
                  is_io_event_ready_at(log_wake, p) &&
                  get_io_event(log_wake[p]).writable &&
                  old_ww.contains_key(get_io_event(log_wake[p]).resource_id) implies {
                    let r = get_io_event(log_wake[p]).resource_id;
                    p + 1 < log_wake.len() as int &&
                    is_wake_task_at(log_wake, p + 1) &&
                    get_wake_task_source_rid(log_wake[p + 1]) == r &&
                    get_wake_task_waker(log_wake[p + 1]) == old_ww[r]
                  }
                by {
                  if p < pre_log.len() as int {
                    assert(log_wake[p] == pre_log[p]);
                    assert(is_io_event_ready_at(pre_log, p));
                    assert(get_io_event(pre_log[p]).writable);
                    assert(old_ww.contains_key(get_io_event(pre_log[p]).resource_id));
                    assert(p + 1 < pre_log.len() as int);
                    assert(log_wake[p + 1] == pre_log[p + 1]);
                  } else if p == pre_log.len() as int {
                    assert(!get_io_event(log_wake[p]).writable);
                  } else {
                    assert(!is_io_event_ready_at(log_wake, p));
                  }
                };

                assert forall |k: int| #![auto] old_log.len() as int <= k < log_wake.len() implies
                  !is_succ_register_timer_at(log_wake, k) &&
                  !io_api_registered_at(log_wake, k) &&
                  !is_deregister_timer_at(log_wake, k) &&
                  !io_api_deregistered_at(log_wake, k) &&
                  !is_succ_set_waker_at(log_wake, k) &&
                  !is_park_begin_at(log_wake, k) &&
                  !is_park_end_at(log_wake, k) &&
                  !is_get_current_time_at(log_wake, k) &&
                  !is_poll_events_at(log_wake, k)
                by {
                  if k < pre_log.len() as int {
                    assert(log_wake[k] == pre_log[k]);
                    assert(!is_succ_register_timer_at(pre_log, k));
                    assert(!io_api_registered_at(pre_log, k));
                    assert(!is_deregister_timer_at(pre_log, k));
                    assert(!io_api_deregistered_at(pre_log, k));
                    assert(!is_succ_set_waker_at(pre_log, k));
                    assert(!is_park_begin_at(pre_log, k));
                    assert(!is_park_end_at(pre_log, k));
                    assert(!is_get_current_time_at(pre_log, k));
                    assert(!is_poll_events_at(pre_log, k));
                  }
                };
              }
            }
            None => {
              proof {
                assert(!old_rw.contains_key(rid@));
                assert forall |p: int| #![auto] old_log.len() as int <= p < log_ier.len() &&
                  is_io_event_ready_at(log_ier, p) &&
                  get_io_event(log_ier[p]).readable &&
                  old_rw.contains_key(get_io_event(log_ier[p]).resource_id) implies {
                    let r = get_io_event(log_ier[p]).resource_id;
                    p + 1 < log_ier.len() as int &&
                    is_wake_task_at(log_ier, p + 1) &&
                    get_wake_task_source_rid(log_ier[p + 1]) == r &&
                    get_wake_task_waker(log_ier[p + 1]) == old_rw[r]
                  }
                by {
                  if p < pre_log.len() as int {
                    assert(log_ier[p] == pre_log[p]);
                    assert(is_io_event_ready_at(pre_log, p));
                    assert(get_io_event(pre_log[p]).readable);
                    assert(old_rw.contains_key(get_io_event(pre_log[p]).resource_id));
                    assert(p + 1 < pre_log.len() as int);
                    assert(log_ier[p + 1] == pre_log[p + 1]);
                  } else {
                    assert(p == pre_log.len() as int);
                    assert(!old_rw.contains_key(get_io_event(log_ier[p]).resource_id));
                  }
                };

                assert forall |p: int| #![auto] old_log.len() as int <= p < log_ier.len() &&
                  is_io_event_ready_at(log_ier, p) &&
                  get_io_event(log_ier[p]).writable &&
                  old_ww.contains_key(get_io_event(log_ier[p]).resource_id) implies {
                    let r = get_io_event(log_ier[p]).resource_id;
                    p + 1 < log_ier.len() as int &&
                    is_wake_task_at(log_ier, p + 1) &&
                    get_wake_task_source_rid(log_ier[p + 1]) == r &&
                    get_wake_task_waker(log_ier[p + 1]) == old_ww[r]
                  }
                by {
                  if p < pre_log.len() as int {
                    assert(log_ier[p] == pre_log[p]);
                    assert(is_io_event_ready_at(pre_log, p));
                    assert(get_io_event(pre_log[p]).writable);
                    assert(old_ww.contains_key(get_io_event(pre_log[p]).resource_id));
                    assert(p + 1 < pre_log.len() as int);
                    assert(log_ier[p + 1] == pre_log[p + 1]);
                  } else {
                    assert(p == pre_log.len() as int);
                    assert(!get_io_event(log_ier[p]).writable);
                  }
                };

                assert forall |k: int| #![auto] old_log.len() as int <= k < log_ier.len() implies
                  !is_succ_register_timer_at(log_ier, k) &&
                  !io_api_registered_at(log_ier, k) &&
                  !is_deregister_timer_at(log_ier, k) &&
                  !io_api_deregistered_at(log_ier, k) &&
                  !is_succ_set_waker_at(log_ier, k) &&
                  !is_park_begin_at(log_ier, k) &&
                  !is_park_end_at(log_ier, k) &&
                  !is_get_current_time_at(log_ier, k) &&
                  !is_poll_events_at(log_ier, k)
                by {
                  if k < pre_log.len() as int {
                    assert(log_ier[k] == pre_log[k]);
                    assert(!is_succ_register_timer_at(pre_log, k));
                    assert(!io_api_registered_at(pre_log, k));
                    assert(!is_deregister_timer_at(pre_log, k));
                    assert(!io_api_deregistered_at(pre_log, k));
                    assert(!is_succ_set_waker_at(pre_log, k));
                    assert(!is_park_begin_at(pre_log, k));
                    assert(!is_park_end_at(pre_log, k));
                    assert(!is_get_current_time_at(pre_log, k));
                    assert(!is_poll_events_at(pre_log, k));
                  }
                };
              }
            }
          }
        }
        IoMode::Writable => {
          mark_io_writable(rid);
          let waker_opt = self.resources.v_get_write_waker(rid.0);
          match waker_opt {
            Some(waker) => {
              self.wake_task_action(&waker, rid);
              let ghost log_wake = self.log@;

              proof {
                let e2 = log_wake[log_ier.len() as int];
                assert(log_wake == log_ier.push(e2));
                free_rids_wf_preserved_by_append(self.free_rids@, log_ier, e2, self.resources@, old_next_rid);

                let ghost ev_rid: ResourceIdView = rid@;
                assert(old_ww.contains_key(ev_rid));
                assert(waker@ == old_ww[ev_rid]);
                assert(get_wake_task_waker(log_wake[log_ier.len() as int]) == old_ww[ev_rid]);

                assert(write_wakers_valid(old_ww, log_ier));
                assert(io_currently_active(log_ier, ev_rid));

                io_wake_step_inv(
                  log_ier, e2, old_ts, old_tm, old_tw, old_rw, old_ww, old_next_rid, ev_rid, false,
                );

                let wt_pos = log_ier.len() as int;
                assert(!is_succ_register_timer_at(log_wake, wt_pos));
                assert(!io_api_registered_at(log_wake, wt_pos));
                assert(!is_deregister_timer_at(log_wake, wt_pos));
                assert(!io_api_deregistered_at(log_wake, wt_pos));
                assert(!is_succ_set_waker_at(log_wake, wt_pos));
                assert(!is_park_begin_at(log_wake, wt_pos));
                assert(!is_park_end_at(log_wake, wt_pos));
                assert(!is_get_current_time_at(log_wake, wt_pos));
                assert(!is_poll_events_at(log_wake, wt_pos));
                assert(!is_io_event_ready_at(log_wake, wt_pos));

                assert forall |p: int| #![auto] old_log.len() as int <= p < log_wake.len() &&
                  is_io_event_ready_at(log_wake, p) &&
                  get_io_event(log_wake[p]).writable &&
                  old_ww.contains_key(get_io_event(log_wake[p]).resource_id) implies {
                    let r = get_io_event(log_wake[p]).resource_id;
                    p + 1 < log_wake.len() as int &&
                    is_wake_task_at(log_wake, p + 1) &&
                    get_wake_task_source_rid(log_wake[p + 1]) == r &&
                    get_wake_task_waker(log_wake[p + 1]) == old_ww[r]
                  }
                by {
                  if p < pre_log.len() as int {
                    assert(log_wake[p] == pre_log[p]);
                    assert(is_io_event_ready_at(pre_log, p));
                    assert(get_io_event(pre_log[p]).writable);
                    assert(old_ww.contains_key(get_io_event(pre_log[p]).resource_id));
                    assert(p + 1 < pre_log.len() as int);
                    assert(log_wake[p + 1] == pre_log[p + 1]);
                  } else if p == pre_log.len() as int {
                    assert(p + 1 == log_ier.len() as int);
                    assert(p + 1 < log_wake.len() as int);
                  } else {
                    assert(!is_io_event_ready_at(log_wake, p));
                  }
                };

                assert forall |p: int| #![auto] old_log.len() as int <= p < log_wake.len() &&
                  is_io_event_ready_at(log_wake, p) &&
                  get_io_event(log_wake[p]).readable &&
                  old_rw.contains_key(get_io_event(log_wake[p]).resource_id) implies {
                    let r = get_io_event(log_wake[p]).resource_id;
                    p + 1 < log_wake.len() as int &&
                    is_wake_task_at(log_wake, p + 1) &&
                    get_wake_task_source_rid(log_wake[p + 1]) == r &&
                    get_wake_task_waker(log_wake[p + 1]) == old_rw[r]
                  }
                by {
                  if p < pre_log.len() as int {
                    assert(log_wake[p] == pre_log[p]);
                    assert(is_io_event_ready_at(pre_log, p));
                    assert(get_io_event(pre_log[p]).readable);
                    assert(old_rw.contains_key(get_io_event(pre_log[p]).resource_id));
                    assert(p + 1 < pre_log.len() as int);
                    assert(log_wake[p + 1] == pre_log[p + 1]);
                  } else if p == pre_log.len() as int {
                    assert(!get_io_event(log_wake[p]).readable);
                  } else {
                    assert(!is_io_event_ready_at(log_wake, p));
                  }
                };

                assert forall |k: int| #![auto] old_log.len() as int <= k < log_wake.len() implies
                  !is_succ_register_timer_at(log_wake, k) &&
                  !io_api_registered_at(log_wake, k) &&
                  !is_deregister_timer_at(log_wake, k) &&
                  !io_api_deregistered_at(log_wake, k) &&
                  !is_succ_set_waker_at(log_wake, k) &&
                  !is_park_begin_at(log_wake, k) &&
                  !is_park_end_at(log_wake, k) &&
                  !is_get_current_time_at(log_wake, k) &&
                  !is_poll_events_at(log_wake, k)
                by {
                  if k < pre_log.len() as int {
                    assert(log_wake[k] == pre_log[k]);
                    assert(!is_succ_register_timer_at(pre_log, k));
                    assert(!io_api_registered_at(pre_log, k));
                    assert(!is_deregister_timer_at(pre_log, k));
                    assert(!io_api_deregistered_at(pre_log, k));
                    assert(!is_succ_set_waker_at(pre_log, k));
                    assert(!is_park_begin_at(pre_log, k));
                    assert(!is_park_end_at(pre_log, k));
                    assert(!is_get_current_time_at(pre_log, k));
                    assert(!is_poll_events_at(pre_log, k));
                  }
                };
              }
            }
            None => {
              proof {
                assert(!old_ww.contains_key(rid@));
                assert forall |p: int| #![auto] old_log.len() as int <= p < log_ier.len() &&
                  is_io_event_ready_at(log_ier, p) &&
                  get_io_event(log_ier[p]).writable &&
                  old_ww.contains_key(get_io_event(log_ier[p]).resource_id) implies {
                    let r = get_io_event(log_ier[p]).resource_id;
                    p + 1 < log_ier.len() as int &&
                    is_wake_task_at(log_ier, p + 1) &&
                    get_wake_task_source_rid(log_ier[p + 1]) == r &&
                    get_wake_task_waker(log_ier[p + 1]) == old_ww[r]
                  }
                by {
                  if p < pre_log.len() as int {
                    assert(log_ier[p] == pre_log[p]);
                    assert(is_io_event_ready_at(pre_log, p));
                    assert(get_io_event(pre_log[p]).writable);
                    assert(old_ww.contains_key(get_io_event(pre_log[p]).resource_id));
                    assert(p + 1 < pre_log.len() as int);
                    assert(log_ier[p + 1] == pre_log[p + 1]);
                  } else {
                    assert(p == pre_log.len() as int);
                    assert(!old_ww.contains_key(get_io_event(log_ier[p]).resource_id));
                  }
                };

                assert forall |p: int| #![auto] old_log.len() as int <= p < log_ier.len() &&
                  is_io_event_ready_at(log_ier, p) &&
                  get_io_event(log_ier[p]).readable &&
                  old_rw.contains_key(get_io_event(log_ier[p]).resource_id) implies {
                    let r = get_io_event(log_ier[p]).resource_id;
                    p + 1 < log_ier.len() as int &&
                    is_wake_task_at(log_ier, p + 1) &&
                    get_wake_task_source_rid(log_ier[p + 1]) == r &&
                    get_wake_task_waker(log_ier[p + 1]) == old_rw[r]
                  }
                by {
                  if p < pre_log.len() as int {
                    assert(log_ier[p] == pre_log[p]);
                    assert(is_io_event_ready_at(pre_log, p));
                    assert(get_io_event(pre_log[p]).readable);
                    assert(old_rw.contains_key(get_io_event(pre_log[p]).resource_id));
                    assert(p + 1 < pre_log.len() as int);
                    assert(log_ier[p + 1] == pre_log[p + 1]);
                  } else {
                    assert(p == pre_log.len() as int);
                    assert(!get_io_event(log_ier[p]).readable);
                  }
                };

                assert forall |k: int| #![auto] old_log.len() as int <= k < log_ier.len() implies
                  !is_succ_register_timer_at(log_ier, k) &&
                  !io_api_registered_at(log_ier, k) &&
                  !is_deregister_timer_at(log_ier, k) &&
                  !io_api_deregistered_at(log_ier, k) &&
                  !is_succ_set_waker_at(log_ier, k) &&
                  !is_park_begin_at(log_ier, k) &&
                  !is_park_end_at(log_ier, k) &&
                  !is_get_current_time_at(log_ier, k) &&
                  !is_poll_events_at(log_ier, k)
                by {
                  if k < pre_log.len() as int {
                    assert(log_ier[k] == pre_log[k]);
                    assert(!is_succ_register_timer_at(pre_log, k));
                    assert(!io_api_registered_at(pre_log, k));
                    assert(!is_deregister_timer_at(pre_log, k));
                    assert(!io_api_deregistered_at(pre_log, k));
                    assert(!is_succ_set_waker_at(pre_log, k));
                    assert(!is_park_begin_at(pre_log, k));
                    assert(!is_park_end_at(pre_log, k));
                    assert(!is_get_current_time_at(pre_log, k));
                    assert(!is_poll_events_at(pre_log, k));
                  }
                };
              }
            }
          }
        }
      }
      i += 1;
    }
  }

  #[verifier::rlimit(80)]
  pub fn park(&mut self, timeout: Option<Duration>) -> (result: IoResult<()>)
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
      wheel_slab_consistent(old(self).wheel@, old(self).resources.timer_map_view()),
    ensures
      self.wheel.full_wf(),
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
    let ghost l0 = self.log@;
    let ghost old_ts = self.resources.timer_set_view();
    let ghost old_tm = self.resources.timer_map_view();
    let ghost old_tw = self.resources.timer_wakers_view();
    let ghost old_rw = self.resources.read_wakers_view();
    let ghost old_ww = self.resources.write_wakers_view();
    let ghost old_next_rid = self.next_resource_id as nat;

    proof {
      reactor_inv_split(l0);
    }

    // park_begin
    self.park_begin_action(timeout);
    let ghost log1 = self.log@;
    proof {
      let e = log1[l0.len() as int];
      assert(log1 == l0.push(e));
      data_inv_preserved_by_harmless_event(old_ts, old_tm, old_tw, old_rw, old_ww, l0, e);
      reactor_safety_inv_preserved_by_non_wake_non_register(l0, e);
      reactor_ext_inv_preserved_by_non_trigger(l0, e);
      alloc_inv_preserved_by_non_registration(l0, e, old_next_rid);
      free_rids_wf_preserved_by_append(self.free_rids@, l0, e, self.resources@, old_next_rid);
    }

    // get_current_time
    let now = self.get_current_time_action();
    let ghost log2 = self.log@;
    let ghost gct_pos: int = log1.len() as int;
    proof {
      let e = log2[log1.len() as int];
      assert(log2 == log1.push(e));
      data_inv_preserved_by_harmless_event(old_ts, old_tm, old_tw, old_rw, old_ww, log1, e);
      reactor_safety_inv_preserved_by_non_wake_non_register(log1, e);
      reactor_ext_inv_preserved_by_non_trigger(log1, e);
      alloc_inv_preserved_by_non_registration(log1, e, old_next_rid);
      free_rids_wf_preserved_by_append(self.free_rids@, log1, e, self.resources@, old_next_rid);
    }

    // compute effective timeout
    let effective_timeout = match (timeout, self.next_deadline()) {
      (Some(t), Some(deadline)) => {
        if deadline.inner <= now.inner {
          Some(Duration::from_millis(0))
        } else {
          let timer_ms = (deadline.inner - now.inner) as u64;
          let timer_dur = Duration::from_millis(timer_ms);
          if timer_dur.as_millis() < t.as_millis() {
            Some(timer_dur)
          } else {
            Some(t)
          }
        }
      }
      (None, Some(deadline)) => {
        if deadline.inner <= now.inner {
          Some(Duration::from_millis(0))
        } else {
          let timer_ms = (deadline.inner - now.inner) as u64;
          Some(Duration::from_millis(timer_ms))
        }
      }
      (t, None) => t,
    };

    // poll_events
    let poll_result = self.poll_events_action(effective_timeout);
    let ghost log3 = self.log@;
    proof {
      let e = log3[log2.len() as int];
      assert(log3 == log2.push(e));
      data_inv_preserved_by_harmless_event(old_ts, old_tm, old_tw, old_rw, old_ww, log2, e);
      reactor_safety_inv_preserved_by_non_wake_non_register(log2, e);
      reactor_ext_inv_preserved_by_non_trigger(log2, e);
      alloc_inv_preserved_by_non_registration(log2, e, old_next_rid);
      free_rids_wf_preserved_by_append(self.free_rids@, log2, e, self.resources@, old_next_rid);
    }

    // error path
    let io_events = match poll_result {
      IoResult::Ok(events) => events,
      IoResult::Err(e) => {
        self.wake_expired_timers(now);
        let ghost log_after_wet = self.log@;

        let result: IoResult<()> = IoResult::Err(e);
        self.park_end_action(timeout, &result);
        let ghost log_final = self.log@;

        proof {
          let end_e = log_final[log_after_wet.len() as int];
          assert(log_final == log_after_wet.push(end_e));
          let ts = self.resources.timer_set_view();
          let tm = self.resources.timer_map_view();
          let tw = self.resources.timer_wakers_view();
          let rw = self.resources.read_wakers_view();
          let ww = self.resources.write_wakers_view();
          data_inv_preserved_by_harmless_event(ts, tm, tw, rw, ww, log_after_wet, end_e);
          reactor_safety_inv_preserved_by_non_wake_non_register(log_after_wet, end_e);
          alloc_inv_preserved_by_non_registration(log_after_wet, end_e, old_next_rid);

          // Prove park cycle structure for ext_inv preservation
          // Step 1: current_park_start forward chain
          // log1: park_begin at end → current_park_start = l0.len()
          // log2 = log1.push(gct): non-park, so preserved
          park_start_preserved_by_non_park(log1, log2[log1.len() as int]);
          // log3 = log2.push(poll): non-park, so preserved
          park_start_preserved_by_non_park(log2, log3[log2.len() as int]);
          // log_after_wet: prefix agrees with log3, no park events in new range
          current_park_start_agrees_on_prefix(log_after_wet, log3, log3.len() as int);
          current_park_start_no_park_in_range(log_after_wet, log3.len() as int, log_after_wet.len() as int);
          // log_final = log_after_wet.push(end_e)
          current_park_start_push(log_after_wet, end_e, log_after_wet.len() as int);
          let ghost park_end_pos: int = log_after_wet.len() as int;
          assert(current_park_start(log_final, park_end_pos) == l0.len() as int);

          // Step 2: prove has_get_current_time_in_park
          assert(log_final[gct_pos] == log_after_wet[gct_pos]);
          assert(log_after_wet[gct_pos] == log3[gct_pos]);
          assert(log3[gct_pos] == log2[gct_pos]);
          assert(is_get_current_time_at(log_final, gct_pos));
          assert(gct_pos > l0.len() as int);
          assert(gct_pos < park_end_pos);
          assert(has_get_current_time_in_park(log_final, park_end_pos));

          // Step 3: prove has_exactly_one_poll_events_in_park
          count_poll_events_in_range_push(log_after_wet, end_e, l0.len() as int, park_end_pos);
          count_poll_events_split(log_after_wet, l0.len() as int, log3.len() as int, park_end_pos);
          count_poll_events_zero_when_no_poll(log_after_wet, log3.len() as int, park_end_pos);
          count_poll_events_agrees_on_prefix(log_after_wet, log3, l0.len() as int, log3.len() as int);
          assert(count_poll_events_in_range(log3, l0.len() as int, log3.len() as int) == 1nat) by {
            reveal_with_fuel(count_poll_events_in_range, 4);
          };
          assert(count_poll_events_in_range(log_final, l0.len() as int, park_end_pos) == 1nat);
          assert(has_exactly_one_poll_events_in_park(log_final, park_end_pos));

          reactor_ext_inv_preserved_by_park_end(log_after_wet, end_e);
          free_rids_wf_preserved_by_append(self.free_rids@, log_after_wet, end_e, self.resources@, old_next_rid);

          // Liveness: prove structural properties needed by liveness lemma
          assert forall |k: int| #![auto] l0.len() as int <= k < log_final.len() && k != gct_pos
            implies !is_get_current_time_at(log_final, k) by {
            assert(log_final[k] == log_after_wet.push(end_e)[k]);
            if k < log3.len() as int {
              assert(log_after_wet[k] == log3[k]);
              assert(log_final[k] == log3[k]);
              // k is l0.len() (park_begin), l0.len()+2 (poll_events), or impossible (l0.len()+1 = gct_pos excluded)
              // All are not GetCurrentTime by enum variant disjointness
              assert(!is_get_current_time_at(log3, k));
            } else if k < log_after_wet.len() as int {
              // wake_expired_timers ensures !is_get_current_time_at for new events
              assert(!is_get_current_time_at(log_after_wet, k));
              assert(log_final[k] == log_after_wet[k]);
            } else {
              // k == park_end_pos: park_end event, not GetCurrentTime
            }
          };

          // Help SMT with event variant information
          assert(is_park_begin_at(log_final, l0.len() as int)) by {
            assert(log_final[l0.len() as int] == log3[l0.len() as int]);
            assert(log3[l0.len() as int] == log2[l0.len() as int]);
            assert(log2[l0.len() as int] == log1[l0.len() as int]);
          };
          assert(is_poll_events_at(log_final, l0.len() as int + 2)) by {
            assert(log_final[l0.len() as int + 2] == log3[l0.len() as int + 2]);
          };
          assert(is_park_end_at(log_final, park_end_pos));

          park_error_path_no_registrations(
            log3, log_after_wet, log_final, l0.len() as int,
          );

          liveness_inv_after_park_error_path(
            l0, log_final, gct_pos, now@,
            ts, tm, old_next_rid,
          );
          assert(reactor_inv(log_final)) by {
            assert(reactor_safety_inv(log_final));
            assert(reactor_liveness_inv(log_final));
            reactor_inv_split(log_final);
          }
        }
        return result;
      }
    };

    // normal path: wake_expired_timers
    self.wake_expired_timers(now);
    let ghost log_after_wet = self.log@;
    let ghost rw_snap = self.resources.read_wakers_view();
    let ghost ww_snap = self.resources.write_wakers_view();

    proof {
      park_start_preserved_by_non_park(log1, log2[log1.len() as int]);
      park_start_preserved_by_non_park(log2, log3[log2.len() as int]);
      current_park_start_agrees_on_prefix(log_after_wet, log3, log3.len() as int);
      current_park_start_no_park_in_range(log_after_wet, log3.len() as int, log_after_wet.len() as int);
    }

    // process IO events
    self.process_io_events(&io_events);
    let ghost log_pre_end = self.log@;

    // park_end
    let result = IoResult::Ok(());
    self.park_end_action(timeout, &result);
    let ghost log_final = self.log@;

    proof {
      let end_e = log_final[log_pre_end.len() as int];
      assert(log_final == log_pre_end.push(end_e));
      let ts = self.resources.timer_set_view();
      let tm = self.resources.timer_map_view();
      let tw = self.resources.timer_wakers_view();
      let rw = self.resources.read_wakers_view();
      let ww = self.resources.write_wakers_view();
      data_inv_preserved_by_harmless_event(ts, tm, tw, rw, ww, log_pre_end, end_e);
      reactor_safety_inv_preserved_by_non_wake_non_register(log_pre_end, end_e);
      alloc_inv_preserved_by_non_registration(log_pre_end, end_e, self.next_resource_id as nat);

      // Prove park cycle structure for ext_inv preservation
      // Step 1: forward chain current_park_start
      park_start_preserved_by_non_park(log1, log2[log1.len() as int]);
      park_start_preserved_by_non_park(log2, log3[log2.len() as int]);
      current_park_start_agrees_on_prefix(log_after_wet, log3, log3.len() as int);
      current_park_start_no_park_in_range(log_after_wet, log3.len() as int, log_after_wet.len() as int);
      current_park_start_agrees_on_prefix(log_pre_end, log_after_wet, log_after_wet.len() as int);
      current_park_start_no_park_in_range(log_pre_end, log_after_wet.len() as int, log_pre_end.len() as int);
      current_park_start_push(log_pre_end, end_e, log_pre_end.len() as int);
      let ghost park_end_pos2: int = log_pre_end.len() as int;
      assert(current_park_start(log_final, park_end_pos2) == l0.len() as int);

      // Step 2: prove has_get_current_time_in_park
      assert(log_final[gct_pos] == log_pre_end[gct_pos]);
      assert(log_pre_end[gct_pos] == log_after_wet[gct_pos]);
      assert(log_after_wet[gct_pos] == log3[gct_pos]);
      assert(log3[gct_pos] == log2[gct_pos]);
      assert(is_get_current_time_at(log_final, gct_pos));
      assert(gct_pos > l0.len() as int);
      assert(gct_pos < park_end_pos2);
      assert(has_get_current_time_in_park(log_final, park_end_pos2));

      // Step 3: prove has_exactly_one_poll_events_in_park
      count_poll_events_in_range_push(log_pre_end, end_e, l0.len() as int, park_end_pos2);
      count_poll_events_split(log_pre_end, l0.len() as int, log3.len() as int, park_end_pos2);
      count_poll_events_split(log_pre_end, log3.len() as int, log_after_wet.len() as int, park_end_pos2);
      // For [log3, log_after_wet): use prefix agreement to switch to log_after_wet, then prove zero
      count_poll_events_agrees_on_prefix(log_pre_end, log_after_wet, log3.len() as int, log_after_wet.len() as int);
      count_poll_events_zero_when_no_poll(log_after_wet, log3.len() as int, log_after_wet.len() as int);
      // For [log_after_wet, log_pre_end): process_io_events ensures !is_poll_events_at
      count_poll_events_zero_when_no_poll(log_pre_end, log_after_wet.len() as int, park_end_pos2);
      // For [l0, log3): use prefix agreement to switch to log3
      count_poll_events_agrees_on_prefix(log_pre_end, log3, l0.len() as int, log3.len() as int);
      assert(count_poll_events_in_range(log3, l0.len() as int, log3.len() as int) == 1nat) by {
        reveal_with_fuel(count_poll_events_in_range, 4);
      };
      assert(count_poll_events_in_range(log_final, l0.len() as int, park_end_pos2) == 1nat);
      assert(has_exactly_one_poll_events_in_park(log_final, park_end_pos2));

      reactor_ext_inv_preserved_by_park_end(log_pre_end, end_e);
      free_rids_wf_preserved_by_append(self.free_rids@, log_pre_end, end_e, self.resources@, self.next_resource_id as nat);

      // Liveness: prove structural properties
      assert forall |k: int| #![auto] l0.len() as int <= k < log_final.len() && k != gct_pos
        implies !is_get_current_time_at(log_final, k) by {
        if k < log3.len() as int {
          assert(log_after_wet[k] == log3[k]);
          assert(log_pre_end[k] == log_after_wet[k]);
          assert(log_final[k] == log3[k]);
          assert(!is_get_current_time_at(log3, k));
        } else if k < log_after_wet.len() as int {
          assert(!is_get_current_time_at(log_after_wet, k));
          assert(log_pre_end[k] == log_after_wet[k]);
          assert(log_final[k] == log_after_wet[k]);
        } else if k < log_pre_end.len() as int {
          assert(!is_get_current_time_at(log_pre_end, k));
          assert(log_final[k] == log_pre_end[k]);
        }
      };

      // Help SMT with event variant info
      assert(is_park_begin_at(log_final, l0.len() as int)) by {
        assert(log_final[l0.len() as int] == log_pre_end[l0.len() as int]);
        assert(log_pre_end[l0.len() as int] == log_after_wet[l0.len() as int]);
        assert(log_after_wet[l0.len() as int] == log3[l0.len() as int]);
        assert(log3[l0.len() as int] == log1[l0.len() as int]);
      };
      assert(is_poll_events_at(log_final, l0.len() as int + 2)) by {
        assert(log_final[l0.len() as int + 2] == log_pre_end[l0.len() as int + 2]);
        assert(log_pre_end[l0.len() as int + 2] == log_after_wet[l0.len() as int + 2]);
        assert(log_after_wet[l0.len() as int + 2] == log3[l0.len() as int + 2]);
      };
      let ghost normal_park_end_pos: int = log_pre_end.len() as int;
      assert(is_park_end_at(log_final, normal_park_end_pos));

      park_normal_path_no_registrations(
        log3, log_after_wet, log_pre_end, log_final, l0.len() as int,
      );

      park_io_event_pairing_read(
        log3, log_after_wet, log_pre_end, log_final,
        l0.len() as int, rw_snap,
      );
      park_io_event_pairing_write(
        log3, log_after_wet, log_pre_end, log_final,
        l0.len() as int, ww_snap,
      );

      liveness_inv_after_park_normal_path(
        l0, log_pre_end, log_final,
        gct_pos, now@,
        ts, tm, self.next_resource_id as nat,
        rw_snap, ww_snap,
      );
      assert(reactor_inv(log_final)) by {
        assert(reactor_safety_inv(log_final));
        assert(reactor_liveness_inv(log_final));
        reactor_inv_split(log_final);
      }
    }
    result
  }
}

}
