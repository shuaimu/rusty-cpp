use vstd::prelude::*;
use crate::spec::log::*;
use crate::spec::types::*;
use crate::spec::predicates::*;
use crate::invariants::*;
use crate::invariants::data_inv::*;

verus! {

// Helper: given a witness, prove find_last_set_waker_for_rid_readable is well-defined
proof fn find_last_readable_exists(l: Log, rid: ResourceIdView, before: int, witness: int)
  requires
    0 <= witness < before,
    witness < l.len(),
    is_succ_set_waker_at(l, witness),
    get_set_waker_rid(l[witness]) == rid,
    get_set_waker_interest(l[witness]).0,
  ensures ({
    let j = find_last_set_waker_for_rid_readable(l, rid, before);
    0 <= j < before &&
    j < l.len() &&
    is_succ_set_waker_at(l, j) &&
    get_set_waker_rid(l[j]) == rid &&
    get_set_waker_interest(l[j]).0 &&
    forall |k: int| j < k < before ==> !(
      is_succ_set_waker_at(l, k) &&
      get_set_waker_rid(l[k]) == rid &&
      get_set_waker_interest(l[k]).0
    )
  }),
  decreases before - witness,
{
  if forall |k: int| witness < k < before ==> !(
    is_succ_set_waker_at(l, k) && get_set_waker_rid(l[k]) == rid && get_set_waker_interest(l[k]).0
  ) {
    // witness is the last
    assert(0 <= witness < before &&
      is_succ_set_waker_at(l, witness) &&
      get_set_waker_rid(l[witness]) == rid &&
      get_set_waker_interest(l[witness]).0 &&
      forall |k: int| witness < k < before ==> !(
        is_succ_set_waker_at(l, k) && get_set_waker_rid(l[k]) == rid && get_set_waker_interest(l[k]).0
      ));
  } else {
    let next = choose |k: int| witness < k < before &&
      is_succ_set_waker_at(l, k) && get_set_waker_rid(l[k]) == rid && get_set_waker_interest(l[k]).0;
    find_last_readable_exists(l, rid, before, next);
  }
}

proof fn find_last_writable_exists(l: Log, rid: ResourceIdView, before: int, witness: int)
  requires
    0 <= witness < before,
    witness < l.len(),
    is_succ_set_waker_at(l, witness),
    get_set_waker_rid(l[witness]) == rid,
    get_set_waker_interest(l[witness]).1,
  ensures ({
    let j = find_last_set_waker_for_rid_writable(l, rid, before);
    0 <= j < before &&
    j < l.len() &&
    is_succ_set_waker_at(l, j) &&
    get_set_waker_rid(l[j]) == rid &&
    get_set_waker_interest(l[j]).1 &&
    forall |k: int| j < k < before ==> !(
      is_succ_set_waker_at(l, k) &&
      get_set_waker_rid(l[k]) == rid &&
      get_set_waker_interest(l[k]).1
    )
  }),
  decreases before - witness,
{
  if forall |k: int| witness < k < before ==> !(
    is_succ_set_waker_at(l, k) && get_set_waker_rid(l[k]) == rid && get_set_waker_interest(l[k]).1
  ) {
    assert(0 <= witness < before &&
      is_succ_set_waker_at(l, witness) &&
      get_set_waker_rid(l[witness]) == rid &&
      get_set_waker_interest(l[witness]).1 &&
      forall |k: int| witness < k < before ==> !(
        is_succ_set_waker_at(l, k) && get_set_waker_rid(l[k]) == rid && get_set_waker_interest(l[k]).1
      ));
  } else {
    let next = choose |k: int| witness < k < before &&
      is_succ_set_waker_at(l, k) && get_set_waker_rid(l[k]) == rid && get_set_waker_interest(l[k]).1;
    find_last_writable_exists(l, rid, before, next);
  }
}

#[verifier::rlimit(40)]
proof fn r16_antecedent_false_after_park(
  l0: Log,
  l_final: Log,
  gct_pos: int,
  now: InstantView,
  timers: Set<(InstantView, ResourceIdView, int)>,
  by_rid: Map<ResourceIdView, (InstantView, ResourceIdView, int)>,
  next_rid: nat,
  i: int,
)
  requires
    reactor_inv(l0),
    gct_pos == l0.len() as int + 1,
    l_final.len() > l0.len() + 2,
    forall |k: int| #![auto] 0 <= k < l0.len() ==> l_final[k] == l0[k],
    is_get_current_time_at(l_final, gct_pos),
    get_current_timestamp(l_final[gct_pos]) == now,
    forall |k: int| #![auto] l0.len() as int <= k < l_final.len() && k != gct_pos ==>
      !is_get_current_time_at(l_final, k),
    forall |k: int| #![auto] l0.len() as int <= k < l_final.len() ==>
      !is_succ_register_timer_at(l_final, k) &&
      !is_deregister_timer_at(l_final, k),
    active_timers_in_heap(by_rid, l_final),
    timer_heap_entries_valid(timers, l_final),
    timer_impl_inv(timers, by_rid, next_rid),
    forall |d: InstantView, r: ResourceIdView, log_idx: int|
      #![auto] timers.contains((d, r, log_idx)) ==> d > now,
    is_succ_register_timer_at(l_final, i),
    has_timeout_point(l_final, i),
    timer_awaiting_wake(l_final, i),
  ensures false,
{
  let n0 = l0.len() as int;
  assert(i < n0);
  assert(l_final[i] == l0[i]);
  let rid = get_register_timer_rid(l_final[i]);
  let deadline = get_register_timer_deadline(l_final[i]);

  assert(timer_active_at(l_final, i, l_final.len() as int));
  assert forall |k: int| i < k < n0 implies
    !timer_retired_at(l0, rid, k)
  by {
    assert(l_final[k] == l0[k]);
    not_timer_retired_transfer(l_final, l0, rid, k);
  };
  assert(timer_active_at(l0, i, n0));
  assert forall |k: int| i < k < n0 implies !(
    is_wake_task_at(l0, k) && get_wake_task_source_rid(l0[k]) == rid
  ) by {
    not_timer_retired_implies(l0, rid, k);
  };
  assert(timer_awaiting_wake(l0, i));

  if has_timeout_point(l0, i) {
    reactor_inv_split(l0);
    let j0 = choose |j: int| j > i &&
      is_wake_task_at(l0, j) &&
      get_wake_task_source_rid(l0[j]) == rid &&
      get_wake_task_waker(l0[j]) == get_register_timer_waker(l0[i]) && {
      let timeout_idx = first_timeout_point(l0, i);
      j > timeout_idx &&
      forall |k: int| timeout_idx < k < j ==> !is_park_end_at(l0, k)
    };
    assert(j0 < n0);
    assert(l_final[j0] == l0[j0]);
    assert(is_wake_task_at(l_final, j0));
    assert(get_wake_task_source_rid(l_final[j0]) == get_register_timer_rid(l_final[i]));
    assert(false);
  }

  let tp = choose |timeout_idx: int| timeout_idx > i &&
    is_get_current_time_at(l_final, timeout_idx) &&
    get_current_timestamp(l_final[timeout_idx]) >= deadline &&
    timer_active_at(l_final, i, timeout_idx);
  if tp < n0 {
    assert(l_final[tp] == l0[tp]);
    assert(is_get_current_time_at(l0, tp));
    assert(get_current_timestamp(l0[tp]) >= get_register_timer_deadline(l0[i]));
    assert forall |k: int| i < k < tp implies !(
      is_deregister_timer_at(l0, k) &&
      get_deregister_timer_rid(l0[k]) == get_register_timer_rid(l0[i])
    ) by {
      assert(l_final[k] == l0[k]);
      assert(l_final[i] == l0[i]);
    };
    assert(timer_active_at(l0, i, tp));
    assert(has_timeout_point(l0, i));
    assert(false);
  }
  assert(tp == gct_pos);
  assert(deadline <= now);

  assert(active_timers_in_heap(by_rid, l_final));
  assert(by_rid.contains_key(rid));
  let entry = by_rid[rid];
  assert(entry.2 == i);
  assert(timer_impl_inv(timers, by_rid, next_rid));
  assert(timers.contains(entry));
  assert(timer_heap_entries_valid(timers, l_final));
  assert(get_register_timer_deadline(l_final[entry.2]) == entry.0);
  assert(entry.0 == deadline);
  assert(entry.0 > now);
  assert(false);
}

#[verifier::rlimit(60)]
proof fn r17a_old_trigger_preserved(l0: Log, l_final: Log, i: int)
  requires
    reactor_inv(l0),
    l_final.len() > l0.len(),
    0 <= i < l0.len(),
    forall |k: int| #![auto] 0 <= k < l0.len() ==> l_final[k] == l0[k],
    forall |k: int| #![auto] l0.len() as int <= k < l_final.len() ==>
      !is_succ_set_waker_at(l_final, k),
    is_io_event_ready_at(l0, i),
    has_valid_set_waker_readable_api(l0, i),
  ensures
    exists |j: int| #![trigger is_wake_task_at(l_final, j)] j > i && {
      let event = get_io_event(l_final[i]);
      let rid = event.resource_id;
      let sw_idx = find_last_set_waker_for_rid_readable(l_final, rid, i);
      let waker = get_set_waker_waker(l_final[sw_idx]);
      is_wake_task_at(l_final, j) &&
      get_wake_task_source_rid(l_final[j]) == rid &&
      get_wake_task_waker(l_final[j]) == waker &&
      j > i &&
      forall |k: int| i < k < j ==> !is_park_end_at(l_final, k) && !is_poll_events_at(l_final, k)
    },
{
  let n0 = l0.len() as int;
  assert(l_final[i] == l0[i]);
  let rid_val = get_io_event(l0[i]).resource_id;

  // Get some set_waker witness from has_valid_set_waker_readable_api(l0, i)
  let some_sw = choose |sw_idx: int| 0 <= sw_idx < i &&
    is_succ_set_waker_at(l0, sw_idx) &&
    get_set_waker_rid(l0[sw_idx]) == rid_val &&
    get_set_waker_interest(l0[sw_idx]).0 &&
    io_api_active_at_set_waker(l0, rid_val, sw_idx);

  // Establish find_last_set_waker_for_rid_readable is well-defined on l0
  find_last_readable_exists(l0, rid_val, i, some_sw);
  let sw0 = find_last_set_waker_for_rid_readable(l0, rid_val, i);
  assert(0 <= sw0 < i);
  assert(sw0 < n0);

  // Transport sw0 properties to l_final
  assert(l_final[sw0] == l0[sw0]);
  assert(is_succ_set_waker_at(l_final, sw0));
  assert(get_set_waker_rid(l_final[sw0]) == rid_val);
  assert(get_set_waker_interest(l_final[sw0]).0);

  // sw0 is also the last on l_final (no new set_waker after n0, and no later ones in [0, i))
  assert forall |k: int| sw0 < k < i implies !(
    is_succ_set_waker_at(l_final, k) &&
    get_set_waker_rid(l_final[k]) == rid_val &&
    get_set_waker_interest(l_final[k]).0
  ) by {
    assert(k < n0);
    assert(l_final[k] == l0[k]);
    assert(!(is_succ_set_waker_at(l0, k) && get_set_waker_rid(l0[k]) == rid_val && get_set_waker_interest(l0[k]).0));
  };

  // Establish find_last on l_final
  find_last_readable_exists(l_final, rid_val, i, sw0);
  let sw_final = find_last_set_waker_for_rid_readable(l_final, rid_val, i);
  assert(sw_final == sw0);

  // Get R17a witness from l0
  reactor_inv_split(l0);
  let j0 = choose |j: int| #![trigger is_wake_task_at(l0, j)] j > i && {
    let event2 = get_io_event(l0[i]);
    let rid2 = event2.resource_id;
    let sw_idx2 = find_last_set_waker_for_rid_readable(l0, rid2, i);
    let waker2 = get_set_waker_waker(l0[sw_idx2]);
    is_wake_task_at(l0, j) &&
    get_wake_task_source_rid(l0[j]) == rid2 &&
    get_wake_task_waker(l0[j]) == waker2 &&
    j > i &&
    forall |k: int| i < k < j ==> !is_park_end_at(l0, k) && !is_poll_events_at(l0, k)
  };
  assert(j0 < n0);
  assert(l_final[j0] == l0[j0]);
  assert(is_wake_task_at(l_final, j0));
  assert(get_wake_task_source_rid(l_final[j0]) == rid_val);

  let waker = get_set_waker_waker(l_final[sw_final]);
  assert(waker == get_set_waker_waker(l0[sw0]));
  assert(get_wake_task_waker(l_final[j0]) == waker);

  assert forall |k: int| i < k < j0 implies
    !is_park_end_at(l_final, k) && !is_poll_events_at(l_final, k)
  by {
    assert(k < n0);
    assert(l_final[k] == l0[k]);
    assert(!is_park_end_at(l0, k) && !is_poll_events_at(l0, k));
  };
}

