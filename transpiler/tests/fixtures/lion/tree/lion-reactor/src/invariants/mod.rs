pub mod timer_waker_validity;
pub mod io_waker_validity;
pub mod timer_reg_uniqueness;
pub mod io_reg_uniqueness;
pub mod timer_io_disjoint;
pub mod wake_has_registration;
pub mod wake_on_expired;
pub mod wake_on_io_ready;
pub mod data_inv;
pub mod timer_deadline_future;
pub mod park_has_timestamp;
pub mod park_poll_once;
pub mod io_ready_in_park;
pub mod register_io_in_cycle;
pub mod deregister_io_in_cycle;
pub mod inbound_register_io_result;
pub mod inbound_deregister_io_result;
pub mod set_waker_active_io;

use vstd::prelude::*;
use crate::spec::log::*;
use crate::spec::predicates::*;

verus! {

pub open spec fn reactor_inv(l: Log) -> bool {
  // R7: timer_reg_uniqueness
  &&& forall |i: int| #![auto] is_succ_register_timer_at(l, i) ==> {
    let rid = get_register_timer_rid(l[i]);
    no_prior_timer_registration(l, rid, i)
  }
  // R8: io_reg_uniqueness
  &&& forall |i: int| #![auto] io_api_registered_at(l, i) ==> {
    let rid = get_io_api_register_rid(l[i]);
    no_prior_io_api_registration(l, rid, i)
  }
  // R9a: timer_io_disjoint_at_timer
  &&& forall |i: int| #![auto] is_succ_register_timer_at(l, i) ==> {
    let rid = get_register_timer_rid(l[i]);
    no_io_api_with_rid_before(l, rid, i)
  }
  // R9b: timer_io_disjoint_at_io
  &&& forall |i: int| #![auto] io_api_registered_at(l, i) ==> {
    let rid = get_io_api_register_rid(l[i]);
    no_timer_with_rid_before(l, rid, i)
  }
  // R5: timer_waker_validity
  &&& forall |i: int| #![auto] timer_waker_validity::is_timer_wake_at(l, i) ==> {
    let rid = get_wake_task_source_rid(l[i]);
    let waker = get_wake_task_waker(l[i]);
    exists |j: int| 0 <= j < i &&
      is_succ_register_timer_at(l, j) &&
      get_register_timer_rid(l[j]) == rid &&
      get_register_timer_waker(l[j]) == waker &&
      timer_active_at(l, j, i)
  }
  // R6: io_waker_validity
  &&& forall |i: int| #![auto] io_waker_validity::is_io_api_wake_at(l, i) ==> {
    let rid = get_wake_task_source_rid(l[i]);
    let waker = get_wake_task_waker(l[i]);
    exists |sw_idx: int| 0 <= sw_idx < i &&
      is_succ_set_waker_at(l, sw_idx) &&
      get_set_waker_rid(l[sw_idx]) == rid &&
      get_set_waker_waker(l[sw_idx]) == waker &&
      io_api_active_at_set_waker(l, rid, sw_idx)
  }
  // R14: wake_has_registration
  &&& forall |i: int| #![auto] is_wake_task_at(l, i) ==> {
    let rid = get_wake_task_source_rid(l[i]);
    (exists |j: int| 0 <= j < i &&
      is_succ_register_timer_at(l, j) &&
      get_register_timer_rid(l[j]) == rid)
    ||
    (exists |j: int| 0 <= j < i &&
      io_api_registered_at(l, j) &&
      get_io_api_register_rid(l[j]) == rid)
  }
  // R16: wake_on_expired (weakened: only for timers still awaiting wake).
  // BRIDGE: same-park-cycle delivery is nevertheless real — see the BRIDGE note on the
  // reactor_liveness_inv copy of this comment below, and the header of
  // lion-reactor-spec/src/invariants/wake_on_expired.rs.
  // DEGENERATE / SAFETY-ONLY (not a liveness guarantee): the antecedent's
  // `data_inv::timer_awaiting_wake(l, i)` requires that NO wake for this rid occurs after i,
  // which directly contradicts the `exists j > i. is_wake_task_at(l, j) && rid matches`
  // consequent (is_wake_task_at forces j < l.len()). So the consequent is UNSATISFIABLE
  // whenever the antecedent holds, and R16 collapses to the SAFETY invariant "no expired
  // timer is ever left awaiting a wake". That is satisfiable, meaningful, and neither unsound
  // nor vacuous — but the `exists j (wake) ... !is_park_end_between` machinery is effectively
  // DEAD: R16 does NOT assert that any wake actually fires. If a genuine liveness guarantee
  // was intended, R16 as written does not provide it. (Contrast R17a/R17b below: their
  // antecedents do NOT forbid future wakes, so their `exists j. wake` is real and non-degenerate.)
  &&& forall |i: int| #![auto]
    (is_succ_register_timer_at(l, i) && has_timeout_point(l, i) && data_inv::timer_awaiting_wake(l, i)) ==>
    exists |j: int| j > i &&
      is_wake_task_at(l, j) &&
      get_wake_task_source_rid(l[j]) == get_register_timer_rid(l[i]) &&
      get_wake_task_waker(l[j]) == get_register_timer_waker(l[i]) && {
      let timeout_idx = first_timeout_point(l, i);
      j > timeout_idx &&
      forall |k: int| timeout_idx < k < j ==> !is_park_end_at(l, k)
    }
  // R17a: wake_on_io_ready_readable
  &&& forall |i: int| #![auto]
    (is_io_event_ready_at(l, i) && has_valid_set_waker_readable_api(l, i)) ==>
    exists |j: int| #![trigger is_wake_task_at(l, j)] j > i && {
      let event = get_io_event(l[i]);
      let rid = event.resource_id;
      let sw_idx = find_last_set_waker_for_rid_readable(l, rid, i);
      let waker = get_set_waker_waker(l[sw_idx]);
      is_wake_task_at(l, j) &&
      get_wake_task_source_rid(l[j]) == rid &&
      get_wake_task_waker(l[j]) == waker &&
      j > i &&
      forall |k: int| i < k < j ==> !is_park_end_at(l, k) && !is_poll_events_at(l, k)
    }
  // R17b: wake_on_io_ready_writable
  &&& forall |i: int| #![auto]
    (is_io_event_ready_at(l, i) && has_valid_set_waker_writable_api(l, i)) ==>
    exists |j: int| #![trigger is_wake_task_at(l, j)] j > i && {
      let event = get_io_event(l[i]);
      let rid = event.resource_id;
      let sw_idx = find_last_set_waker_for_rid_writable(l, rid, i);
      let waker = get_set_waker_waker(l[sw_idx]);
      is_wake_task_at(l, j) &&
      get_wake_task_source_rid(l[j]) == rid &&
      get_wake_task_waker(l[j]) == waker &&
      j > i &&
      forall |k: int| i < k < j ==> !is_park_end_at(l, k) && !is_poll_events_at(l, k)
    }
}

