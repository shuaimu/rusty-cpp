use crate::reactor::Reactor;
use crate::types::{Interest, ResourceId, Waker};
use crate::spec::types::ResourceIdView;
use crate::invariants::*;
use crate::invariants::data_inv::*;
use crate::spec::log::*;
use crate::spec::predicates::*;
use crate::proof::preservation::*;
use crate::proof::safety_preservation::*;
use crate::proof::preservation_ext::*;
use vstd::prelude::*;

verus! {

proof fn lift_io_currently_active_to_set_waker(
  old_log: Log,
  log_mid: Log,
  log_final: Log,
  rid: ResourceIdView,
)
  requires
    io_currently_active(old_log, rid),
    log_mid.len() == old_log.len() + 1,
    old_log =~= log_mid.subrange(0, old_log.len() as int),
    log_final.len() == log_mid.len() + 1,
    log_mid =~= log_final.subrange(0, log_mid.len() as int),
    !io_api_deregistered_at(log_mid, old_log.len() as int),
    !io_api_deregistered_at(log_final, log_mid.len() as int),
  ensures
    io_api_active_at_set_waker(log_final, rid, log_mid.len() as int),
    io_currently_active(log_mid, rid),
    io_currently_active(log_final, rid),
{
  let reg_idx = choose |reg_idx: int| 0 <= reg_idx < old_log.len() &&
    io_api_registered_at(old_log, reg_idx) &&
    get_io_api_register_rid(old_log[reg_idx]) == rid &&
    io_api_active_at(old_log, reg_idx, old_log.len() as int);

  assert(log_mid[reg_idx] == old_log[reg_idx]);
  assert(io_api_registered_at(log_mid, reg_idx));
  assert(get_io_api_register_rid(log_mid[reg_idx]) == rid);
  assert forall |k: int| reg_idx < k < log_mid.len() implies !(
    io_api_deregistered_at(log_mid, k) &&
    get_io_api_deregister_rid(log_mid[k]) == get_io_api_register_rid(log_mid[reg_idx])
  ) by {
    if k < old_log.len() as int {
      assert(log_mid[k] == old_log[k]);
      assert(!(io_api_deregistered_at(old_log, k) &&
        get_io_api_deregister_rid(old_log[k]) == get_io_api_register_rid(old_log[reg_idx])));
    }
  }
  assert(io_api_active_at(log_mid, reg_idx, log_mid.len() as int));
  assert(io_currently_active(log_mid, rid));

  assert(log_final[reg_idx] == old_log[reg_idx]);
  assert(io_api_registered_at(log_final, reg_idx));
  assert(get_io_api_register_rid(log_final[reg_idx]) == rid);
  assert forall |k: int| reg_idx < k < log_final.len() as int implies !(
    io_api_deregistered_at(log_final, k) &&
    get_io_api_deregister_rid(log_final[k]) == get_io_api_register_rid(log_final[reg_idx])
  ) by {
    if k < old_log.len() as int {
      assert(log_final[k] == old_log[k]);
      assert(!(io_api_deregistered_at(old_log, k) &&
        get_io_api_deregister_rid(old_log[k]) == get_io_api_register_rid(old_log[reg_idx])));
    } else if k < log_mid.len() as int {
      assert(log_final[k] == log_mid[k]);
    }
  }
  assert(io_api_active_at(log_final, reg_idx, log_final.len() as int));
  assert(io_currently_active(log_final, rid));
  assert(io_api_active_at(log_final, reg_idx, log_mid.len() as int));
  assert(io_api_active_at_set_waker(log_final, rid, log_mid.len() as int));
}

impl Reactor {
  pub fn set_waker(&mut self, resource_id: ResourceId, interest: Interest, waker: Waker)
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
      io_currently_active(old(self).log@, resource_id@),
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
    let ghost old_rw = self.resources.read_wakers_view();
    let ghost old_ww = self.resources.write_wakers_view();
    let ghost old_tw = self.resources.timer_wakers_view();
    let ghost old_ts = self.resources.timer_set_view();
    let ghost old_tm = self.resources.timer_map_view();
    let ghost rid = resource_id@;
    let ghost waker_val = waker@;
    let ghost readable = interest.readable;
    let ghost writable = interest.writable;

    self.set_waker_begin_action(resource_id, interest, &waker);

    let ghost log_mid = self.log@;
    let ghost mid_resources = self.resources@;

    proof {
      let begin_event = log_mid[old_log.len() as int];
      assert(log_mid == old_log.push(begin_event));
      data_inv_preserved_by_harmless_event(old_ts, old_tm, old_tw, old_rw, old_ww, old_log, begin_event);
      reactor_inv_preserved_by_non_trigger(old_log, begin_event);
      reactor_ext_inv_preserved_by_non_trigger(old_log, begin_event);
      alloc_inv_preserved_by_non_registration(old_log, begin_event, self.next_resource_id as nat);
      free_rids_wf_preserved_by_append(old(self).free_rids@, old_log, begin_event, mid_resources, self.next_resource_id as nat);
    }

    if interest.readable {
      self.resources.v_set_read_waker(resource_id.0, waker.clone());
    }
    if interest.writable {
      self.resources.v_set_write_waker(resource_id.0, waker.clone());
    }

    let ghost new_rw = self.resources.read_wakers_view();
    let ghost new_ww = self.resources.write_wakers_view();

    proof {
      assert(self.resources@.dom() =~= mid_resources.dom());
      free_rids_wf_preserved_by_same_domain(old(self).free_rids@, log_mid, mid_resources, self.resources@, self.next_resource_id as nat);
    }

    self.set_waker_end_action(resource_id, interest, &waker);

    let ghost log_final = self.log@;

    proof {
      let end_event = log_final[log_mid.len() as int];
      assert(log_final == log_mid.push(end_event));

      lift_io_currently_active_to_set_waker(old_log, log_mid, log_final, rid);

      reactor_inv_preserved_by_non_trigger(log_mid, end_event);
      reactor_ext_inv_preserved_by_succ_set_waker(log_mid, end_event);
      alloc_inv_preserved_by_non_registration(log_mid, end_event, self.next_resource_id as nat);
      free_rids_wf_preserved_by_append(old(self).free_rids@, log_mid, end_event, self.resources@, self.next_resource_id as nat);

      timer_heap_entries_valid_preserved_by_non_timer_event(old_ts, log_mid, end_event);
      active_timers_in_heap_preserved_by_non_timer_event(old_tm, log_mid, end_event);
      timer_wakers_match_preserved_by_append(old_tw, old_tm, log_mid, end_event);

      read_wakers_valid_after_set_waker_event(old_rw, log_mid, end_event, rid, waker_val, readable);
      write_wakers_valid_after_set_waker_event(old_ww, log_mid, end_event, rid, waker_val, writable);
      read_wakers_complete_after_set_waker_event(old_rw, log_mid, end_event, rid, readable);
      write_wakers_complete_after_set_waker_event(old_ww, log_mid, end_event, rid, writable);

      assert(new_rw == if readable { old_rw.insert(rid, waker_val) } else { old_rw });
      assert(new_ww == if writable { old_ww.insert(rid, waker_val) } else { old_ww });
    }
  }
}

}