#[verifier::rlimit(60)]
proof fn r17b_old_trigger_preserved(l0: Log, l_final: Log, i: int)
  requires
    reactor_inv(l0),
    l_final.len() > l0.len(),
    0 <= i < l0.len(),
    forall |k: int| #![auto] 0 <= k < l0.len() ==> l_final[k] == l0[k],
    forall |k: int| #![auto] l0.len() as int <= k < l_final.len() ==>
      !is_succ_set_waker_at(l_final, k),
    is_io_event_ready_at(l0, i),
    has_valid_set_waker_writable_api(l0, i),
  ensures
    exists |j: int| #![trigger is_wake_task_at(l_final, j)] j > i && {
      let event = get_io_event(l_final[i]);
      let rid = event.resource_id;
      let sw_idx = find_last_set_waker_for_rid_writable(l_final, rid, i);
      let waker = get_set_waker_waker(l_final[sw_idx]);
      is_wake_task_at(l_final, j) &&
      get_wake_task_source_rid(l_final[j]) == rid &&
      get_wake_task_waker(l_final[j]) == waker &&
      j > i &&
      forall |k: int| i < k < j ==> !is_park_end_at(l_final, k) && !is_poll_events_at(l_final, k)
    },
{
  let n0 = l0.len() as int;
  assert(l_final[i] == l0[i]);
  let rid_val = get_io_event(l0[i]).resource_id;

  let some_sw = choose |sw_idx: int| 0 <= sw_idx < i &&
    is_succ_set_waker_at(l0, sw_idx) &&
    get_set_waker_rid(l0[sw_idx]) == rid_val &&
    get_set_waker_interest(l0[sw_idx]).1 &&
    io_api_active_at_set_waker(l0, rid_val, sw_idx);

  find_last_writable_exists(l0, rid_val, i, some_sw);
  let sw0 = find_last_set_waker_for_rid_writable(l0, rid_val, i);
  assert(0 <= sw0 < i);
  assert(sw0 < n0);

  assert(l_final[sw0] == l0[sw0]);
  assert(is_succ_set_waker_at(l_final, sw0));
  assert(get_set_waker_rid(l_final[sw0]) == rid_val);
  assert(get_set_waker_interest(l_final[sw0]).1);
  assert forall |k: int| sw0 < k < i implies !(
    is_succ_set_waker_at(l_final, k) &&
    get_set_waker_rid(l_final[k]) == rid_val &&
    get_set_waker_interest(l_final[k]).1
  ) by {
    assert(k < n0);
    assert(l_final[k] == l0[k]);
    assert(!(is_succ_set_waker_at(l0, k) && get_set_waker_rid(l0[k]) == rid_val && get_set_waker_interest(l0[k]).1));
  };

  find_last_writable_exists(l_final, rid_val, i, sw0);
  let sw_final = find_last_set_waker_for_rid_writable(l_final, rid_val, i);
  assert(sw_final == sw0);

  reactor_inv_split(l0);
  let j0 = choose |j: int| #![trigger is_wake_task_at(l0, j)] j > i && {
    let event2 = get_io_event(l0[i]);
    let rid2 = event2.resource_id;
    let sw_idx2 = find_last_set_waker_for_rid_writable(l0, rid2, i);
    let waker2 = get_set_waker_waker(l0[sw_idx2]);
    is_wake_task_at(l0, j) &&
    get_wake_task_source_rid(l0[j]) == rid2 &&
    get_wake_task_waker(l0[j]) == waker2 &&
    j > i &&
    forall |k: int| i < k < j ==> !is_park_end_at(l0, k) && !is_poll_events_at(l0, k)
  };
  assert(j0 < n0);
  assert(l_final[j0] == l0[j0]);
  assert(is_wake_task_at(l_final, j0));
  assert(get_wake_task_source_rid(l_final[j0]) == rid_val);

  let waker = get_set_waker_waker(l_final[sw_final]);
  assert(waker == get_set_waker_waker(l0[sw0]));
  assert(get_wake_task_waker(l_final[j0]) == waker);

  assert forall |k: int| i < k < j0 implies
    !is_park_end_at(l_final, k) && !is_poll_events_at(l_final, k)
  by {
    assert(k < n0);
    assert(l_final[k] == l0[k]);
    assert(!is_park_end_at(l0, k) && !is_poll_events_at(l0, k));
  };
}