pub open spec fn reactor_safety_inv(l: Log) -> bool {
  // R7: timer_reg_uniqueness
  &&& forall |i: int| #![auto] is_succ_register_timer_at(l, i) ==> {
    let rid = get_register_timer_rid(l[i]);
    no_prior_timer_registration(l, rid, i)
  }
  // R8: io_reg_uniqueness
  &&& forall |i: int| #![auto] io_api_registered_at(l, i) ==> {
    let rid = get_io_api_register_rid(l[i]);
    no_prior_io_api_registration(l, rid, i)
  }
  // R9a: timer_io_disjoint_at_timer
  &&& forall |i: int| #![auto] is_succ_register_timer_at(l, i) ==> {
    let rid = get_register_timer_rid(l[i]);
    no_io_api_with_rid_before(l, rid, i)
  }
  // R9b: timer_io_disjoint_at_io
  &&& forall |i: int| #![auto] io_api_registered_at(l, i) ==> {
    let rid = get_io_api_register_rid(l[i]);
    no_timer_with_rid_before(l, rid, i)
  }
  // R5: timer_waker_validity
  &&& forall |i: int| #![auto] timer_waker_validity::is_timer_wake_at(l, i) ==> {
    let rid = get_wake_task_source_rid(l[i]);
    let waker = get_wake_task_waker(l[i]);
    exists |j: int| 0 <= j < i &&
      is_succ_register_timer_at(l, j) &&
      get_register_timer_rid(l[j]) == rid &&
      get_register_timer_waker(l[j]) == waker &&
      timer_active_at(l, j, i)
  }
  // R6: io_waker_validity
  &&& forall |i: int| #![auto] io_waker_validity::is_io_api_wake_at(l, i) ==> {
    let rid = get_wake_task_source_rid(l[i]);
    let waker = get_wake_task_waker(l[i]);
    exists |sw_idx: int| 0 <= sw_idx < i &&
      is_succ_set_waker_at(l, sw_idx) &&
      get_set_waker_rid(l[sw_idx]) == rid &&
      get_set_waker_waker(l[sw_idx]) == waker &&
      io_api_active_at_set_waker(l, rid, sw_idx)
  }
  // R14: wake_has_registration
  &&& forall |i: int| #![auto] is_wake_task_at(l, i) ==> {
    let rid = get_wake_task_source_rid(l[i]);
    (exists |j: int| 0 <= j < i &&
      is_succ_register_timer_at(l, j) &&
      get_register_timer_rid(l[j]) == rid)
    ||
    (exists |j: int| 0 <= j < i &&
      io_api_registered_at(l, j) &&
      get_io_api_register_rid(l[j]) == rid)
  }
}

