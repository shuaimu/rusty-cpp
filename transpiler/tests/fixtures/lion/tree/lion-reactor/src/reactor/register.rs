use crate::reactor::Reactor;
use crate::types::{Interest, IoResult, ResourceId, Source};
use crate::invariants::*;
use crate::invariants::data_inv::*;
use crate::spec::log::*;
use crate::spec::predicates::*;
use crate::spec::types::IoResultView;
use crate::proof::preservation::*;
use crate::proof::safety_preservation::*;
use crate::proof::preservation_ext::*;
use crate::invariants::register_io_in_cycle::*;
use crate::invariants::deregister_io_in_cycle::*;
use crate::invariants::inbound_register_io_result::*;
use crate::invariants::inbound_deregister_io_result::*;
use vstd::prelude::*;

verus! {

impl Reactor {
  pub fn register_io_resource(
    &mut self,
    source: &mut Source,
    interest: Interest,
  ) -> (result: IoResult<ResourceId>)
    requires
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

    self.register_io_begin_action(source, interest);

    let ghost log1 = self.log@;

    proof {
      let e1 = log1[old_log.len() as int];
      assert(log1 == old_log.push(e1));
      data_inv_preserved_by_harmless_event(old_ts, old_tm, old_tw, old_rw, old_ww, old_log, e1);
      reactor_inv_preserved_by_non_trigger(old_log, e1);
      reactor_ext_inv_preserved_by_non_trigger(old_log, e1);
      alloc_inv_preserved_by_non_registration(old_log, e1, old_next_rid);
      free_rids_wf_preserved_by_append(old(self).free_rids@, old_log, e1, self.resources@, old_next_rid);
    }

    let alloc_result = self.alloc_resource_id();
    let ghost new_next_rid = self.next_resource_id as nat;

    let result = match alloc_result {
      IoResult::Ok(resource_id) => {
        let ghost rid = resource_id@;

        let os_result = self.register_io_source_action(source, resource_id, interest);

        let ghost log2 = self.log@;

        proof {
          let e2 = log2[log1.len() as int];
          assert(log2 == log1.push(e2));
          data_inv_preserved_by_harmless_event(old_ts, old_tm, old_tw, old_rw, old_ww, log1, e2);
          reactor_inv_preserved_by_non_trigger(log1, e2);

          assert(is_inbound_register_io_begin_at(log2, old_log.len() as int));
          assert(in_register_io_cycle(log2, log1.len() as int));
          reactor_ext_inv_preserved_by_outbound_register_io(log1, e2);

          alloc_inv_preserved_by_non_registration(log1, e2, new_next_rid);
          free_rids_wf_preserved_by_append(self.free_rids@, log1, e2, self.resources@, new_next_rid);
        }

        match os_result {
          IoResult::Ok(()) => {
            proof {
              assert(!self.resources@.contains_key(rid));
            }
            self.resources.v_insert_io_slot(resource_id.0);
            proof {
              free_rids_wf_preserved_by_resource_insert(self.free_rids@, log2, old(self).resources@, new_next_rid, rid, self.resources@[rid]);
            }
            IoResult::Ok(resource_id)
          }
          IoResult::Err(e) => {
            IoResult::Err(e)
          }
        }
      }
      IoResult::Err(e) => IoResult::Err(e),
    };

    let ghost pre_end_log = self.log@;
    let ghost pre_end_free_rids = self.free_rids@;
    let ghost pre_end_resources = self.resources@;

    self.register_io_end_action(source, interest, &result);

    let ghost log_final = self.log@;

    proof {
      let end_event = log_final[pre_end_log.len() as int];
      assert(log_final == pre_end_log.push(end_event));

      match result {
        IoResult::Ok(resource_id) => {
          let rid = resource_id@;

          assert(pre_end_log.len() == log1.len() + 1);

          assert forall |k: int| 0 <= k < pre_end_log.len() as int
            && io_api_registered_at(pre_end_log, k) && get_io_api_register_rid(pre_end_log[k]) == rid implies
            exists |j: int| k < j < pre_end_log.len() as int
              && io_api_deregistered_at(pre_end_log, j) && get_io_api_deregister_rid(pre_end_log[j]) == rid
          by {
            assert(k < log1.len() as int);
            assert(pre_end_log[k] == log1[k]);
            assert(io_api_registered_at(log1, k));
            let j = choose |j: int| k < j < log1.len() as int
              && io_api_deregistered_at(log1, j) && get_io_api_deregister_rid(log1[j]) == rid;
            assert(pre_end_log[j] == log1[j]);
            assert(io_api_deregistered_at(pre_end_log, j));
            assert(get_io_api_deregister_rid(pre_end_log[j]) == rid);
          }

          let e2 = pre_end_log[log1.len() as int];
          assert(pre_end_log == log1.push(e2));

          assert forall |k: int| 0 <= k < pre_end_log.len() as int
            && is_succ_register_timer_at(pre_end_log, k) && get_register_timer_rid(pre_end_log[k]) == rid implies
            exists |j: int| k < j < pre_end_log.len() as int
              && timer_retired_at(pre_end_log, rid, j)
          by {
            assert(k < log1.len() as int);
            assert(pre_end_log[k] == log1[k]);
            assert(is_succ_register_timer_at(log1, k));
            let j = choose |j: int| k < j < log1.len() as int
              && timer_retired_at(log1, rid, j);
            timer_retired_preserved(log1, e2, rid, j);
          }

          reactor_inv_preserved_by_succ_register_io(pre_end_log, end_event);

          reveal_with_fuel(find_register_io_cycle_begin, 3);

          let outbound_idx = (old_log.len() + 1) as int;
          assert(io_syscall_register_at(log_final, outbound_idx));
          assert(get_outbound_register_io_source(log_final[outbound_idx]) == source@);
          assert(get_outbound_register_io_interest(log_final[outbound_idx]) == interest@);
          assert(inbound_result_matches_outbound_register(
            IoResultView::Ok(rid),
            get_outbound_register_io_result(log_final[outbound_idx]),
            get_io_syscall_register_rid(log_final[outbound_idx]),
          ));
          assert(has_matching_outbound_register(
            log_final, old_log.len() as int, pre_end_log.len() as int,
            source@, interest@, IoResultView::Ok(rid),
          ));
          assert(register_io_result_valid(log_final, pre_end_log.len() as int));

          reactor_ext_inv_preserved_by_succ_register_io(pre_end_log, end_event);

          alloc_inv_preserved_by_registration(pre_end_log, end_event, new_next_rid);

          free_rids_wf_preserved_by_register_io(pre_end_free_rids, pre_end_log, end_event, pre_end_resources, new_next_rid, rid);

          data_inv_preserved_by_fresh_register_io(
            old_ts, old_tm, old_tw, old_rw, old_ww,
            pre_end_log, end_event, rid,
          );
        }
        IoResult::Err(_) => {
          data_inv_preserved_by_harmless_event(old_ts, old_tm, old_tw, old_rw, old_ww, pre_end_log, end_event);
          reactor_inv_preserved_by_non_trigger(pre_end_log, end_event);
          reactor_ext_inv_preserved_by_non_trigger(pre_end_log, end_event);
          alloc_inv_preserved_by_non_registration(pre_end_log, end_event, new_next_rid);
          free_rids_wf_preserved_by_append(pre_end_free_rids, pre_end_log, end_event, pre_end_resources, new_next_rid);
        }
      }
    }
    result
  }

  pub fn deregister_io_resource(
    &mut self,
    resource_id: ResourceId,
    source: &mut Source,
  ) -> (result: IoResult<()>)
    requires
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
      old(self).resources@.contains_key(resource_id@),
      old(self).resources@[resource_id@].is_io(),
      wheel_slab_consistent(old(self).wheel@, old(self).resources.timer_map_view()),
    ensures
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
    let ghost rid = resource_id@;

    self.deregister_io_begin_action(resource_id);

    let ghost log1 = self.log@;

    proof {
      let e1 = log1[old_log.len() as int];
      assert(log1 == old_log.push(e1));
      reactor_inv_preserved_by_non_trigger(old_log, e1);
      reactor_ext_inv_preserved_by_non_trigger(old_log, e1);
      alloc_inv_preserved_by_non_registration(old_log, e1, old_next_rid);
    }

    let ghost pre_dereg_free_rids = self.free_rids@;
    let ghost pre_dereg_resources = self.resources@;

    self.resources.v_remove_io_slot(resource_id.0);
    // [RID-REUSE DISABLED — restore when generational ids are added]
    // self.free_rids.push(resource_id.0);

    let ghost new_rw = self.resources.read_wakers_view();
    let ghost new_ww = self.resources.write_wakers_view();

    proof {
      assert(new_rw == old_rw.remove(rid));
      assert(new_ww == old_ww.remove(rid));

      let e1 = log1[old_log.len() as int];
      io_not_active_after_deregister(old_log, e1, rid);

      timer_heap_entries_valid_preserved_by_non_timer_event(old_ts, old_log, e1);
      active_timers_in_heap_preserved_by_non_timer_event(old_tm, old_log, e1);
      timer_wakers_match_preserved_by_append(old_tw, old_tm, old_log, e1);
      read_wakers_valid_after_deregister_io_and_remove(old_rw, old_log, e1, rid);
      write_wakers_valid_after_deregister_io_and_remove(old_ww, old_log, e1, rid);
      read_wakers_complete_preserved_by_non_trigger(old_rw, old_log, e1);
      read_wakers_complete_remove_deregistered(old_rw, log1, rid);
      write_wakers_complete_preserved_by_non_trigger(old_ww, old_log, e1);
      write_wakers_complete_remove_deregistered(old_ww, log1, rid);

      assert(data_inv(old_ts, old_tm, old_tw, new_rw, new_ww, log1));

      if pre_dereg_resources.contains_key(rid) {
        if old_tm.contains_key(rid) {
          self.resources.timer_map_key_is_timer(rid);
          assert(false);
        }
        assert(!old_tm.contains_key(rid));

        assert forall |k: int| 0 <= k < old_log.len() && is_succ_register_timer_at(old_log, k) && get_register_timer_rid(old_log[k]) == rid implies
          exists |j: int| k < j < old_log.len() && timer_retired_at(old_log, rid, j)
        by {
          assert(active_timers_in_heap(old_tm, old_log));
          if timer_active_at(old_log, k, old_log.len() as int) {
            assert forall |k2: int| k < k2 < old_log.len() implies
              !(is_wake_task_at(old_log, k2) && get_wake_task_source_rid(old_log[k2]) == rid)
            by {
              if is_wake_task_at(old_log, k2) && get_wake_task_source_rid(old_log[k2]) == rid {
                timer_retired_from_wake(old_log, rid, k2);
                assert(timer_retired_at(old_log, rid, k2));
                assert(false);
              }
            };
            assert(timer_awaiting_wake(old_log, k));
            assert(old_tm.contains_key(rid));
            assert(false);
          }
        }

        // [RID-REUSE DISABLED — restore when generational ids are added]
        // Push removed: free_rids stays unchanged. Re-derive free_rids_wf for the
        // UNCHANGED seq across the begin-event append + io-slot removal (the first two
        // steps the push lemma performed internally, minus the final push).
        free_rids_wf_preserved_by_append(
          pre_dereg_free_rids, old_log, e1, pre_dereg_resources, old_next_rid,
        );
        free_rids_wf_preserved_by_resource_remove(
          pre_dereg_free_rids, log1, pre_dereg_resources, old_next_rid, rid,
        );
      } else {
        assert(false);
      }
    }

    let result = self.deregister_io_source_action(source, resource_id);

    let ghost log2 = self.log@;

    proof {
      let e2 = log2[log1.len() as int];
      assert(log2 == log1.push(e2));
      data_inv_preserved_by_harmless_event(old_ts, old_tm, old_tw, new_rw, new_ww, log1, e2);
      reactor_inv_preserved_by_non_trigger(log1, e2);

      assert(is_inbound_deregister_io_begin_at(log2, old_log.len() as int));
      assert(in_deregister_io_cycle(log2, log1.len() as int));
      reactor_ext_inv_preserved_by_outbound_deregister_io(log1, e2);

      alloc_inv_preserved_by_non_registration(log1, e2, old_next_rid);
      free_rids_wf_preserved_by_append(self.free_rids@, log1, e2, self.resources@, old_next_rid);
    }

    self.deregister_io_end_action(resource_id, &result);

    let ghost log_final = self.log@;

    proof {
      let end_event = log_final[log2.len() as int];
      assert(log_final == log2.push(end_event));

      timer_heap_entries_valid_preserved_by_non_timer_event(old_ts, log2, end_event);
      active_timers_in_heap_preserved_by_non_timer_event(old_tm, log2, end_event);
      timer_wakers_match_preserved_by_append(old_tw, old_tm, log2, end_event);
      read_wakers_valid_after_deregister_io_and_remove(new_rw, log2, end_event, rid);
      assert(new_rw.remove(rid) =~= new_rw);
      write_wakers_valid_after_deregister_io_and_remove(new_ww, log2, end_event, rid);
      assert(new_ww.remove(rid) =~= new_ww);
      read_wakers_complete_preserved_by_non_trigger(new_rw, log2, end_event);
      write_wakers_complete_preserved_by_non_trigger(new_ww, log2, end_event);
      reactor_inv_preserved_by_non_trigger(log2, end_event);

      reveal_with_fuel(find_deregister_io_cycle_begin, 3);

      let outbound_idx = (old_log.len() + 1) as int;
      assert(io_syscall_deregistered_at(log_final, outbound_idx));
      assert(get_io_syscall_deregister_rid(log_final[outbound_idx]) == rid);
      assert(get_outbound_deregister_io_result(log_final[outbound_idx]) == result@);
      assert(has_matching_outbound_deregister(
        log_final, old_log.len() as int, log2.len() as int,
        rid, result@,
      ));
      assert(deregister_io_result_valid(log_final, log2.len() as int));

      reactor_ext_inv_preserved_by_inbound_deregister_io_end(log2, end_event);

      alloc_inv_preserved_by_non_registration(log2, end_event, old_next_rid);
      free_rids_wf_preserved_by_append(self.free_rids@, log2, end_event, self.resources@, old_next_rid);
    }
    result
  }
}

}