#[verifier::rlimit(30)]
proof fn has_valid_set_waker_readable_prefix(l0: Log, l_final: Log, i: int)
  requires
    0 <= i < l0.len(),
    l_final.len() > l0.len(),
    forall |k: int| #![auto] 0 <= k < l0.len() ==> l_final[k] == l0[k],
    forall |k: int| #![auto] l0.len() as int <= k < l_final.len() ==>
      !is_succ_set_waker_at(l_final, k) &&
      !io_api_registered_at(l_final, k) &&
      !io_api_deregistered_at(l_final, k),
    is_io_event_ready_at(l_final, i),
    has_valid_set_waker_readable_api(l_final, i),
  ensures
    has_valid_set_waker_readable_api(l0, i),
{
  assert(l_final[i] == l0[i]);
  let rid = get_io_event(l_final[i]).resource_id;
  let sw = choose |sw_idx: int| 0 <= sw_idx < i &&
    is_succ_set_waker_at(l_final, sw_idx) &&
    get_set_waker_rid(l_final[sw_idx]) == rid &&
    get_set_waker_interest(l_final[sw_idx]).0 &&
    io_api_active_at_set_waker(l_final, rid, sw_idx) &&
    forall |k: int| sw_idx < k < i ==> !(
      io_api_deregistered_at(l_final, k) && get_io_api_deregister_rid(l_final[k]) == rid
    );
  assert(sw < l0.len());
  assert(l_final[sw] == l0[sw]);
  assert(is_succ_set_waker_at(l0, sw));
  assert(get_set_waker_rid(l0[sw]) == rid);
  assert(get_set_waker_interest(l0[sw]).0);

  let reg = choose |reg_idx: int| 0 <= reg_idx < sw &&
    io_api_registered_at(l_final, reg_idx) &&
    get_io_api_register_rid(l_final[reg_idx]) == rid &&
    io_api_active_at(l_final, reg_idx, sw);
  assert(reg < l0.len());
  assert(l_final[reg] == l0[reg]);
  assert(io_api_registered_at(l0, reg));
  assert(get_io_api_register_rid(l0[reg]) == rid);
  assert forall |k: int| reg < k < sw implies !(
    io_api_deregistered_at(l0, k) && get_io_api_deregister_rid(l0[k]) == get_io_api_register_rid(l0[reg])
  ) by {
    assert(l_final[k] == l0[k]);
    assert(l_final[reg] == l0[reg]);
    assert(!(io_api_deregistered_at(l_final, k) && get_io_api_deregister_rid(l_final[k]) == get_io_api_register_rid(l_final[reg])));
  };
  assert(io_api_active_at(l0, reg, sw));
  assert(io_api_active_at_set_waker(l0, rid, sw));
  assert forall |k: int| sw < k < i implies !(
    io_api_deregistered_at(l0, k) && get_io_api_deregister_rid(l0[k]) == rid
  ) by {
    assert(l_final[k] == l0[k]);
    assert(!(io_api_deregistered_at(l_final, k) && get_io_api_deregister_rid(l_final[k]) == rid));
  };
}