pub open spec fn reactor_liveness_inv(l: Log) -> bool {
  // R16: wake_on_expired (weakened: only for timers still awaiting wake).
  //
  // BRIDGE (why "an expired timer is woken in the same park cycle" is nevertheless real,
  // despite the DEGENERATE warning below): reactor_inv is only re-established at
  // park-cycle boundaries — park() proves reactor_inv(log_final) as a whole-cycle
  // postcondition, so mid-cycle logs never owe this invariant. A GetCurrentTime >=
  // deadline observed while the timer is active is a permanent timeout point; at the
  // first boundary after it, the collapsed safety form forces a WakeTask (or a
  // DeregisterTimer) to have landed between that clock read and the cycle's end — i.e.
  // within the same park cycle. The timing thus comes from the checkpoint granularity,
  // NOT from the dead `!is_park_end` clause (and would evaporate if preservation ever
  // moved to per-event appends). Consumption: lion-liveness's timeout_triggers_wake_lemma
  // takes not-deregistered as a side condition (the task-keeps-its-resource assumption)
  // and extracts "a wake exists" by contradiction with this antecedent's no-wake clause;
  // the timer contract's round bound (K) comes from the env-form clock path, never from
  // here. Full discussion: header of lion-reactor-spec/src/invariants/wake_on_expired.rs.
  // DEGENERATE / SAFETY-ONLY (not a liveness guarantee): the antecedent's
  // `data_inv::timer_awaiting_wake(l, i)` requires that NO wake for this rid occurs after i,
  // which directly contradicts the `exists j > i. is_wake_task_at(l, j) && rid matches`
  // consequent (is_wake_task_at forces j < l.len()). So the consequent is UNSATISFIABLE
  // whenever the antecedent holds, and R16 collapses to the SAFETY invariant "no expired
  // timer is ever left awaiting a wake". That is satisfiable, meaningful, and neither unsound
  // nor vacuous — but the `exists j (wake) ... !is_park_end_between` machinery is effectively
  // DEAD: R16 does NOT assert that any wake actually fires. If a genuine liveness guarantee
  // was intended, R16 as written does not provide it. (Contrast R17a/R17b below: their
  // antecedents do NOT forbid future wakes, so their `exists j. wake` is real and non-degenerate.)
  &&& forall |i: int| #![auto]
    (is_succ_register_timer_at(l, i) && has_timeout_point(l, i) && data_inv::timer_awaiting_wake(l, i)) ==>
    exists |j: int| j > i &&
      is_wake_task_at(l, j) &&
      get_wake_task_source_rid(l[j]) == get_register_timer_rid(l[i]) &&
      get_wake_task_waker(l[j]) == get_register_timer_waker(l[i]) && {
      let timeout_idx = first_timeout_point(l, i);
      j > timeout_idx &&
      forall |k: int| timeout_idx < k < j ==> !is_park_end_at(l, k)
    }
  // R17a: wake_on_io_ready_readable
  &&& forall |i: int| #![auto]
    (is_io_event_ready_at(l, i) && has_valid_set_waker_readable_api(l, i)) ==>
    exists |j: int| #![trigger is_wake_task_at(l, j)] j > i && {
      let event = get_io_event(l[i]);
      let rid = event.resource_id;
      let sw_idx = find_last_set_waker_for_rid_readable(l, rid, i);
      let waker = get_set_waker_waker(l[sw_idx]);
      is_wake_task_at(l, j) &&
      get_wake_task_source_rid(l[j]) == rid &&
      get_wake_task_waker(l[j]) == waker &&
      j > i &&
      forall |k: int| i < k < j ==> !is_park_end_at(l, k) && !is_poll_events_at(l, k)
    }
  // R17b: wake_on_io_ready_writable
  &&& forall |i: int| #![auto]
    (is_io_event_ready_at(l, i) && has_valid_set_waker_writable_api(l, i)) ==>
    exists |j: int| #![trigger is_wake_task_at(l, j)] j > i && {
      let event = get_io_event(l[i]);
      let rid = event.resource_id;
      let sw_idx = find_last_set_waker_for_rid_writable(l, rid, i);
      let waker = get_set_waker_waker(l[sw_idx]);
      is_wake_task_at(l, j) &&
      get_wake_task_source_rid(l[j]) == rid &&
      get_wake_task_waker(l[j]) == waker &&
      j > i &&
      forall |k: int| i < k < j ==> !is_park_end_at(l, k) && !is_poll_events_at(l, k)
    }
}