#[verifier::rlimit(30)]
proof fn has_valid_set_waker_writable_prefix(l0: Log, l_final: Log, i: int)
  requires
    0 <= i < l0.len(),
    l_final.len() > l0.len(),
    forall |k: int| #![auto] 0 <= k < l0.len() ==> l_final[k] == l0[k],
    forall |k: int| #![auto] l0.len() as int <= k < l_final.len() ==>
      !is_succ_set_waker_at(l_final, k) &&
      !io_api_registered_at(l_final, k) &&
      !io_api_deregistered_at(l_final, k),
    is_io_event_ready_at(l_final, i),
    has_valid_set_waker_writable_api(l_final, i),
  ensures
    has_valid_set_waker_writable_api(l0, i),
{
  assert(l_final[i] == l0[i]);
  let rid = get_io_event(l_final[i]).resource_id;
  let sw = choose |sw_idx: int| 0 <= sw_idx < i &&
    is_succ_set_waker_at(l_final, sw_idx) &&
    get_set_waker_rid(l_final[sw_idx]) == rid &&
    get_set_waker_interest(l_final[sw_idx]).1 &&
    io_api_active_at_set_waker(l_final, rid, sw_idx) &&
    forall |k: int| sw_idx < k < i ==> !(
      io_api_deregistered_at(l_final, k) && get_io_api_deregister_rid(l_final[k]) == rid
    );
  assert(sw < l0.len());
  assert(l_final[sw] == l0[sw]);
  assert(is_succ_set_waker_at(l0, sw));
  assert(get_set_waker_rid(l0[sw]) == rid);
  assert(get_set_waker_interest(l0[sw]).1);

  let reg = choose |reg_idx: int| 0 <= reg_idx < sw &&
    io_api_registered_at(l_final, reg_idx) &&
    get_io_api_register_rid(l_final[reg_idx]) == rid &&
    io_api_active_at(l_final, reg_idx, sw);
  assert(reg < l0.len());
  assert(l_final[reg] == l0[reg]);
  assert(io_api_registered_at(l0, reg));
  assert(get_io_api_register_rid(l0[reg]) == rid);
  assert forall |k: int| reg < k < sw implies !(
    io_api_deregistered_at(l0, k) && get_io_api_deregister_rid(l0[k]) == get_io_api_register_rid(l0[reg])
  ) by {
    assert(l_final[k] == l0[k]);
    assert(l_final[reg] == l0[reg]);
    assert(!(io_api_deregistered_at(l_final, k) && get_io_api_deregister_rid(l_final[k]) == get_io_api_register_rid(l_final[reg])));
  };
  assert(io_api_active_at(l0, reg, sw));
  assert(io_api_active_at_set_waker(l0, rid, sw));
  assert forall |k: int| sw < k < i implies !(
    io_api_deregistered_at(l0, k) && get_io_api_deregister_rid(l0[k]) == rid
  ) by {
    assert(l_final[k] == l0[k]);
    assert(!(io_api_deregistered_at(l_final, k) && get_io_api_deregister_rid(l_final[k]) == rid));
  };
}

pub proof fn liveness_inv_after_park_error_path(
  l0: Log,
  l_final: Log,
  gct_pos: int,
  now: InstantView,
  timers: Set<(InstantView, ResourceIdView, int)>,
  by_rid: Map<ResourceIdView, (InstantView, ResourceIdView, int)>,
  next_rid: nat,
)
  requires
    reactor_inv(l0),
    gct_pos == l0.len() as int + 1,
    l_final.len() > l0.len() + 2,
    forall |k: int| #![auto] 0 <= k < l0.len() ==> l_final[k] == l0[k],
    is_get_current_time_at(l_final, gct_pos),
    get_current_timestamp(l_final[gct_pos]) == now,
    forall |k: int| #![auto] l0.len() as int <= k < l_final.len() && k != gct_pos ==>
      !is_get_current_time_at(l_final, k),
    forall |k: int| #![auto] l0.len() as int <= k < l_final.len() ==>
      !is_succ_register_timer_at(l_final, k) &&
      !is_deregister_timer_at(l_final, k) &&
      !io_api_registered_at(l_final, k) &&
      !io_api_deregistered_at(l_final, k) &&
      !is_succ_set_waker_at(l_final, k) &&
      !is_io_event_ready_at(l_final, k),
    active_timers_in_heap(by_rid, l_final),
    timer_heap_entries_valid(timers, l_final),
    timer_impl_inv(timers, by_rid, next_rid),
    forall |d: InstantView, r: ResourceIdView, log_idx: int|
      #![auto] timers.contains((d, r, log_idx)) ==> d > now,
  ensures
    reactor_liveness_inv(l_final),
{
  let n0 = l0.len() as int;

  assert forall |i: int| #![auto]
    (is_succ_register_timer_at(l_final, i) && has_timeout_point(l_final, i) && timer_awaiting_wake(l_final, i)) implies
    exists |j: int| j > i &&
      is_wake_task_at(l_final, j) &&
      get_wake_task_source_rid(l_final[j]) == get_register_timer_rid(l_final[i]) &&
      get_wake_task_waker(l_final[j]) == get_register_timer_waker(l_final[i]) && {
      let timeout_idx = first_timeout_point(l_final, i);
      j > timeout_idx &&
      forall |k: int| timeout_idx < k < j ==> !is_park_end_at(l_final, k)
    }
  by {
    r16_antecedent_false_after_park(l0, l_final, gct_pos, now, timers, by_rid, next_rid, i);
  }

  assert forall |i: int| #![auto]
    (is_io_event_ready_at(l_final, i) && has_valid_set_waker_readable_api(l_final, i)) implies
    exists |j: int| #![trigger is_wake_task_at(l_final, j)] j > i && {
      let event = get_io_event(l_final[i]);
      let rid = event.resource_id;
      let sw_idx = find_last_set_waker_for_rid_readable(l_final, rid, i);
      let waker = get_set_waker_waker(l_final[sw_idx]);
      is_wake_task_at(l_final, j) &&
      get_wake_task_source_rid(l_final[j]) == rid &&
      get_wake_task_waker(l_final[j]) == waker &&
      j > i &&
      forall |k: int| i < k < j ==> !is_park_end_at(l_final, k) && !is_poll_events_at(l_final, k)
    }
  by {
    assert(i < n0);
    assert(l_final[i] == l0[i]);
    assert(is_io_event_ready_at(l0, i));
    has_valid_set_waker_readable_prefix(l0, l_final, i);
    r17a_old_trigger_preserved(l0, l_final, i);
  }

  assert forall |i: int| #![auto]
    (is_io_event_ready_at(l_final, i) && has_valid_set_waker_writable_api(l_final, i)) implies
    exists |j: int| #![trigger is_wake_task_at(l_final, j)] j > i && {
      let event = get_io_event(l_final[i]);
      let rid = event.resource_id;
      let sw_idx = find_last_set_waker_for_rid_writable(l_final, rid, i);
      let waker = get_set_waker_waker(l_final[sw_idx]);
      is_wake_task_at(l_final, j) &&
      get_wake_task_source_rid(l_final[j]) == rid &&
      get_wake_task_waker(l_final[j]) == waker &&
      j > i &&
      forall |k: int| i < k < j ==> !is_park_end_at(l_final, k) && !is_poll_events_at(l_final, k)
    }
  by {
    assert(i < n0);
    assert(l_final[i] == l0[i]);
    assert(is_io_event_ready_at(l0, i));
    has_valid_set_waker_writable_prefix(l0, l_final, i);
    r17b_old_trigger_preserved(l0, l_final, i);
  }
}

#[verifier::rlimit(60)]
proof fn r17a_new_trigger(
  l0: Log,
  l_pre_end: Log,
  l_final: Log,
  read_wakers: Map<ResourceIdView, WakerView>,
  i: int,
)
  requires
    l_final.len() == l_pre_end.len() + 1,
    l0.len() <= i,
    i < l_pre_end.len(),
    forall |k: int| #![auto] 0 <= k < l_pre_end.len() ==> l_final[k] == l_pre_end[k],
    forall |k: int| #![auto] l0.len() as int <= k < l_final.len() ==>
      !is_succ_set_waker_at(l_final, k) &&
      !io_api_deregistered_at(l_final, k),
    is_io_event_ready_at(l_final, i),
    has_valid_set_waker_readable_api(l_final, i),
    read_wakers_valid(read_wakers, l_pre_end),
    read_wakers_complete(read_wakers, l_pre_end),
    read_wakers.contains_key(get_io_event(l_final[i]).resource_id),
    i + 1 < l_final.len(),
    is_wake_task_at(l_final, i + 1),
    get_wake_task_source_rid(l_final[i + 1]) == get_io_event(l_final[i]).resource_id,
    get_wake_task_waker(l_final[i + 1]) == read_wakers[get_io_event(l_final[i]).resource_id],
  ensures
    exists |j: int| #![trigger is_wake_task_at(l_final, j)] j > i && {
      let event = get_io_event(l_final[i]);
      let rid = event.resource_id;
      let sw_idx = find_last_set_waker_for_rid_readable(l_final, rid, i);
      let waker = get_set_waker_waker(l_final[sw_idx]);
      is_wake_task_at(l_final, j) &&
      get_wake_task_source_rid(l_final[j]) == rid &&
      get_wake_task_waker(l_final[j]) == waker &&
      j > i &&
      forall |k: int| i < k < j ==> !is_park_end_at(l_final, k) && !is_poll_events_at(l_final, k)
    },
{
  let rid = get_io_event(l_final[i]).resource_id;

  assert(read_wakers.contains_key(rid));
  assert(io_currently_active(l_pre_end, rid));

  let sw_v = choose |sw_idx: int| 0 <= sw_idx < l_pre_end.len() &&
    is_succ_set_waker_at(l_pre_end, sw_idx) &&
    get_set_waker_rid(l_pre_end[sw_idx]) == rid &&
    get_set_waker_interest(l_pre_end[sw_idx]).0 &&
    get_set_waker_waker(l_pre_end[sw_idx]) == read_wakers[rid] &&
    io_api_active_at_set_waker(l_pre_end, rid, sw_idx) &&
    forall |k: int| sw_idx < k < l_pre_end.len() ==> !(
      is_succ_set_waker_at(l_pre_end, k) &&
      get_set_waker_rid(l_pre_end[k]) == rid &&
      get_set_waker_interest(l_pre_end[k]).0
    );
  assert(0 <= sw_v < l_pre_end.len() as int);
  assert(sw_v < l0.len() as int) by {
    if sw_v >= l0.len() as int {
      assert(l_final[sw_v] == l_pre_end[sw_v]);
      assert(!is_succ_set_waker_at(l_final, sw_v));
      assert(is_succ_set_waker_at(l_pre_end, sw_v));
    }
  };

  assert(l_final[sw_v] == l_pre_end[sw_v]);
  assert(is_succ_set_waker_at(l_final, sw_v));
  assert(get_set_waker_rid(l_final[sw_v]) == rid);
  assert(get_set_waker_interest(l_final[sw_v]).0);

  assert forall |k: int| sw_v < k < i implies !(
    is_succ_set_waker_at(l_final, k) &&
    get_set_waker_rid(l_final[k]) == rid &&
    get_set_waker_interest(l_final[k]).0
  ) by {
    if k < l0.len() as int {
      assert(k < l_pre_end.len());
      assert(l_final[k] == l_pre_end[k]);
      assert(!(is_succ_set_waker_at(l_pre_end, k) && get_set_waker_rid(l_pre_end[k]) == rid && get_set_waker_interest(l_pre_end[k]).0));
    } else {
      assert(!is_succ_set_waker_at(l_final, k));
    }
  };

  find_last_readable_exists(l_final, rid, i, sw_v);
  let fl = find_last_set_waker_for_rid_readable(l_final, rid, i);
  assert(fl == sw_v);
  assert(get_set_waker_waker(l_final[fl]) == read_wakers[rid]);
  assert(get_wake_task_waker(l_final[i + 1]) == get_set_waker_waker(l_final[fl]));
}