pub proof fn reactor_inv_split(l: Log)
  ensures
    reactor_inv(l) <==> (reactor_safety_inv(l) && reactor_liveness_inv(l)),
{}

pub open spec fn reactor_ext_inv(l: Log) -> bool {
  // R1: timer_deadline_future
  &&& forall |i: int| #![auto] is_succ_register_timer_at(l, i) ==>
    timer_deadline_future::timer_deadline_future_at(l, i)
  // R2: park_has_timestamp
  &&& forall |i: int| #![auto] is_park_end_at(l, i) ==>
    park_has_timestamp::has_get_current_time_in_park(l, i)
  // R3: park_poll_once
  &&& forall |i: int| #![auto] is_park_end_at(l, i) ==>
    park_poll_once::has_exactly_one_poll_events_in_park(l, i)
  // R4: io_ready_in_park
  &&& forall |i: int| #![auto] is_io_event_ready_at(l, i) ==>
    io_ready_in_park::is_in_park_cycle(l, i)
  // R10: register_io_in_cycle
  &&& forall |i: int| #![auto] io_syscall_register_at(l, i) ==>
    register_io_in_cycle::in_register_io_cycle(l, i)
  // R11: deregister_io_in_cycle
  &&& forall |i: int| #![auto] io_syscall_deregistered_at(l, i) ==>
    deregister_io_in_cycle::in_deregister_io_cycle(l, i)
  // R13: inbound_deregister_io_result
  &&& forall |i: int| #![auto] is_inbound_deregister_io_end_at(l, i) ==>
    inbound_deregister_io_result::deregister_io_result_valid(l, i)
  // R12: inbound_register_io_result
  &&& forall |i: int| #![auto] io_api_registered_at(l, i) ==>
    inbound_register_io_result::register_io_result_valid(l, i)
  // R15: set_waker_active_io
  &&& forall |i: int| #![auto] is_succ_set_waker_at(l, i) ==>
    set_waker_active_io::set_waker_on_active_io(l, i)
}

pub open spec fn reactor_wf(l: Log, next_rid: nat) -> bool {
  reactor_inv(l) &&
  reactor_ext_inv(l) &&
  alloc_inv(l, next_rid)
}

}