#[verifier::rlimit(60)]
proof fn r17b_new_trigger(
  l0: Log,
  l_pre_end: Log,
  l_final: Log,
  write_wakers: Map<ResourceIdView, WakerView>,
  i: int,
)
  requires
    l_final.len() == l_pre_end.len() + 1,
    l0.len() <= i,
    i < l_pre_end.len(),
    forall |k: int| #![auto] 0 <= k < l_pre_end.len() ==> l_final[k] == l_pre_end[k],
    forall |k: int| #![auto] l0.len() as int <= k < l_final.len() ==>
      !is_succ_set_waker_at(l_final, k) &&
      !io_api_deregistered_at(l_final, k),
    is_io_event_ready_at(l_final, i),
    has_valid_set_waker_writable_api(l_final, i),
    write_wakers_valid(write_wakers, l_pre_end),
    write_wakers_complete(write_wakers, l_pre_end),
    write_wakers.contains_key(get_io_event(l_final[i]).resource_id),
    i + 1 < l_final.len(),
    is_wake_task_at(l_final, i + 1),
    get_wake_task_source_rid(l_final[i + 1]) == get_io_event(l_final[i]).resource_id,
    get_wake_task_waker(l_final[i + 1]) == write_wakers[get_io_event(l_final[i]).resource_id],
  ensures
    exists |j: int| #![trigger is_wake_task_at(l_final, j)] j > i && {
      let event = get_io_event(l_final[i]);
      let rid = event.resource_id;
      let sw_idx = find_last_set_waker_for_rid_writable(l_final, rid, i);
      let waker = get_set_waker_waker(l_final[sw_idx]);
      is_wake_task_at(l_final, j) &&
      get_wake_task_source_rid(l_final[j]) == rid &&
      get_wake_task_waker(l_final[j]) == waker &&
      j > i &&
      forall |k: int| i < k < j ==> !is_park_end_at(l_final, k) && !is_poll_events_at(l_final, k)
    },
{
  let rid = get_io_event(l_final[i]).resource_id;

  assert(write_wakers.contains_key(rid));
  assert(io_currently_active(l_pre_end, rid));

  let sw_v = choose |sw_idx: int| 0 <= sw_idx < l_pre_end.len() &&
    is_succ_set_waker_at(l_pre_end, sw_idx) &&
    get_set_waker_rid(l_pre_end[sw_idx]) == rid &&
    get_set_waker_interest(l_pre_end[sw_idx]).1 &&
    get_set_waker_waker(l_pre_end[sw_idx]) == write_wakers[rid] &&
    io_api_active_at_set_waker(l_pre_end, rid, sw_idx) &&
    forall |k: int| sw_idx < k < l_pre_end.len() ==> !(
      is_succ_set_waker_at(l_pre_end, k) &&
      get_set_waker_rid(l_pre_end[k]) == rid &&
      get_set_waker_interest(l_pre_end[k]).1
    );
  assert(0 <= sw_v < l_pre_end.len() as int);
  assert(sw_v < l0.len() as int) by {
    if sw_v >= l0.len() as int {
      assert(l_final[sw_v] == l_pre_end[sw_v]);
      assert(!is_succ_set_waker_at(l_final, sw_v));
      assert(is_succ_set_waker_at(l_pre_end, sw_v));
    }
  };

  assert(l_final[sw_v] == l_pre_end[sw_v]);
  assert(is_succ_set_waker_at(l_final, sw_v));
  assert(get_set_waker_rid(l_final[sw_v]) == rid);
  assert(get_set_waker_interest(l_final[sw_v]).1);

  assert forall |k: int| sw_v < k < i implies !(
    is_succ_set_waker_at(l_final, k) &&
    get_set_waker_rid(l_final[k]) == rid &&
    get_set_waker_interest(l_final[k]).1
  ) by {
    if k < l0.len() as int {
      assert(k < l_pre_end.len());
      assert(l_final[k] == l_pre_end[k]);
      assert(!(is_succ_set_waker_at(l_pre_end, k) && get_set_waker_rid(l_pre_end[k]) == rid && get_set_waker_interest(l_pre_end[k]).1));
    } else {
      assert(!is_succ_set_waker_at(l_final, k));
    }
  };

  find_last_writable_exists(l_final, rid, i, sw_v);
  let fl = find_last_set_waker_for_rid_writable(l_final, rid, i);
  assert(fl == sw_v);
  assert(get_set_waker_waker(l_final[fl]) == write_wakers[rid]);
  assert(get_wake_task_waker(l_final[i + 1]) == get_set_waker_waker(l_final[fl]));
}

proof fn has_valid_readable_implies_rw_has_key(
  l0: Log,
  l_pre_end: Log,
  l_final: Log,
  read_wakers: Map<ResourceIdView, WakerView>,
  i: int,
)
  requires
    l_final.len() == l_pre_end.len() + 1,
    l0.len() <= i,
    i < l_pre_end.len(),
    forall |k: int| #![auto] 0 <= k < l_pre_end.len() ==> l_final[k] == l_pre_end[k],
    forall |k: int| #![auto] l0.len() as int <= k < l_final.len() ==>
      !is_succ_set_waker_at(l_final, k) &&
      !io_api_registered_at(l_final, k) &&
      !io_api_deregistered_at(l_final, k),
    is_io_event_ready_at(l_final, i),
    has_valid_set_waker_readable_api(l_final, i),
    read_wakers_complete(read_wakers, l_pre_end),
  ensures
    read_wakers.contains_key(get_io_event(l_final[i]).resource_id),
{
  let rid = get_io_event(l_final[i]).resource_id;
  let n0 = l0.len() as int;

  let sw = choose |sw_idx: int| 0 <= sw_idx < i &&
    is_succ_set_waker_at(l_final, sw_idx) &&
    get_set_waker_rid(l_final[sw_idx]) == rid &&
    get_set_waker_interest(l_final[sw_idx]).0 &&
    io_api_active_at_set_waker(l_final, rid, sw_idx) &&
    forall |k: int| sw_idx < k < i ==> !(
      io_api_deregistered_at(l_final, k) && get_io_api_deregister_rid(l_final[k]) == rid
    );
  assert(sw < n0) by {
    if sw >= n0 {
      assert(!is_succ_set_waker_at(l_final, sw));
    }
  };
  assert(l_final[sw] == l_pre_end[sw]);
  assert(is_succ_set_waker_at(l_pre_end, sw));
  assert(get_set_waker_rid(l_pre_end[sw]) == rid);
  assert(get_set_waker_interest(l_pre_end[sw]).0);

  let reg = choose |reg_idx: int| 0 <= reg_idx < sw &&
    io_api_registered_at(l_final, reg_idx) &&
    get_io_api_register_rid(l_final[reg_idx]) == rid &&
    io_api_active_at(l_final, reg_idx, sw);
  assert(l_final[reg] == l_pre_end[reg]);
  assert(io_api_registered_at(l_pre_end, reg));
  assert(get_io_api_register_rid(l_pre_end[reg]) == rid);

  assert forall |k: int| reg < k < l_pre_end.len() implies !(
    io_api_deregistered_at(l_pre_end, k) && get_io_api_deregister_rid(l_pre_end[k]) == rid
  ) by {
    assert(l_final[k] == l_pre_end[k]);
    if k < sw {
      assert(!(io_api_deregistered_at(l_final, k) && get_io_api_deregister_rid(l_final[k]) == rid));
    } else if k < i {
      assert(!(io_api_deregistered_at(l_final, k) && get_io_api_deregister_rid(l_final[k]) == rid));
    } else {
      assert(!io_api_deregistered_at(l_final, k));
    }
  };
  assert(io_api_active_at(l_pre_end, reg, l_pre_end.len() as int));
  assert(io_currently_active(l_pre_end, rid));
  assert(io_api_active_at_set_waker(l_pre_end, rid, sw));
  assert(has_active_readable_set_waker(l_pre_end, rid));
}

proof fn has_valid_writable_implies_ww_has_key(
  l0: Log,
  l_pre_end: Log,
  l_final: Log,
  write_wakers: Map<ResourceIdView, WakerView>,
  i: int,
)
  requires
    l_final.len() == l_pre_end.len() + 1,
    l0.len() <= i,
    i < l_pre_end.len(),
    forall |k: int| #![auto] 0 <= k < l_pre_end.len() ==> l_final[k] == l_pre_end[k],
    forall |k: int| #![auto] l0.len() as int <= k < l_final.len() ==>
      !is_succ_set_waker_at(l_final, k) &&
      !io_api_registered_at(l_final, k) &&
      !io_api_deregistered_at(l_final, k),
    is_io_event_ready_at(l_final, i),
    has_valid_set_waker_writable_api(l_final, i),
    write_wakers_complete(write_wakers, l_pre_end),
  ensures
    write_wakers.contains_key(get_io_event(l_final[i]).resource_id),
{
  let rid = get_io_event(l_final[i]).resource_id;
  let n0 = l0.len() as int;

  let sw = choose |sw_idx: int| 0 <= sw_idx < i &&
    is_succ_set_waker_at(l_final, sw_idx) &&
    get_set_waker_rid(l_final[sw_idx]) == rid &&
    get_set_waker_interest(l_final[sw_idx]).1 &&
    io_api_active_at_set_waker(l_final, rid, sw_idx) &&
    forall |k: int| sw_idx < k < i ==> !(
      io_api_deregistered_at(l_final, k) && get_io_api_deregister_rid(l_final[k]) == rid
    );
  assert(sw < n0) by {
    if sw >= n0 {
      assert(!is_succ_set_waker_at(l_final, sw));
    }
  };
  assert(l_final[sw] == l_pre_end[sw]);
  assert(is_succ_set_waker_at(l_pre_end, sw));
  assert(get_set_waker_rid(l_pre_end[sw]) == rid);
  assert(get_set_waker_interest(l_pre_end[sw]).1);

  let reg = choose |reg_idx: int| 0 <= reg_idx < sw &&
    io_api_registered_at(l_final, reg_idx) &&
    get_io_api_register_rid(l_final[reg_idx]) == rid &&
    io_api_active_at(l_final, reg_idx, sw);
  assert(l_final[reg] == l_pre_end[reg]);
  assert(io_api_registered_at(l_pre_end, reg));
  assert(get_io_api_register_rid(l_pre_end[reg]) == rid);

  assert forall |k: int| reg < k < l_pre_end.len() implies !(
    io_api_deregistered_at(l_pre_end, k) && get_io_api_deregister_rid(l_pre_end[k]) == rid
  ) by {
    assert(l_final[k] == l_pre_end[k]);
    if k < sw {
      assert(!(io_api_deregistered_at(l_final, k) && get_io_api_deregister_rid(l_final[k]) == rid));
    } else if k < i {
      assert(!(io_api_deregistered_at(l_final, k) && get_io_api_deregister_rid(l_final[k]) == rid));
    } else {
      assert(!io_api_deregistered_at(l_final, k));
    }
  };
  assert(io_api_active_at(l_pre_end, reg, l_pre_end.len() as int));
  assert(io_currently_active(l_pre_end, rid));
  assert(io_api_active_at_set_waker(l_pre_end, rid, sw));
  assert(has_active_writable_set_waker(l_pre_end, rid));
}

#[verifier::rlimit(60)]
pub proof fn liveness_inv_after_park_normal_path(
  l0: Log,
  l_pre_end: Log,
  l_final: Log,
  gct_pos: int,
  now: InstantView,
  timers: Set<(InstantView, ResourceIdView, int)>,
  by_rid: Map<ResourceIdView, (InstantView, ResourceIdView, int)>,
  next_rid: nat,
  read_wakers: Map<ResourceIdView, WakerView>,
  write_wakers: Map<ResourceIdView, WakerView>,
)
  requires
    reactor_inv(l0),
    gct_pos == l0.len() as int + 1,
    l_pre_end.len() > l0.len() + 2,
    l_final.len() == l_pre_end.len() + 1,
    forall |k: int| #![auto] 0 <= k < l_pre_end.len() ==> l_final[k] == l_pre_end[k],
    forall |k: int| #![auto] 0 <= k < l0.len() ==> l_pre_end[k] == l0[k],
    is_get_current_time_at(l_final, gct_pos),
    get_current_timestamp(l_final[gct_pos]) == now,
    forall |k: int| #![auto] l0.len() as int <= k < l_final.len() && k != gct_pos ==>
      !is_get_current_time_at(l_final, k),
    forall |k: int| #![auto] l0.len() as int <= k < l_final.len() ==>
      !is_succ_register_timer_at(l_final, k) &&
      !is_deregister_timer_at(l_final, k) &&
      !io_api_registered_at(l_final, k) &&
      !io_api_deregistered_at(l_final, k) &&
      !is_succ_set_waker_at(l_final, k),
    active_timers_in_heap(by_rid, l_final),
    timer_heap_entries_valid(timers, l_final),
    timer_impl_inv(timers, by_rid, next_rid),
    forall |d: InstantView, r: ResourceIdView, log_idx: int|
      #![auto] timers.contains((d, r, log_idx)) ==> d > now,
    is_park_end_at(l_final, l_pre_end.len() as int),
    read_wakers_valid(read_wakers, l_pre_end),
    read_wakers_complete(read_wakers, l_pre_end),
    write_wakers_valid(write_wakers, l_pre_end),
    write_wakers_complete(write_wakers, l_pre_end),
    forall |p: int| #![auto] l0.len() as int <= p < l_pre_end.len() as int &&
      is_io_event_ready_at(l_final, p) &&
      get_io_event(l_final[p]).readable &&
      read_wakers.contains_key(get_io_event(l_final[p]).resource_id) ==> {
        let rid = get_io_event(l_final[p]).resource_id;
        p + 1 < l_final.len() as int &&
        is_wake_task_at(l_final, p + 1) &&
        get_wake_task_source_rid(l_final[p + 1]) == rid &&
        get_wake_task_waker(l_final[p + 1]) == read_wakers[rid]
      },
    forall |p: int| #![auto] l0.len() as int <= p < l_pre_end.len() as int &&
      is_io_event_ready_at(l_final, p) &&
      get_io_event(l_final[p]).writable &&
      write_wakers.contains_key(get_io_event(l_final[p]).resource_id) ==> {
        let rid = get_io_event(l_final[p]).resource_id;
        p + 1 < l_final.len() as int &&
        is_wake_task_at(l_final, p + 1) &&
        get_wake_task_source_rid(l_final[p + 1]) == rid &&
        get_wake_task_waker(l_final[p + 1]) == write_wakers[rid]
      },
  ensures
    reactor_liveness_inv(l_final),
{
  let n0 = l0.len() as int;
  let n_pe = l_pre_end.len() as int;

  assert forall |k: int| #![auto] 0 <= k < n0 implies l_final[k] == l0[k] by {
    assert(l_pre_end[k] == l0[k]);
    assert(l_final[k] == l_pre_end[k]);
  };

  // R16: antecedent is false
  assert forall |i: int| #![auto]
    (is_succ_register_timer_at(l_final, i) && has_timeout_point(l_final, i) && timer_awaiting_wake(l_final, i)) implies
    exists |j: int| j > i &&
      is_wake_task_at(l_final, j) &&
      get_wake_task_source_rid(l_final[j]) == get_register_timer_rid(l_final[i]) &&
      get_wake_task_waker(l_final[j]) == get_register_timer_waker(l_final[i]) && {
      let timeout_idx = first_timeout_point(l_final, i);
      j > timeout_idx &&
      forall |k: int| timeout_idx < k < j ==> !is_park_end_at(l_final, k)
    }
  by {
    r16_antecedent_false_after_park(l0, l_final, gct_pos, now, timers, by_rid, next_rid, i);
  }

  // R17a
  assert forall |i: int| #![auto]
    (is_io_event_ready_at(l_final, i) && has_valid_set_waker_readable_api(l_final, i)) implies
    exists |j: int| #![trigger is_wake_task_at(l_final, j)] j > i && {
      let event = get_io_event(l_final[i]);
      let rid = event.resource_id;
      let sw_idx = find_last_set_waker_for_rid_readable(l_final, rid, i);
      let waker = get_set_waker_waker(l_final[sw_idx]);
      is_wake_task_at(l_final, j) &&
      get_wake_task_source_rid(l_final[j]) == rid &&
      get_wake_task_waker(l_final[j]) == waker &&
      j > i &&
      forall |k: int| i < k < j ==> !is_park_end_at(l_final, k) && !is_poll_events_at(l_final, k)
    }
  by {
    if i < n0 {
      assert(l_final[i] == l0[i]);
      assert(is_io_event_ready_at(l0, i));
      has_valid_set_waker_readable_prefix(l0, l_final, i);
      r17a_old_trigger_preserved(l0, l_final, i);
    } else {
      assert(i < n_pe) by {
        if i == n_pe {
          assert(is_park_end_at(l_final, n_pe));
        }
      };
      let rid = get_io_event(l_final[i]).resource_id;
      has_valid_readable_implies_rw_has_key(l0, l_pre_end, l_final, read_wakers, i);
      r17a_new_trigger(l0, l_pre_end, l_final, read_wakers, i);
    }
  }

  // R17b
  assert forall |i: int| #![auto]
    (is_io_event_ready_at(l_final, i) && has_valid_set_waker_writable_api(l_final, i)) implies
    exists |j: int| #![trigger is_wake_task_at(l_final, j)] j > i && {
      let event = get_io_event(l_final[i]);
      let rid = event.resource_id;
      let sw_idx = find_last_set_waker_for_rid_writable(l_final, rid, i);
      let waker = get_set_waker_waker(l_final[sw_idx]);
      is_wake_task_at(l_final, j) &&
      get_wake_task_source_rid(l_final[j]) == rid &&
      get_wake_task_waker(l_final[j]) == waker &&
      j > i &&
      forall |k: int| i < k < j ==> !is_park_end_at(l_final, k) && !is_poll_events_at(l_final, k)
    }
  by {
    if i < n0 {
      assert(l_final[i] == l0[i]);
      assert(is_io_event_ready_at(l0, i));
      has_valid_set_waker_writable_prefix(l0, l_final, i);
      r17b_old_trigger_preserved(l0, l_final, i);
    } else {
      assert(i < n_pe) by {
        if i == n_pe {
          assert(is_park_end_at(l_final, n_pe));
        }
      };
      let rid = get_io_event(l_final[i]).resource_id;
      has_valid_writable_implies_ww_has_key(l0, l_pre_end, l_final, write_wakers, i);
      r17b_new_trigger(l0, l_pre_end, l_final, write_wakers, i);
    }
  }
}

} // end verus!
