use vstd::prelude::*;
use crate::spec::log::*;
use crate::spec::types::*;
use crate::spec::predicates::*;
use crate::invariants::*;
use crate::invariants::timer_waker_validity::*;
use crate::invariants::io_waker_validity::*;
use crate::invariants::data_inv::*;

verus! {

#[verifier::rlimit(300)]
pub proof fn reactor_safety_inv_preserved_by_non_wake(l: Log, e: ReactorEvent)
  requires
    reactor_safety_inv(l),
    !is_succ_register_timer_at(l.push(e), l.len() as int),
    !io_api_registered_at(l.push(e), l.len() as int),
    !is_wake_task_at(l.push(e), l.len() as int),
  ensures
    reactor_safety_inv(l.push(e)),
{
  let l2 = l.push(e);
  let n = l.len() as int;

  // R7
  assert forall |i: int| #![auto] is_succ_register_timer_at(l2, i) implies {
    let rid = get_register_timer_rid(l2[i]);
    no_prior_timer_registration(l2, rid, i)
  } by {
    assert(i < n);
    assert(l2[i] == l[i]);
    assert(is_succ_register_timer_at(l, i));
    let rid = get_register_timer_rid(l[i]);
    assert(no_prior_timer_registration(l, rid, i));
    no_prior_timer_reg_preserved(l, e, rid, i);
  }

  // R8
  assert forall |i: int| #![auto] io_api_registered_at(l2, i) implies {
    let rid = get_io_api_register_rid(l2[i]);
    no_prior_io_api_registration(l2, rid, i)
  } by {
    assert(i < n);
    assert(l2[i] == l[i]);
    assert(io_api_registered_at(l, i));
    let rid = get_io_api_register_rid(l[i]);
    assert(no_prior_io_api_registration(l, rid, i));
    no_prior_io_api_reg_preserved(l, e, rid, i);
  }

  // R9a
  assert forall |i: int| #![auto] is_succ_register_timer_at(l2, i) implies {
    let rid = get_register_timer_rid(l2[i]);
    no_io_api_with_rid_before(l2, rid, i)
  } by {
    assert(i < n);
    assert(l2[i] == l[i]);
    assert(is_succ_register_timer_at(l, i));
    let rid = get_register_timer_rid(l[i]);
    assert(no_io_api_with_rid_before(l, rid, i));
    no_io_api_with_rid_preserved(l, e, rid, i);
  }

  // R9b
  assert forall |i: int| #![auto] io_api_registered_at(l2, i) implies {
    let rid = get_io_api_register_rid(l2[i]);
    no_timer_with_rid_before(l2, rid, i)
  } by {
    assert(i < n);
    assert(l2[i] == l[i]);
    assert(io_api_registered_at(l, i));
    let rid = get_io_api_register_rid(l[i]);
    assert(no_timer_with_rid_before(l, rid, i));
    no_timer_with_rid_preserved(l, e, rid, i);
  }

  // R5
  assert forall |i: int| #![auto] is_timer_wake_at(l2, i) implies {
    let rid = get_wake_task_source_rid(l2[i]);
    let waker = get_wake_task_waker(l2[i]);
    exists |j: int| 0 <= j < i &&
      is_succ_register_timer_at(l2, j) &&
      get_register_timer_rid(l2[j]) == rid &&
      get_register_timer_waker(l2[j]) == waker &&
      timer_active_at(l2, j, i)
  } by {
    assert(i < n);
    assert(l2[i] == l[i]);
    assert(is_wake_task_at(l, i));
    assert(is_timer_wake_at(l2, i));
    let j2 = choose |j2: int| 0 <= j2 < i &&
      is_succ_register_timer_at(l2, j2) &&
      get_register_timer_rid(l2[j2]) == get_wake_task_source_rid(l2[i]) &&
      timer_active_at(l2, j2, i);
    assert(j2 < n);
    assert(l2[j2] == l[j2]);
    assert(is_succ_register_timer_at(l, j2));
    assert(get_register_timer_rid(l[j2]) == get_wake_task_source_rid(l[i]));
    assert(timer_active_at(l2, j2, i));
    let rid2 = get_register_timer_rid(l2[j2]);
    assert forall |k: int| j2 < k < i implies
      !timer_retired_at(l, rid2, k)
    by {
      assert(l2[k] == l[k]);
      not_timer_retired_transfer(l2, l, rid2, k);
    };
    assert(timer_active_at(l, j2, i));
    assert(is_timer_wake_at(l, i));
    let rid = get_wake_task_source_rid(l[i]);
    let waker = get_wake_task_waker(l[i]);
    let j = choose |j: int| 0 <= j < i &&
      is_succ_register_timer_at(l, j) &&
      get_register_timer_rid(l[j]) == rid &&
      get_register_timer_waker(l[j]) == waker &&
      timer_active_at(l, j, i);
    assert(l2[j] == l[j]);
    assert(is_succ_register_timer_at(l2, j));
    assert(get_register_timer_rid(l2[j]) == rid);
    assert(get_register_timer_waker(l2[j]) == waker);
    assert(timer_active_at(l, j, i));
    assert forall |k: int| j < k < i implies
      !timer_retired_at(l2, rid, k)
    by {
      assert(l2[k] == l[k]);
      not_timer_retired_transfer(l, l2, rid, k);
    };
    assert(timer_active_at(l2, j, i));
  }

  // R6
  assert forall |i: int| #![auto] is_io_api_wake_at(l2, i) implies {
    let rid = get_wake_task_source_rid(l2[i]);
    let waker = get_wake_task_waker(l2[i]);
    exists |sw_idx: int| 0 <= sw_idx < i &&
      is_succ_set_waker_at(l2, sw_idx) &&
      get_set_waker_rid(l2[sw_idx]) == rid &&
      get_set_waker_waker(l2[sw_idx]) == waker &&
      io_api_active_at_set_waker(l2, rid, sw_idx)
  } by {
    assert(i < n);
    assert(l2[i] == l[i]);
    assert(is_wake_task_at(l, i));
    assert(is_io_api_wake_at(l2, i));
    let j_io = choose |j_io: int| 0 <= j_io < i &&
      io_api_registered_at(l2, j_io) &&
      get_io_api_register_rid(l2[j_io]) == get_wake_task_source_rid(l2[i]) &&
      io_api_active_at(l2, j_io, i);
    assert(j_io < n);
    assert(l2[j_io] == l[j_io]);
    assert(io_api_registered_at(l, j_io));
    assert(get_io_api_register_rid(l[j_io]) == get_wake_task_source_rid(l[i]));
    assert(io_api_active_at(l2, j_io, i));
    assert forall |k: int| j_io < k < i implies !(
      io_api_deregistered_at(l, k) &&
      get_io_api_deregister_rid(l[k]) == get_io_api_register_rid(l[j_io])
    ) by {
      assert(l2[k] == l[k]);
      assert(!(io_api_deregistered_at(l2, k) &&
        get_io_api_deregister_rid(l2[k]) == get_io_api_register_rid(l2[j_io])));
    }
    assert(io_api_active_at(l, j_io, i));
    assert(is_io_api_wake_at(l, i));
    let rid = get_wake_task_source_rid(l[i]);
    let waker = get_wake_task_waker(l[i]);
    let sw_idx = choose |sw_idx: int| 0 <= sw_idx < i &&
      is_succ_set_waker_at(l, sw_idx) &&
      get_set_waker_rid(l[sw_idx]) == rid &&
      get_set_waker_waker(l[sw_idx]) == waker &&
      io_api_active_at_set_waker(l, rid, sw_idx);
    assert(l2[sw_idx] == l[sw_idx]);
    let reg_idx = choose |reg_idx: int| 0 <= reg_idx < sw_idx &&
      io_api_registered_at(l, reg_idx) &&
      get_io_api_register_rid(l[reg_idx]) == rid &&
      io_api_active_at(l, reg_idx, sw_idx);
    assert(l2[reg_idx] == l[reg_idx]);
    assert(io_api_active_at(l, reg_idx, sw_idx));
    assert forall |k: int| reg_idx < k < sw_idx implies !(
      io_api_deregistered_at(l2, k) &&
      get_io_api_deregister_rid(l2[k]) == get_io_api_register_rid(l2[reg_idx])
    ) by {
      assert(l2[k] == l[k]);
      assert(!(io_api_deregistered_at(l, k) &&
        get_io_api_deregister_rid(l[k]) == get_io_api_register_rid(l[reg_idx])));
    }
    assert(io_api_registered_at(l2, reg_idx));
    assert(get_io_api_register_rid(l2[reg_idx]) == rid);
    assert(io_api_active_at(l2, reg_idx, sw_idx));
    assert(io_api_active_at_set_waker(l2, rid, sw_idx));
    assert(is_succ_set_waker_at(l2, sw_idx));
    assert(get_set_waker_rid(l2[sw_idx]) == get_wake_task_source_rid(l2[i]));
    assert(get_set_waker_waker(l2[sw_idx]) == get_wake_task_waker(l2[i]));
    assert(io_api_active_at_set_waker(l2, get_wake_task_source_rid(l2[i]), sw_idx));
  }

  // R14
  assert forall |i: int| #![auto] is_wake_task_at(l2, i) implies {
    let rid = get_wake_task_source_rid(l2[i]);
    (exists |j: int| 0 <= j < i &&
      is_succ_register_timer_at(l2, j) &&
      get_register_timer_rid(l2[j]) == rid)
    ||
    (exists |j: int| 0 <= j < i &&
      io_api_registered_at(l2, j) &&
      get_io_api_register_rid(l2[j]) == rid)
  } by {
    assert(i < n);
    assert(l2[i] == l[i]);
    assert(is_wake_task_at(l, i));
    let rid = get_wake_task_source_rid(l[i]);
    if exists |j: int| 0 <= j < i &&
      is_succ_register_timer_at(l, j) &&
      get_register_timer_rid(l[j]) == rid
    {
      let j = choose |j: int| 0 <= j < i &&
        is_succ_register_timer_at(l, j) &&
        get_register_timer_rid(l[j]) == rid;
      assert(l2[j] == l[j]);
      assert(is_succ_register_timer_at(l2, j));
      assert(get_register_timer_rid(l2[j]) == get_wake_task_source_rid(l2[i]));
    } else {
      let j = choose |j: int| 0 <= j < i &&
        io_api_registered_at(l, j) &&
        get_io_api_register_rid(l[j]) == rid;
      assert(l2[j] == l[j]);
      assert(io_api_registered_at(l2, j));
      assert(get_io_api_register_rid(l2[j]) == get_wake_task_source_rid(l2[i]));
    }
  }
}

#[verifier::rlimit(20)]
pub proof fn timer_heap_entries_valid_preserved_by_non_timer_event(
  timers_view: Set<(InstantView, ResourceIdView, int)>,
  l: Log,
  e: ReactorEvent,
)
  requires
    timer_heap_entries_valid(timers_view, l),
    !is_deregister_timer_at(l.push(e), l.len() as int),
    !is_wake_task_at(l.push(e), l.len() as int),
  ensures
    timer_heap_entries_valid(timers_view, l.push(e)),
{
  let l2 = l.push(e);
  let n = l.len() as int;
  assert forall |d: InstantView, rid: ResourceIdView, log_idx: int|
    #![auto] timers_view.contains((d, rid, log_idx)) implies {
      timer_awaiting_wake(l2, log_idx) &&
      get_register_timer_rid(l2[log_idx]) == rid &&
      get_register_timer_deadline(l2[log_idx]) == d
    }
  by {
    assert(timer_awaiting_wake(l, log_idx));
    assert(log_idx < n);
    assert(l2[log_idx] == l[log_idx]);
    assert(is_succ_register_timer_at(l2, log_idx));
    let the_rid = get_register_timer_rid(l2[log_idx]);
    assert forall |k: int| log_idx < k < l2.len() implies
      !timer_retired_at(l2, the_rid, k)
    by {
      if k < n {
        assert(l2[k] == l[k]);
        not_timer_retired_preserved(l, e, the_rid, k);
      } else {
        assert(k == n);
        assert(l2[k] == e);
        assert(!is_deregister_timer_at(l2, n));
        assert(!is_wake_task_at(l2, n));
        not_timer_retired(l2, the_rid, k);
      }
    };
    assert(timer_active_at(l2, log_idx, l2.len() as int));
    assert forall |k: int| log_idx < k < l2.len() implies !(
      is_wake_task_at(l2, k) && get_wake_task_source_rid(l2[k]) == the_rid
    ) by {
      not_timer_retired_implies(l2, the_rid, k);
    };
    assert(timer_awaiting_wake(l2, log_idx));
  }
}

pub proof fn timer_wakers_match_preserved_by_append(
  timer_wakers_view: Map<ResourceIdView, WakerView>,
  by_rid_view: Map<ResourceIdView, (InstantView, ResourceIdView, int)>,
  l: Log,
  e: ReactorEvent,
)
  requires
    timer_wakers_match(timer_wakers_view, by_rid_view, l),
  ensures
    timer_wakers_match(timer_wakers_view, by_rid_view, l.push(e)),
{
  let l2 = l.push(e);
  assert forall |rid: ResourceIdView| #![auto]
    timer_wakers_view.contains_key(rid) implies {
      by_rid_view.contains_key(rid) && {
        let log_idx = by_rid_view[rid].2;
        0 <= log_idx < l2.len() &&
        is_succ_register_timer_at(l2, log_idx) &&
        get_register_timer_rid(l2[log_idx]) == rid &&
        timer_wakers_view[rid] == get_register_timer_waker(l2[log_idx])
      }
    }
  by {
    if timer_wakers_view.contains_key(rid) {
      let log_idx = by_rid_view[rid].2;
      assert(log_idx < l.len());
      assert(l2[log_idx] == l[log_idx]);
    }
  }
}

#[verifier::rlimit(30)]
pub proof fn active_timers_in_heap_preserved_by_non_timer_event(
  by_rid_view: Map<ResourceIdView, (InstantView, ResourceIdView, int)>,
  l: Log,
  e: ReactorEvent,
)
  requires
    active_timers_in_heap(by_rid_view, l),
    !is_succ_register_timer_at(l.push(e), l.len() as int),
    !is_deregister_timer_at(l.push(e), l.len() as int),
    !is_wake_task_at(l.push(e), l.len() as int),
  ensures
    active_timers_in_heap(by_rid_view, l.push(e)),
{
  let l2 = l.push(e);
  let n = l.len() as int;
  assert forall |log_idx: int| #![auto] timer_awaiting_wake(l2, log_idx) implies {
    let rid = get_register_timer_rid(l2[log_idx]);
    by_rid_view.contains_key(rid) &&
    by_rid_view[rid].2 == log_idx
  } by {
    assert(log_idx < n);
    assert(l2[log_idx] == l[log_idx]);
    assert(is_succ_register_timer_at(l, log_idx));
    let rid = get_register_timer_rid(l[log_idx]);
    assert forall |k: int| log_idx < k < n implies
      !timer_retired_at(l, rid, k)
    by {
      assert(l2[k] == l[k]);
      not_timer_retired_shrink(l, e, rid, k);
    };
    assert(timer_active_at(l, log_idx, l.len() as int));
    assert forall |k: int| log_idx < k < n implies !(
      is_wake_task_at(l, k) && get_wake_task_source_rid(l[k]) == rid
    ) by {
      not_timer_retired_implies(l, rid, k);
    };
    assert(timer_awaiting_wake(l, log_idx));
  }
}

#[verifier::rlimit(30)]
pub proof fn timer_awaiting_wake_shrink_past_non_matching_wake(
  l: Log,
  e: ReactorEvent,
  log_idx: int,
)
  requires
    timer_awaiting_wake(l.push(e), log_idx),
    is_wake_task_at(l.push(e), l.len() as int),
    get_wake_task_source_rid(e) != get_register_timer_rid(l.push(e)[log_idx]),
  ensures
    timer_awaiting_wake(l, log_idx),
{
  let l2 = l.push(e);
  let n = l.len() as int;
  assert(log_idx < n);
  assert(l2[log_idx] == l[log_idx]);
  assert(is_succ_register_timer_at(l, log_idx));
  let rid = get_register_timer_rid(l[log_idx]);
  assert forall |k: int| log_idx < k < n implies
    !timer_retired_at(l, rid, k)
  by {
    assert(l2[k] == l[k]);
    not_timer_retired_shrink(l, e, rid, k);
  };
  assert(timer_active_at(l, log_idx, l.len() as int));
  assert forall |k: int| log_idx < k < n implies !(
    is_wake_task_at(l, k) && get_wake_task_source_rid(l[k]) == rid
  ) by {
    not_timer_retired_implies(l, rid, k);
  };
}

pub proof fn park_pre_wake_preserved(
  timers_view: Set<(InstantView, ResourceIdView, int)>,
  by_rid_view: Map<ResourceIdView, (InstantView, ResourceIdView, int)>,
  timer_wakers_view: Map<ResourceIdView, WakerView>,
  read_wakers_view: Map<ResourceIdView, WakerView>,
  write_wakers_view: Map<ResourceIdView, WakerView>,
  l: Log,
  e: ReactorEvent,
  next_rid: nat,
)
  requires
    reactor_safety_inv(l),
    alloc_inv(l, next_rid),
    timer_heap_entries_valid(timers_view, l),
    active_timers_in_heap(by_rid_view, l),
    timer_wakers_match(timer_wakers_view, by_rid_view, l),
    timer_heap_has_wakers(timer_wakers_view, by_rid_view),
    read_wakers_complete(read_wakers_view, l),
    write_wakers_complete(write_wakers_view, l),
    !is_succ_register_timer_at(l.push(e), l.len() as int),
    !io_api_registered_at(l.push(e), l.len() as int),
    !is_wake_task_at(l.push(e), l.len() as int),
    !is_deregister_timer_at(l.push(e), l.len() as int),
    !is_succ_set_waker_at(l.push(e), l.len() as int),
  ensures
    reactor_safety_inv(l.push(e)),
    alloc_inv(l.push(e), next_rid),
    timer_heap_entries_valid(timers_view, l.push(e)),
    active_timers_in_heap(by_rid_view, l.push(e)),
    timer_wakers_match(timer_wakers_view, by_rid_view, l.push(e)),
    timer_heap_has_wakers(timer_wakers_view, by_rid_view),
    read_wakers_complete(read_wakers_view, l.push(e)),
    write_wakers_complete(write_wakers_view, l.push(e)),
{
  reactor_safety_inv_preserved_by_non_wake(l, e);
  alloc_inv_preserved_by_non_registration(l, e, next_rid);
  timer_heap_entries_valid_preserved_by_non_timer_event(timers_view, l, e);
  active_timers_in_heap_preserved_by_non_timer_event(by_rid_view, l, e);
  timer_wakers_match_preserved_by_append(timer_wakers_view, by_rid_view, l, e);
  read_wakers_complete_preserved_by_non_trigger(read_wakers_view, l, e);
  write_wakers_complete_preserved_by_non_trigger(write_wakers_view, l, e);
}

#[verifier::rlimit(300)]
pub proof fn reactor_safety_inv_preserved_by_wake(l: Log, e: ReactorEvent)
  requires
    reactor_safety_inv(l),
    is_wake_task_at(l.push(e), l.len() as int),
    !is_succ_register_timer_at(l.push(e), l.len() as int),
    !io_api_registered_at(l.push(e), l.len() as int),
    (exists |j: int| 0 <= j < l.len() &&
      is_succ_register_timer_at(l, j) &&
      get_register_timer_rid(l[j]) == get_wake_task_source_rid(e))
    ||
    (exists |j: int| 0 <= j < l.len() &&
      io_api_registered_at(l, j) &&
      get_io_api_register_rid(l[j]) == get_wake_task_source_rid(e)),
    is_timer_wake_at(l.push(e), l.len() as int) ==> (
      exists |j: int| 0 <= j < l.len() &&
        is_succ_register_timer_at(l, j) &&
        get_register_timer_rid(l[j]) == get_wake_task_source_rid(e) &&
        get_register_timer_waker(l[j]) == get_wake_task_waker(e) &&
        timer_active_at(l, j, l.len() as int)
    ),
    is_io_api_wake_at(l.push(e), l.len() as int) ==> (
      exists |sw_idx: int| 0 <= sw_idx < l.len() &&
        is_succ_set_waker_at(l, sw_idx) &&
        get_set_waker_rid(l[sw_idx]) == get_wake_task_source_rid(e) &&
        get_set_waker_waker(l[sw_idx]) == get_wake_task_waker(e) &&
        io_api_active_at_set_waker(l, get_wake_task_source_rid(e), sw_idx)
    ),
  ensures
    reactor_safety_inv(l.push(e)),
{
  let l2 = l.push(e);
  let n = l.len() as int;

  // R7
  assert forall |i: int| #![auto] is_succ_register_timer_at(l2, i) implies {
    let rid = get_register_timer_rid(l2[i]);
    no_prior_timer_registration(l2, rid, i)
  } by {
    assert(i < n);
    assert(l2[i] == l[i]);
    assert(is_succ_register_timer_at(l, i));
    let rid = get_register_timer_rid(l[i]);
    assert(no_prior_timer_registration(l, rid, i));
    no_prior_timer_reg_preserved(l, e, rid, i);
  }

  // R8
  assert forall |i: int| #![auto] io_api_registered_at(l2, i) implies {
    let rid = get_io_api_register_rid(l2[i]);
    no_prior_io_api_registration(l2, rid, i)
  } by {
    assert(i < n);
    assert(l2[i] == l[i]);
    assert(io_api_registered_at(l, i));
    let rid = get_io_api_register_rid(l[i]);
    assert(no_prior_io_api_registration(l, rid, i));
    no_prior_io_api_reg_preserved(l, e, rid, i);
  }

  // R9a
  assert forall |i: int| #![auto] is_succ_register_timer_at(l2, i) implies {
    let rid = get_register_timer_rid(l2[i]);
    no_io_api_with_rid_before(l2, rid, i)
  } by {
    assert(i < n);
    assert(l2[i] == l[i]);
    assert(is_succ_register_timer_at(l, i));
    let rid = get_register_timer_rid(l[i]);
    assert(no_io_api_with_rid_before(l, rid, i));
    no_io_api_with_rid_preserved(l, e, rid, i);
  }

  // R9b
  assert forall |i: int| #![auto] io_api_registered_at(l2, i) implies {
    let rid = get_io_api_register_rid(l2[i]);
    no_timer_with_rid_before(l2, rid, i)
  } by {
    assert(i < n);
    assert(l2[i] == l[i]);
    assert(io_api_registered_at(l, i));
    let rid = get_io_api_register_rid(l[i]);
    assert(no_timer_with_rid_before(l, rid, i));
    no_timer_with_rid_preserved(l, e, rid, i);
  }

  // R5
  assert forall |i: int| #![auto] is_timer_wake_at(l2, i) implies {
    let rid = get_wake_task_source_rid(l2[i]);
    let waker = get_wake_task_waker(l2[i]);
    exists |j: int| 0 <= j < i &&
      is_succ_register_timer_at(l2, j) &&
      get_register_timer_rid(l2[j]) == rid &&
      get_register_timer_waker(l2[j]) == waker &&
      timer_active_at(l2, j, i)
  } by {
    if i < n {
      assert(l2[i] == l[i]);
      assert(is_wake_task_at(l, i));
      assert(is_timer_wake_at(l2, i));
      let j2 = choose |j2: int| 0 <= j2 < i &&
        is_succ_register_timer_at(l2, j2) &&
        get_register_timer_rid(l2[j2]) == get_wake_task_source_rid(l2[i]) &&
        timer_active_at(l2, j2, i);
      assert(j2 < n);
      assert(l2[j2] == l[j2]);
      assert(is_succ_register_timer_at(l, j2));
      assert(get_register_timer_rid(l[j2]) == get_wake_task_source_rid(l[i]));
      assert(timer_active_at(l2, j2, i));
      let rid2 = get_register_timer_rid(l2[j2]);
      assert forall |k: int| j2 < k < i implies
        !timer_retired_at(l, rid2, k)
      by {
        assert(l2[k] == l[k]);
        not_timer_retired_transfer(l2, l, rid2, k);
      };
      assert(timer_active_at(l, j2, i));
      assert(is_timer_wake_at(l, i));
      let rid = get_wake_task_source_rid(l[i]);
      let waker = get_wake_task_waker(l[i]);
      let j = choose |j: int| 0 <= j < i &&
        is_succ_register_timer_at(l, j) &&
        get_register_timer_rid(l[j]) == rid &&
        get_register_timer_waker(l[j]) == waker &&
        timer_active_at(l, j, i);
      assert(l2[j] == l[j]);
      assert(is_succ_register_timer_at(l2, j));
      assert(get_register_timer_rid(l2[j]) == rid);
      assert(get_register_timer_waker(l2[j]) == waker);
      assert(timer_active_at(l, j, i));
      assert forall |k: int| j < k < i implies
        !timer_retired_at(l2, rid, k)
      by {
        assert(l2[k] == l[k]);
        not_timer_retired_transfer(l, l2, rid, k);
      };
      assert(timer_active_at(l2, j, i));
    } else {
      assert(i == n);
      assert(is_timer_wake_at(l2, n));
      let rid = get_wake_task_source_rid(l2[n]);
      let waker = get_wake_task_waker(l2[n]);
      let j = choose |j: int| 0 <= j < n &&
        is_succ_register_timer_at(l, j) &&
        get_register_timer_rid(l[j]) == rid &&
        get_register_timer_waker(l[j]) == waker &&
        timer_active_at(l, j, n);
      assert(l2[j] == l[j]);
      assert(is_succ_register_timer_at(l2, j));
      assert(get_register_timer_rid(l2[j]) == rid);
      assert(get_register_timer_waker(l2[j]) == waker);
      assert(timer_active_at(l, j, n));
      assert forall |k: int| j < k < n implies
        !timer_retired_at(l2, rid, k)
      by {
        assert(l2[k] == l[k]);
        not_timer_retired_transfer(l, l2, rid, k);
      };
    }
  }

  // R6
  assert forall |i: int| #![auto] is_io_api_wake_at(l2, i) implies {
    let rid = get_wake_task_source_rid(l2[i]);
    let waker = get_wake_task_waker(l2[i]);
    exists |sw_idx: int| 0 <= sw_idx < i &&
      is_succ_set_waker_at(l2, sw_idx) &&
      get_set_waker_rid(l2[sw_idx]) == rid &&
      get_set_waker_waker(l2[sw_idx]) == waker &&
      io_api_active_at_set_waker(l2, rid, sw_idx)
  } by {
    if i < n {
      assert(l2[i] == l[i]);
      assert(is_wake_task_at(l, i));
      assert(is_io_api_wake_at(l2, i));
      let j_io = choose |j_io: int| 0 <= j_io < i &&
        io_api_registered_at(l2, j_io) &&
        get_io_api_register_rid(l2[j_io]) == get_wake_task_source_rid(l2[i]) &&
        io_api_active_at(l2, j_io, i);
      assert(j_io < n);
      assert(l2[j_io] == l[j_io]);
      assert(io_api_registered_at(l, j_io));
      assert(get_io_api_register_rid(l[j_io]) == get_wake_task_source_rid(l[i]));
      assert(io_api_active_at(l2, j_io, i));
      assert forall |k: int| j_io < k < i implies !(
        io_api_deregistered_at(l, k) &&
        get_io_api_deregister_rid(l[k]) == get_io_api_register_rid(l[j_io])
      ) by {
        assert(l2[k] == l[k]);
        assert(!(io_api_deregistered_at(l2, k) &&
          get_io_api_deregister_rid(l2[k]) == get_io_api_register_rid(l2[j_io])));
      }
      assert(io_api_active_at(l, j_io, i));
      assert(is_io_api_wake_at(l, i));
      let rid = get_wake_task_source_rid(l[i]);
      let waker = get_wake_task_waker(l[i]);
      let sw_idx = choose |sw_idx: int| 0 <= sw_idx < i &&
        is_succ_set_waker_at(l, sw_idx) &&
        get_set_waker_rid(l[sw_idx]) == rid &&
        get_set_waker_waker(l[sw_idx]) == waker &&
        io_api_active_at_set_waker(l, rid, sw_idx);
      assert(l2[sw_idx] == l[sw_idx]);
      let reg_idx = choose |reg_idx: int| 0 <= reg_idx < sw_idx &&
        io_api_registered_at(l, reg_idx) &&
        get_io_api_register_rid(l[reg_idx]) == rid &&
        io_api_active_at(l, reg_idx, sw_idx);
      assert(l2[reg_idx] == l[reg_idx]);
      assert(io_api_active_at(l, reg_idx, sw_idx));
      assert forall |k: int| reg_idx < k < sw_idx implies !(
        io_api_deregistered_at(l2, k) &&
        get_io_api_deregister_rid(l2[k]) == get_io_api_register_rid(l2[reg_idx])
      ) by {
        assert(l2[k] == l[k]);
        assert(!(io_api_deregistered_at(l, k) &&
          get_io_api_deregister_rid(l[k]) == get_io_api_register_rid(l[reg_idx])));
      }
      assert(io_api_registered_at(l2, reg_idx));
      assert(get_io_api_register_rid(l2[reg_idx]) == rid);
      assert(io_api_active_at(l2, reg_idx, sw_idx));
      assert(io_api_active_at_set_waker(l2, rid, sw_idx));
      assert(is_succ_set_waker_at(l2, sw_idx));
      assert(get_set_waker_rid(l2[sw_idx]) == get_wake_task_source_rid(l2[i]));
      assert(get_set_waker_waker(l2[sw_idx]) == get_wake_task_waker(l2[i]));
      assert(io_api_active_at_set_waker(l2, get_wake_task_source_rid(l2[i]), sw_idx));
    } else {
      assert(i == n);
      assert(is_io_api_wake_at(l2, n));
      let rid = get_wake_task_source_rid(l2[n]);
      let waker = get_wake_task_waker(l2[n]);
      let sw_idx = choose |sw_idx: int| 0 <= sw_idx < n &&
        is_succ_set_waker_at(l, sw_idx) &&
        get_set_waker_rid(l[sw_idx]) == rid &&
        get_set_waker_waker(l[sw_idx]) == waker &&
        io_api_active_at_set_waker(l, rid, sw_idx);
      assert(l2[sw_idx] == l[sw_idx]);
      let reg_idx = choose |reg_idx: int| 0 <= reg_idx < sw_idx &&
        io_api_registered_at(l, reg_idx) &&
        get_io_api_register_rid(l[reg_idx]) == rid &&
        io_api_active_at(l, reg_idx, sw_idx);
      assert(l2[reg_idx] == l[reg_idx]);
      assert forall |k: int| reg_idx < k < sw_idx implies !(
        io_api_deregistered_at(l2, k) &&
        get_io_api_deregister_rid(l2[k]) == get_io_api_register_rid(l2[reg_idx])
      ) by {
        assert(l2[k] == l[k]);
        assert(!(io_api_deregistered_at(l, k) &&
          get_io_api_deregister_rid(l[k]) == get_io_api_register_rid(l[reg_idx])));
      }
      assert(io_api_registered_at(l2, reg_idx));
      assert(get_io_api_register_rid(l2[reg_idx]) == rid);
      assert(io_api_active_at(l2, reg_idx, sw_idx));
      assert(io_api_active_at_set_waker(l2, rid, sw_idx));
      assert(is_succ_set_waker_at(l2, sw_idx));
      assert(get_set_waker_rid(l2[sw_idx]) == get_wake_task_source_rid(l2[n]));
      assert(get_set_waker_waker(l2[sw_idx]) == get_wake_task_waker(l2[n]));
      assert(io_api_active_at_set_waker(l2, get_wake_task_source_rid(l2[n]), sw_idx));
    }
  }

  // R14
  assert forall |i: int| #![auto] is_wake_task_at(l2, i) implies {
    let rid = get_wake_task_source_rid(l2[i]);
    (exists |j: int| 0 <= j < i &&
      is_succ_register_timer_at(l2, j) &&
      get_register_timer_rid(l2[j]) == rid)
    ||
    (exists |j: int| 0 <= j < i &&
      io_api_registered_at(l2, j) &&
      get_io_api_register_rid(l2[j]) == rid)
  } by {
    if i < n {
      assert(l2[i] == l[i]);
      assert(is_wake_task_at(l, i));
      let rid = get_wake_task_source_rid(l[i]);
      if exists |j: int| 0 <= j < i &&
        is_succ_register_timer_at(l, j) &&
        get_register_timer_rid(l[j]) == rid
      {
        let j = choose |j: int| 0 <= j < i &&
          is_succ_register_timer_at(l, j) &&
          get_register_timer_rid(l[j]) == rid;
        assert(l2[j] == l[j]);
        assert(is_succ_register_timer_at(l2, j));
        assert(get_register_timer_rid(l2[j]) == get_wake_task_source_rid(l2[i]));
      } else {
        let j = choose |j: int| 0 <= j < i &&
          io_api_registered_at(l, j) &&
          get_io_api_register_rid(l[j]) == rid;
        assert(l2[j] == l[j]);
        assert(io_api_registered_at(l2, j));
        assert(get_io_api_register_rid(l2[j]) == get_wake_task_source_rid(l2[i]));
      }
    } else {
      assert(i == n);
      let rid = get_wake_task_source_rid(l2[n]);
      if exists |j: int| 0 <= j < n &&
        is_succ_register_timer_at(l, j) &&
        get_register_timer_rid(l[j]) == rid
      {
        let j = choose |j: int| 0 <= j < n &&
          is_succ_register_timer_at(l, j) &&
          get_register_timer_rid(l[j]) == rid;
        assert(l2[j] == l[j]);
        assert(is_succ_register_timer_at(l2, j));
        assert(get_register_timer_rid(l2[j]) == rid);
      } else {
        let j = choose |j: int| 0 <= j < n &&
          io_api_registered_at(l, j) &&
          get_io_api_register_rid(l[j]) == rid;
        assert(l2[j] == l[j]);
        assert(io_api_registered_at(l2, j));
        assert(get_io_api_register_rid(l2[j]) == rid);
      }
    }
  }
}

#[verifier::rlimit(50)]
pub proof fn read_wakers_valid_preserved_by_non_set_waker(
  read_wakers_view: Map<ResourceIdView, WakerView>,
  l: Log,
  e: ReactorEvent,
)
  requires
    read_wakers_valid(read_wakers_view, l),
    !is_succ_set_waker_at(l.push(e), l.len() as int),
    !io_api_deregistered_at(l.push(e), l.len() as int),
  ensures
    read_wakers_valid(read_wakers_view, l.push(e)),
{
  let l2 = l.push(e);
  let n = l.len() as int;
  assert forall |rid: ResourceIdView| #![auto]
    read_wakers_view.contains_key(rid) implies
    io_currently_active(l2, rid) &&
    exists |sw_idx: int| 0 <= sw_idx < l2.len() &&
      is_succ_set_waker_at(l2, sw_idx) &&
      get_set_waker_rid(l2[sw_idx]) == rid &&
      get_set_waker_interest(l2[sw_idx]).0 &&
      get_set_waker_waker(l2[sw_idx]) == read_wakers_view[rid] &&
      io_api_active_at_set_waker(l2, rid, sw_idx) &&
      forall |k: int| sw_idx < k < l2.len() ==> !(
        is_succ_set_waker_at(l2, k) &&
        get_set_waker_rid(l2[k]) == rid &&
        get_set_waker_interest(l2[k]).0
      )
  by {
    assert(io_currently_active(l, rid));
    let reg_idx_outer = choose |reg_idx: int| 0 <= reg_idx < l.len() &&
      io_api_registered_at(l, reg_idx) &&
      get_io_api_register_rid(l[reg_idx]) == rid &&
      io_api_active_at(l, reg_idx, l.len() as int);
    assert(l2[reg_idx_outer] == l[reg_idx_outer]);
    assert(io_api_registered_at(l2, reg_idx_outer));
    assert(get_io_api_register_rid(l2[reg_idx_outer]) == rid);
    assert forall |k: int| reg_idx_outer < k < l2.len() as int implies !(
      io_api_deregistered_at(l2, k) &&
      get_io_api_deregister_rid(l2[k]) == get_io_api_register_rid(l2[reg_idx_outer])
    ) by {
      if k < n {
        assert(l2[k] == l[k]);
        assert(!(io_api_deregistered_at(l, k) &&
          get_io_api_deregister_rid(l[k]) == get_io_api_register_rid(l[reg_idx_outer])));
      } else {
        assert(!io_api_deregistered_at(l2, k));
      }
    }
    assert(io_api_active_at(l2, reg_idx_outer, l2.len() as int));
    assert(io_currently_active(l2, rid));

    let sw_idx = choose |sw_idx: int| 0 <= sw_idx < l.len() &&
      is_succ_set_waker_at(l, sw_idx) &&
      get_set_waker_rid(l[sw_idx]) == rid &&
      get_set_waker_interest(l[sw_idx]).0 &&
      get_set_waker_waker(l[sw_idx]) == read_wakers_view[rid] &&
      io_api_active_at_set_waker(l, rid, sw_idx) &&
      forall |k: int| sw_idx < k < l.len() ==> !(
        is_succ_set_waker_at(l, k) &&
        get_set_waker_rid(l[k]) == rid &&
        get_set_waker_interest(l[k]).0
      );
    assert(l2[sw_idx] == l[sw_idx]);
    let reg_idx = choose |reg_idx: int| 0 <= reg_idx < sw_idx &&
      io_api_registered_at(l, reg_idx) &&
      get_io_api_register_rid(l[reg_idx]) == rid &&
      io_api_active_at(l, reg_idx, sw_idx);
    assert(l2[reg_idx] == l[reg_idx]);
    assert(io_api_registered_at(l2, reg_idx));
    assert(get_io_api_register_rid(l2[reg_idx]) == rid);
    assert forall |k: int| reg_idx < k < sw_idx implies !(
      io_api_deregistered_at(l2, k) &&
      get_io_api_deregister_rid(l2[k]) == get_io_api_register_rid(l2[reg_idx])
    ) by {
      assert(l2[k] == l[k]);
      assert(!(io_api_deregistered_at(l, k) &&
        get_io_api_deregister_rid(l[k]) == get_io_api_register_rid(l[reg_idx])));
    }
    assert(io_api_active_at(l2, reg_idx, sw_idx));
    assert(io_api_active_at_set_waker(l2, rid, sw_idx));
    assert(is_succ_set_waker_at(l2, sw_idx));
    assert(get_set_waker_rid(l2[sw_idx]) == rid);
    assert(get_set_waker_interest(l2[sw_idx]).0);
    assert(get_set_waker_waker(l2[sw_idx]) == read_wakers_view[rid]);
    assert forall |k: int| sw_idx < k < l2.len() implies !(
      is_succ_set_waker_at(l2, k) &&
      get_set_waker_rid(l2[k]) == rid &&
      get_set_waker_interest(l2[k]).0
    ) by {
      if k < n {
        assert(l2[k] == l[k]);
        assert(!(is_succ_set_waker_at(l, k) &&
          get_set_waker_rid(l[k]) == rid &&
          get_set_waker_interest(l[k]).0));
      }
    }
  }
}

#[verifier::rlimit(50)]
pub proof fn write_wakers_valid_preserved_by_non_set_waker(
  write_wakers_view: Map<ResourceIdView, WakerView>,
  l: Log,
  e: ReactorEvent,
)
  requires
    write_wakers_valid(write_wakers_view, l),
    !is_succ_set_waker_at(l.push(e), l.len() as int),
    !io_api_deregistered_at(l.push(e), l.len() as int),
  ensures
    write_wakers_valid(write_wakers_view, l.push(e)),
{
  let l2 = l.push(e);
  let n = l.len() as int;
  assert forall |rid: ResourceIdView| #![auto]
    write_wakers_view.contains_key(rid) implies
    io_currently_active(l2, rid) &&
    exists |sw_idx: int| 0 <= sw_idx < l2.len() &&
      is_succ_set_waker_at(l2, sw_idx) &&
      get_set_waker_rid(l2[sw_idx]) == rid &&
      get_set_waker_interest(l2[sw_idx]).1 &&
      get_set_waker_waker(l2[sw_idx]) == write_wakers_view[rid] &&
      io_api_active_at_set_waker(l2, rid, sw_idx) &&
      forall |k: int| sw_idx < k < l2.len() ==> !(
        is_succ_set_waker_at(l2, k) &&
        get_set_waker_rid(l2[k]) == rid &&
        get_set_waker_interest(l2[k]).1
      )
  by {
    assert(io_currently_active(l, rid));
    let reg_idx_outer = choose |reg_idx: int| 0 <= reg_idx < l.len() &&
      io_api_registered_at(l, reg_idx) &&
      get_io_api_register_rid(l[reg_idx]) == rid &&
      io_api_active_at(l, reg_idx, l.len() as int);
    assert(l2[reg_idx_outer] == l[reg_idx_outer]);
    assert(io_api_registered_at(l2, reg_idx_outer));
    assert(get_io_api_register_rid(l2[reg_idx_outer]) == rid);
    assert forall |k: int| reg_idx_outer < k < l2.len() as int implies !(
      io_api_deregistered_at(l2, k) &&
      get_io_api_deregister_rid(l2[k]) == get_io_api_register_rid(l2[reg_idx_outer])
    ) by {
      if k < n {
        assert(l2[k] == l[k]);
        assert(!(io_api_deregistered_at(l, k) &&
          get_io_api_deregister_rid(l[k]) == get_io_api_register_rid(l[reg_idx_outer])));
      } else {
        assert(!io_api_deregistered_at(l2, k));
      }
    }
    assert(io_api_active_at(l2, reg_idx_outer, l2.len() as int));
    assert(io_currently_active(l2, rid));

    let sw_idx = choose |sw_idx: int| 0 <= sw_idx < l.len() &&
      is_succ_set_waker_at(l, sw_idx) &&
      get_set_waker_rid(l[sw_idx]) == rid &&
      get_set_waker_interest(l[sw_idx]).1 &&
      get_set_waker_waker(l[sw_idx]) == write_wakers_view[rid] &&
      io_api_active_at_set_waker(l, rid, sw_idx) &&
      forall |k: int| sw_idx < k < l.len() ==> !(
        is_succ_set_waker_at(l, k) &&
        get_set_waker_rid(l[k]) == rid &&
        get_set_waker_interest(l[k]).1
      );
    assert(l2[sw_idx] == l[sw_idx]);
    let reg_idx = choose |reg_idx: int| 0 <= reg_idx < sw_idx &&
      io_api_registered_at(l, reg_idx) &&
      get_io_api_register_rid(l[reg_idx]) == rid &&
      io_api_active_at(l, reg_idx, sw_idx);
    assert(l2[reg_idx] == l[reg_idx]);
    assert(io_api_registered_at(l2, reg_idx));
    assert(get_io_api_register_rid(l2[reg_idx]) == rid);
    assert forall |k: int| reg_idx < k < sw_idx implies !(
      io_api_deregistered_at(l2, k) &&
      get_io_api_deregister_rid(l2[k]) == get_io_api_register_rid(l2[reg_idx])
    ) by {
      assert(l2[k] == l[k]);
      assert(!(io_api_deregistered_at(l, k) &&
        get_io_api_deregister_rid(l[k]) == get_io_api_register_rid(l[reg_idx])));
    }
    assert(io_api_active_at(l2, reg_idx, sw_idx));
    assert(io_api_active_at_set_waker(l2, rid, sw_idx));
    assert(is_succ_set_waker_at(l2, sw_idx));
    assert(get_set_waker_rid(l2[sw_idx]) == rid);
    assert(get_set_waker_interest(l2[sw_idx]).1);
    assert(get_set_waker_waker(l2[sw_idx]) == write_wakers_view[rid]);
    assert forall |k: int| sw_idx < k < l2.len() implies !(
      is_succ_set_waker_at(l2, k) &&
      get_set_waker_rid(l2[k]) == rid &&
      get_set_waker_interest(l2[k]).1
    ) by {
      if k < n {
        assert(l2[k] == l[k]);
        assert(!(is_succ_set_waker_at(l, k) &&
          get_set_waker_rid(l[k]) == rid &&
          get_set_waker_interest(l[k]).1));
      }
    }
  }
}

#[verifier::rlimit(80)]
pub proof fn reactor_safety_inv_preserved_by_io_read_wake(
  l: Log,
  e: ReactorEvent,
  read_wakers_view: Map<ResourceIdView, WakerView>,
  rid: ResourceIdView,
)
  requires
    reactor_safety_inv(l),
    read_wakers_valid(read_wakers_view, l),
    is_wake_task_at(l.push(e), l.len() as int),
    get_wake_task_source_rid(e) == rid,
    read_wakers_view.contains_key(rid),
    read_wakers_view[rid] == get_wake_task_waker(e),
    io_currently_active(l, rid),
  ensures
    reactor_safety_inv(l.push(e)),
{
  let l2 = l.push(e);
  let n = l.len() as int;

  let sw_idx = choose |sw_idx: int| 0 <= sw_idx < n &&
    is_succ_set_waker_at(l, sw_idx) &&
    get_set_waker_rid(l[sw_idx]) == rid &&
    get_set_waker_interest(l[sw_idx]).0 &&
    get_set_waker_waker(l[sw_idx]) == read_wakers_view[rid] &&
    io_api_active_at_set_waker(l, rid, sw_idx);

  let active_reg = choose |reg_idx: int| 0 <= reg_idx < n &&
    io_api_registered_at(l, reg_idx) &&
    get_io_api_register_rid(l[reg_idx]) == rid &&
    io_api_active_at(l, reg_idx, n);

  let reg_idx = choose |reg_idx: int| 0 <= reg_idx < sw_idx &&
    io_api_registered_at(l, reg_idx) &&
    get_io_api_register_rid(l[reg_idx]) == rid &&
    io_api_active_at(l, reg_idx, sw_idx);

  assert(!is_succ_register_timer_at(l2, n));
  assert(!io_api_registered_at(l2, n));

  assert(io_api_registered_at(l, reg_idx));
  assert(get_io_api_register_rid(l[reg_idx]) == get_wake_task_source_rid(e));

  assert(!is_timer_wake_at(l2, n)) by {
    if is_timer_wake_at(l2, n) {
      let j = choose |j: int| 0 <= j < n &&
        is_succ_register_timer_at(l2, j) &&
        get_register_timer_rid(l2[j]) == rid &&
        timer_active_at(l2, j, n);
      assert(l2[j] == l[j]);
      assert(is_succ_register_timer_at(l, j));
      assert(get_register_timer_rid(l[j]) == rid);
      if j < active_reg {
        assert(no_timer_with_rid_before(l, rid, active_reg));
        let d = choose |d: int| j < d < active_reg && timer_retired_at(l, rid, d);
        timer_retired_preserved(l, e, rid, d);
      } else {
        assert(no_io_api_with_rid_before(l, rid, j));
        let d = choose |d: int| active_reg < d < j &&
          io_api_deregistered_at(l, d) && get_io_api_deregister_rid(l[d]) == rid;
        assert(active_reg < d && d < n);
        assert(io_api_active_at(l, active_reg, n));
      }
    }
  }

  assert(is_succ_set_waker_at(l, sw_idx));
  assert(get_set_waker_rid(l[sw_idx]) == get_wake_task_source_rid(e));
  assert(get_set_waker_waker(l[sw_idx]) == get_wake_task_waker(e));
  assert(io_api_active_at_set_waker(l, get_wake_task_source_rid(e), sw_idx));

  reactor_safety_inv_preserved_by_wake(l, e);
}

#[verifier::rlimit(80)]
pub proof fn reactor_safety_inv_preserved_by_io_write_wake(
  l: Log,
  e: ReactorEvent,
  write_wakers_view: Map<ResourceIdView, WakerView>,
  rid: ResourceIdView,
)
  requires
    reactor_safety_inv(l),
    write_wakers_valid(write_wakers_view, l),
    is_wake_task_at(l.push(e), l.len() as int),
    get_wake_task_source_rid(e) == rid,
    write_wakers_view.contains_key(rid),
    write_wakers_view[rid] == get_wake_task_waker(e),
    io_currently_active(l, rid),
  ensures
    reactor_safety_inv(l.push(e)),
{
  let l2 = l.push(e);
  let n = l.len() as int;

  let sw_idx = choose |sw_idx: int| 0 <= sw_idx < n &&
    is_succ_set_waker_at(l, sw_idx) &&
    get_set_waker_rid(l[sw_idx]) == rid &&
    get_set_waker_interest(l[sw_idx]).1 &&
    get_set_waker_waker(l[sw_idx]) == write_wakers_view[rid] &&
    io_api_active_at_set_waker(l, rid, sw_idx);

  let active_reg = choose |reg_idx: int| 0 <= reg_idx < n &&
    io_api_registered_at(l, reg_idx) &&
    get_io_api_register_rid(l[reg_idx]) == rid &&
    io_api_active_at(l, reg_idx, n);

  let reg_idx = choose |reg_idx: int| 0 <= reg_idx < sw_idx &&
    io_api_registered_at(l, reg_idx) &&
    get_io_api_register_rid(l[reg_idx]) == rid &&
    io_api_active_at(l, reg_idx, sw_idx);

  assert(!is_succ_register_timer_at(l2, n));
  assert(!io_api_registered_at(l2, n));

  assert(io_api_registered_at(l, reg_idx));
  assert(get_io_api_register_rid(l[reg_idx]) == get_wake_task_source_rid(e));

  assert(!is_timer_wake_at(l2, n)) by {
    if is_timer_wake_at(l2, n) {
      let j = choose |j: int| 0 <= j < n &&
        is_succ_register_timer_at(l2, j) &&
        get_register_timer_rid(l2[j]) == rid &&
        timer_active_at(l2, j, n);
      assert(l2[j] == l[j]);
      assert(is_succ_register_timer_at(l, j));
      assert(get_register_timer_rid(l[j]) == rid);
      if j < active_reg {
        assert(no_timer_with_rid_before(l, rid, active_reg));
        let d = choose |d: int| j < d < active_reg && timer_retired_at(l, rid, d);
        timer_retired_preserved(l, e, rid, d);
      } else {
        assert(no_io_api_with_rid_before(l, rid, j));
        let d = choose |d: int| active_reg < d < j &&
          io_api_deregistered_at(l, d) && get_io_api_deregister_rid(l[d]) == rid;
        assert(active_reg < d && d < n);
        assert(io_api_active_at(l, active_reg, n));
      }
    }
  }

  assert(is_succ_set_waker_at(l, sw_idx));
  assert(get_set_waker_rid(l[sw_idx]) == get_wake_task_source_rid(e));
  assert(get_set_waker_waker(l[sw_idx]) == get_wake_task_waker(e));
  assert(io_api_active_at_set_waker(l, get_wake_task_source_rid(e), sw_idx));

  reactor_safety_inv_preserved_by_wake(l, e);
}

#[verifier::rlimit(80)]
pub proof fn reactor_safety_inv_preserved_by_timer_wake(
  l: Log,
  e: ReactorEvent,
  timer_reg_idx: int,
)
  requires
    reactor_safety_inv(l),
    is_wake_task_at(l.push(e), l.len() as int),
    0 <= timer_reg_idx < l.len(),
    is_succ_register_timer_at(l, timer_reg_idx),
    get_register_timer_rid(l[timer_reg_idx]) == get_wake_task_source_rid(e),
    get_register_timer_waker(l[timer_reg_idx]) == get_wake_task_waker(e),
    timer_active_at(l, timer_reg_idx, l.len() as int),
  ensures
    reactor_safety_inv(l.push(e)),
{
  let l2 = l.push(e);
  let n = l.len() as int;
  let rid = get_wake_task_source_rid(e);

  assert(!is_succ_register_timer_at(l2, n));
  assert(!io_api_registered_at(l2, n));

  assert(is_succ_register_timer_at(l, timer_reg_idx));
  assert(get_register_timer_rid(l[timer_reg_idx]) == rid);
  assert(get_register_timer_waker(l[timer_reg_idx]) == get_wake_task_waker(e));
  assert(timer_active_at(l, timer_reg_idx, n));

  assert(!is_io_api_wake_at(l2, n)) by {
    if is_io_api_wake_at(l2, n) {
      let j = choose |j: int| 0 <= j < n &&
        io_api_registered_at(l2, j) &&
        get_io_api_register_rid(l2[j]) == rid &&
        io_api_active_at(l2, j, n);
      assert(l2[j] == l[j]);
      assert(io_api_registered_at(l, j));
      assert(get_io_api_register_rid(l[j]) == rid);
      if j < timer_reg_idx {
        assert(no_io_api_with_rid_before(l, rid, timer_reg_idx));
        assert(io_api_registered_at(l, j) && get_io_api_register_rid(l[j]) == rid && 0 <= j && j < timer_reg_idx);
        let d = choose |d: int| j < d < timer_reg_idx &&
          io_api_deregistered_at(l, d) && get_io_api_deregister_rid(l[d]) == rid;
        assert(l2[d] == l[d]);
        assert(io_api_deregistered_at(l2, d) && get_io_api_deregister_rid(l2[d]) == get_io_api_register_rid(l2[j]));
        assert(io_api_active_at(l2, j, n));
        assert(false);
      } else {
        assert(no_timer_with_rid_before(l, rid, j));
        let d = choose |d: int| timer_reg_idx < d < j && timer_retired_at(l, rid, d);
        assert(timer_reg_idx < d && d < n);
        assert(timer_active_at(l, timer_reg_idx, n));
      }
    }
  }

  reactor_safety_inv_preserved_by_wake(l, e);
}

#[verifier::rlimit(30)]
pub proof fn timer_heap_no_duplicate_rid(
  timers_view: Set<(InstantView, ResourceIdView, int)>,
  log: Log,
  d: InstantView,
  rid: ResourceIdView,
  log_idx: int,
)
  requires
    timer_heap_entries_valid(timers_view, log),
    reactor_safety_inv(log),
    timers_view.contains((d, rid, log_idx)),
  ensures
    forall |d2: InstantView, log_idx2: int|
      #![auto] timers_view.contains((d2, rid, log_idx2)) ==> d2 == d && log_idx2 == log_idx,
{
  assert forall |d2: InstantView, log_idx2: int|
    #![auto] timers_view.contains((d2, rid, log_idx2)) implies d2 == d && log_idx2 == log_idx
  by {
    assert(timer_awaiting_wake(log, log_idx));
    assert(timer_awaiting_wake(log, log_idx2));
    assert(is_succ_register_timer_at(log, log_idx));
    assert(is_succ_register_timer_at(log, log_idx2));
    assert(get_register_timer_rid(log[log_idx]) == rid);
    assert(get_register_timer_rid(log[log_idx2]) == rid);
    if log_idx < log_idx2 {
      assert(no_prior_timer_registration(log, rid, log_idx2));
      let dd = choose |dd: int| log_idx < dd < log_idx2 && timer_retired_at(log, rid, dd);
      timer_retired_implies(log, rid, dd);
      assert(timer_active_at(log, log_idx, log.len() as int));
      assert(log_idx < dd && dd < log.len());
      assert(false);
    } else if log_idx2 < log_idx {
      assert(no_prior_timer_registration(log, rid, log_idx));
      let dd = choose |dd: int| log_idx2 < dd < log_idx && timer_retired_at(log, rid, dd);
      timer_retired_implies(log, rid, dd);
      assert(timer_active_at(log, log_idx2, log.len() as int));
      assert(log_idx2 < dd && dd < log.len());
      assert(false);
    } else {
      assert(log_idx == log_idx2);
      assert(d == get_register_timer_deadline(log[log_idx]));
      assert(d2 == get_register_timer_deadline(log[log_idx2]));
    }
  }
}

#[verifier::rlimit(50)]
pub proof fn timer_heap_entries_valid_preserved_by_wake_of_nonmember_rid(
  timers_view: Set<(InstantView, ResourceIdView, int)>,
  l: Log,
  e: ReactorEvent,
  waked_rid: ResourceIdView,
)
  requires
    timer_heap_entries_valid(timers_view, l),
    is_wake_task_at(l.push(e), l.len() as int),
    get_wake_task_source_rid(e) == waked_rid,
    !is_deregister_timer_at(l.push(e), l.len() as int),
    forall |d: InstantView, log_idx: int|
      #![auto] timers_view.contains((d, waked_rid, log_idx)) ==> false,
  ensures
    timer_heap_entries_valid(timers_view, l.push(e)),
{
  let l2 = l.push(e);
  let n = l.len() as int;
  assert forall |d: InstantView, rid: ResourceIdView, log_idx: int|
    #![auto] timers_view.contains((d, rid, log_idx)) implies {
      timer_awaiting_wake(l2, log_idx) &&
      get_register_timer_rid(l2[log_idx]) == rid &&
      get_register_timer_deadline(l2[log_idx]) == d
    }
  by {
    assert(timer_awaiting_wake(l, log_idx));
    assert(log_idx < n);
    assert(l2[log_idx] == l[log_idx]);
    assert(is_succ_register_timer_at(l2, log_idx));
    assert(rid != waked_rid);
    let the_rid = get_register_timer_rid(l2[log_idx]);
    assert forall |k: int| log_idx < k < l2.len() implies
      !timer_retired_at(l2, the_rid, k)
    by {
      if k < n {
        assert(l2[k] == l[k]);
        not_timer_retired_preserved(l, e, the_rid, k);
      } else {
        assert(k == n);
        assert(!is_deregister_timer_at(l2, n));
        assert(get_wake_task_source_rid(l2[n]) == waked_rid);
        assert(the_rid == rid);
        assert(rid != waked_rid);
        not_timer_retired(l2, the_rid, k);
      }
    };
    assert(timer_active_at(l2, log_idx, l2.len() as int));
    assert forall |k: int| log_idx < k < l2.len() implies !(
      is_wake_task_at(l2, k) && get_wake_task_source_rid(l2[k]) == the_rid
    ) by {
      not_timer_retired_implies(l2, the_rid, k);
    };
    assert(timer_awaiting_wake(l2, log_idx));
  }
}

#[verifier::rlimit(30)]
pub proof fn read_wakers_complete_preserved_by_non_trigger(
  read_wakers_view: Map<ResourceIdView, WakerView>,
  l: Log,
  e: ReactorEvent,
)
  requires
    read_wakers_complete(read_wakers_view, l),
    !io_api_registered_at(l.push(e), l.len() as int),
    !is_succ_set_waker_at(l.push(e), l.len() as int),
  ensures
    read_wakers_complete(read_wakers_view, l.push(e)),
{
  let l2 = l.push(e);
  let n = l.len() as int;
  assert forall |rid: ResourceIdView| #![auto]
    has_active_readable_set_waker(l2, rid) implies
    read_wakers_view.contains_key(rid)
  by {
    let reg_idx = choose |reg_idx: int| 0 <= reg_idx < l2.len() &&
      io_api_registered_at(l2, reg_idx) &&
      get_io_api_register_rid(l2[reg_idx]) == rid &&
      io_api_active_at(l2, reg_idx, l2.len() as int) &&
      exists |sw_idx: int| reg_idx < sw_idx < l2.len() &&
        is_succ_set_waker_at(l2, sw_idx) &&
        get_set_waker_rid(l2[sw_idx]) == rid &&
        get_set_waker_interest(l2[sw_idx]).0;
    let sw_idx = choose |sw_idx: int| reg_idx < sw_idx < l2.len() &&
      is_succ_set_waker_at(l2, sw_idx) &&
      get_set_waker_rid(l2[sw_idx]) == rid &&
      get_set_waker_interest(l2[sw_idx]).0;
    assert(reg_idx < n);
    assert(sw_idx < n);
    assert(l2[reg_idx] == l[reg_idx]);
    assert(l2[sw_idx] == l[sw_idx]);
    assert forall |k: int| reg_idx < k < n implies !(
      io_api_deregistered_at(l, k) &&
      get_io_api_deregister_rid(l[k]) == get_io_api_register_rid(l[reg_idx])
    ) by {
      assert(l2[k] == l[k]);
      assert(!(io_api_deregistered_at(l2, k) &&
        get_io_api_deregister_rid(l2[k]) == get_io_api_register_rid(l2[reg_idx])));
    }
    assert(io_api_active_at(l, reg_idx, n));
    assert(is_succ_set_waker_at(l, sw_idx));
    assert(get_set_waker_rid(l[sw_idx]) == rid);
    assert(get_set_waker_interest(l[sw_idx]).0);
    assert(reg_idx < sw_idx && sw_idx < l.len());
    assert(has_active_readable_set_waker(l, rid));
  }
}

#[verifier::rlimit(30)]
pub proof fn write_wakers_complete_preserved_by_non_trigger(
  write_wakers_view: Map<ResourceIdView, WakerView>,
  l: Log,
  e: ReactorEvent,
)
  requires
    write_wakers_complete(write_wakers_view, l),
    !io_api_registered_at(l.push(e), l.len() as int),
    !is_succ_set_waker_at(l.push(e), l.len() as int),
  ensures
    write_wakers_complete(write_wakers_view, l.push(e)),
{
  let l2 = l.push(e);
  let n = l.len() as int;
  assert forall |rid: ResourceIdView| #![auto]
    has_active_writable_set_waker(l2, rid) implies
    write_wakers_view.contains_key(rid)
  by {
    let reg_idx = choose |reg_idx: int| 0 <= reg_idx < l2.len() &&
      io_api_registered_at(l2, reg_idx) &&
      get_io_api_register_rid(l2[reg_idx]) == rid &&
      io_api_active_at(l2, reg_idx, l2.len() as int) &&
      exists |sw_idx: int| reg_idx < sw_idx < l2.len() &&
        is_succ_set_waker_at(l2, sw_idx) &&
        get_set_waker_rid(l2[sw_idx]) == rid &&
        get_set_waker_interest(l2[sw_idx]).1;
    let sw_idx = choose |sw_idx: int| reg_idx < sw_idx < l2.len() &&
      is_succ_set_waker_at(l2, sw_idx) &&
      get_set_waker_rid(l2[sw_idx]) == rid &&
      get_set_waker_interest(l2[sw_idx]).1;
    assert(reg_idx < n);
    assert(sw_idx < n);
    assert(l2[reg_idx] == l[reg_idx]);
    assert(l2[sw_idx] == l[sw_idx]);
    assert forall |k: int| reg_idx < k < n implies !(
      io_api_deregistered_at(l, k) &&
      get_io_api_deregister_rid(l[k]) == get_io_api_register_rid(l[reg_idx])
    ) by {
      assert(l2[k] == l[k]);
      assert(!(io_api_deregistered_at(l2, k) &&
        get_io_api_deregister_rid(l2[k]) == get_io_api_register_rid(l2[reg_idx])));
    }
    assert(io_api_active_at(l, reg_idx, n));
    assert(is_succ_set_waker_at(l, sw_idx));
    assert(get_set_waker_rid(l[sw_idx]) == rid);
    assert(get_set_waker_interest(l[sw_idx]).1);
    assert(reg_idx < sw_idx && sw_idx < l.len());
    assert(has_active_writable_set_waker(l, rid));
  }
}

#[verifier::rlimit(30)]
pub proof fn timer_heap_no_io_rid(
  timers_view: Set<(InstantView, ResourceIdView, int)>,
  l: Log,
  io_rid: ResourceIdView,
)
  requires
    timer_heap_entries_valid(timers_view, l),
    reactor_safety_inv(l),
    io_currently_active(l, io_rid),
  ensures
    forall |d: InstantView, log_idx: int|
      #![auto] timers_view.contains((d, io_rid, log_idx)) ==> false,
{
  let io_reg = choose |reg_idx: int| 0 <= reg_idx < l.len() &&
    io_api_registered_at(l, reg_idx) &&
    get_io_api_register_rid(l[reg_idx]) == io_rid &&
    io_api_active_at(l, reg_idx, l.len() as int);
  assert forall |d: InstantView, log_idx: int|
    #![auto] timers_view.contains((d, io_rid, log_idx)) implies false
  by {
    assert(timer_awaiting_wake(l, log_idx));
    assert(is_succ_register_timer_at(l, log_idx));
    assert(get_register_timer_rid(l[log_idx]) == io_rid);
    assert(timer_active_at(l, log_idx, l.len() as int));
    if log_idx < io_reg {
      assert(no_timer_with_rid_before(l, io_rid, io_reg));
      let dd = choose |dd: int| log_idx < dd < io_reg && timer_retired_at(l, io_rid, dd);
      timer_retired_implies(l, io_rid, dd);
      assert(timer_awaiting_wake(l, log_idx));
      assert(log_idx < dd && dd < l.len());
      assert(false);
    } else {
      assert(no_io_api_with_rid_before(l, io_rid, log_idx));
      let dd = choose |dd: int| io_reg < dd < log_idx &&
        io_api_deregistered_at(l, dd) && get_io_api_deregister_rid(l[dd]) == io_rid;
      assert(io_api_active_at(l, io_reg, l.len() as int));
      assert(false);
    }
  }
}

pub proof fn timer_data_inv_preserved_by_io_wake(
  timers_view: Set<(InstantView, ResourceIdView, int)>,
  by_rid_view: Map<ResourceIdView, (InstantView, ResourceIdView, int)>,
  timer_wakers_view: Map<ResourceIdView, WakerView>,
  l: Log,
  e: ReactorEvent,
  io_rid: ResourceIdView,
)
  requires
    timer_heap_entries_valid(timers_view, l),
    active_timers_in_heap(by_rid_view, l),
    timer_wakers_match(timer_wakers_view, by_rid_view, l),
    timer_heap_has_wakers(timer_wakers_view, by_rid_view),
    reactor_safety_inv(l),
    is_wake_task_at(l.push(e), l.len() as int),
    get_wake_task_source_rid(e) == io_rid,
    !is_deregister_timer_at(l.push(e), l.len() as int),
    io_currently_active(l, io_rid),
  ensures
    timer_heap_entries_valid(timers_view, l.push(e)),
    active_timers_in_heap(by_rid_view, l.push(e)),
    timer_wakers_match(timer_wakers_view, by_rid_view, l.push(e)),
    timer_heap_has_wakers(timer_wakers_view, by_rid_view),
{
  timer_heap_no_io_rid(timers_view, l, io_rid);
  timer_heap_entries_valid_preserved_by_wake_of_nonmember_rid(timers_view, l, e, io_rid);
  timer_wakers_match_preserved_by_append(timer_wakers_view, by_rid_view, l, e);
  let l2 = l.push(e);
  assert forall |log_idx: int| #![auto] timer_awaiting_wake(l2, log_idx) implies {
    let rid = get_register_timer_rid(l2[log_idx]);
    by_rid_view.contains_key(rid) &&
    by_rid_view[rid].2 == log_idx
  } by {
    let rid = get_register_timer_rid(l2[log_idx]);
    assert(log_idx < l.len() as int);
    assert(l2[log_idx] == l[log_idx]);
    assert(is_succ_register_timer_at(l, log_idx));
    assert(rid != io_rid) by {
      if rid == io_rid {
        assert(timers_view.contains((get_register_timer_deadline(l[log_idx]), io_rid, log_idx)) ==> false);
      }
    }
    let ew = l2[l.len() as int];
    timer_awaiting_wake_shrink_past_non_matching_wake(l, ew, log_idx);
    assert(timer_awaiting_wake(l, log_idx));
  }
}

pub proof fn data_inv_preserved_by_harmless_event(
  timers_view: Set<(InstantView, ResourceIdView, int)>,
  by_rid_view: Map<ResourceIdView, (InstantView, ResourceIdView, int)>,
  timer_wakers_view: Map<ResourceIdView, WakerView>,
  read_wakers_view: Map<ResourceIdView, WakerView>,
  write_wakers_view: Map<ResourceIdView, WakerView>,
  l: Log,
  e: ReactorEvent,
)
  requires
    data_inv(timers_view, by_rid_view, timer_wakers_view, read_wakers_view, write_wakers_view, l),
    !is_succ_register_timer_at(l.push(e), l.len() as int),
    !is_deregister_timer_at(l.push(e), l.len() as int),
    !is_wake_task_at(l.push(e), l.len() as int),
    !is_succ_set_waker_at(l.push(e), l.len() as int),
    !io_api_registered_at(l.push(e), l.len() as int),
    !io_api_deregistered_at(l.push(e), l.len() as int),
  ensures
    data_inv(timers_view, by_rid_view, timer_wakers_view, read_wakers_view, write_wakers_view, l.push(e)),
{
  timer_heap_entries_valid_preserved_by_non_timer_event(timers_view, l, e);
  active_timers_in_heap_preserved_by_non_timer_event(by_rid_view, l, e);
  timer_wakers_match_preserved_by_append(timer_wakers_view, by_rid_view, l, e);
  read_wakers_valid_preserved_by_non_set_waker(read_wakers_view, l, e);
  write_wakers_valid_preserved_by_non_set_waker(write_wakers_view, l, e);
  read_wakers_complete_preserved_by_non_trigger(read_wakers_view, l, e);
  write_wakers_complete_preserved_by_non_trigger(write_wakers_view, l, e);
}

#[verifier::rlimit(30)]
pub proof fn read_wakers_complete_preserved_by_fresh_register_io(
  read_wakers_view: Map<ResourceIdView, WakerView>,
  l: Log,
  e: ReactorEvent,
  new_rid: ResourceIdView,
)
  requires
    read_wakers_complete(read_wakers_view, l),
    io_api_registered_at(l.push(e), l.len() as int),
    get_io_api_register_rid(l.push(e)[l.len() as int]) == new_rid,
    no_prior_io_api_registration(l, new_rid, l.len() as int),
  ensures
    read_wakers_complete(read_wakers_view, l.push(e)),
{
  let l2 = l.push(e);
  let n = l.len() as int;
  assert forall |rid: ResourceIdView| #![auto]
    has_active_readable_set_waker(l2, rid) implies
    read_wakers_view.contains_key(rid)
  by {
    let reg_idx = choose |reg_idx: int| 0 <= reg_idx < l2.len() &&
      io_api_registered_at(l2, reg_idx) &&
      get_io_api_register_rid(l2[reg_idx]) == rid &&
      io_api_active_at(l2, reg_idx, l2.len() as int) &&
      exists |sw_idx: int| reg_idx < sw_idx < l2.len() &&
        is_succ_set_waker_at(l2, sw_idx) &&
        get_set_waker_rid(l2[sw_idx]) == rid &&
        get_set_waker_interest(l2[sw_idx]).0;
    let sw_idx = choose |sw_idx: int| reg_idx < sw_idx < l2.len() &&
      is_succ_set_waker_at(l2, sw_idx) &&
      get_set_waker_rid(l2[sw_idx]) == rid &&
      get_set_waker_interest(l2[sw_idx]).0;
    if rid == new_rid {
      if reg_idx == n {
        assert(reg_idx < sw_idx);
        assert(sw_idx < l2.len());
        assert(sw_idx < n + 1);
        assert(false);
      } else {
        assert(reg_idx < n);
        assert(l2[reg_idx] == l[reg_idx]);
        assert(io_api_registered_at(l, reg_idx));
        assert(get_io_api_register_rid(l[reg_idx]) == new_rid);
        assert(no_prior_io_api_registration(l, new_rid, n));
        let d = choose |d: int| reg_idx < d < n &&
          io_api_deregistered_at(l, d) && get_io_api_deregister_rid(l[d]) == new_rid;
        assert(l2[d] == l[d]);
        assert(io_api_deregistered_at(l2, d));
        assert(get_io_api_deregister_rid(l2[d]) == get_io_api_register_rid(l2[reg_idx]));
        assert(false);
      }
    }
    assert(rid != new_rid);
    assert(reg_idx < n);
    assert(sw_idx < n);
    assert(l2[reg_idx] == l[reg_idx]);
    assert(l2[sw_idx] == l[sw_idx]);
    assert forall |k: int| reg_idx < k < n implies !(
      io_api_deregistered_at(l, k) &&
      get_io_api_deregister_rid(l[k]) == get_io_api_register_rid(l[reg_idx])
    ) by {
      assert(l2[k] == l[k]);
      assert(!(io_api_deregistered_at(l2, k) &&
        get_io_api_deregister_rid(l2[k]) == get_io_api_register_rid(l2[reg_idx])));
    }
    assert(io_api_active_at(l, reg_idx, n));
    assert(is_succ_set_waker_at(l, sw_idx));
    assert(get_set_waker_rid(l[sw_idx]) == rid);
    assert(get_set_waker_interest(l[sw_idx]).0);
    assert(reg_idx < sw_idx && sw_idx < l.len());
    assert(has_active_readable_set_waker(l, rid));
  }
}

#[verifier::rlimit(30)]
pub proof fn write_wakers_complete_preserved_by_fresh_register_io(
  write_wakers_view: Map<ResourceIdView, WakerView>,
  l: Log,
  e: ReactorEvent,
  new_rid: ResourceIdView,
)
  requires
    write_wakers_complete(write_wakers_view, l),
    io_api_registered_at(l.push(e), l.len() as int),
    get_io_api_register_rid(l.push(e)[l.len() as int]) == new_rid,
    no_prior_io_api_registration(l, new_rid, l.len() as int),
  ensures
    write_wakers_complete(write_wakers_view, l.push(e)),
{
  let l2 = l.push(e);
  let n = l.len() as int;
  assert forall |rid: ResourceIdView| #![auto]
    has_active_writable_set_waker(l2, rid) implies
    write_wakers_view.contains_key(rid)
  by {
    let reg_idx = choose |reg_idx: int| 0 <= reg_idx < l2.len() &&
      io_api_registered_at(l2, reg_idx) &&
      get_io_api_register_rid(l2[reg_idx]) == rid &&
      io_api_active_at(l2, reg_idx, l2.len() as int) &&
      exists |sw_idx: int| reg_idx < sw_idx < l2.len() &&
        is_succ_set_waker_at(l2, sw_idx) &&
        get_set_waker_rid(l2[sw_idx]) == rid &&
        get_set_waker_interest(l2[sw_idx]).1;
    let sw_idx = choose |sw_idx: int| reg_idx < sw_idx < l2.len() &&
      is_succ_set_waker_at(l2, sw_idx) &&
      get_set_waker_rid(l2[sw_idx]) == rid &&
      get_set_waker_interest(l2[sw_idx]).1;
    if rid == new_rid {
      if reg_idx == n {
        assert(reg_idx < sw_idx);
        assert(sw_idx < l2.len());
        assert(sw_idx < n + 1);
        assert(false);
      } else {
        assert(reg_idx < n);
        assert(l2[reg_idx] == l[reg_idx]);
        assert(io_api_registered_at(l, reg_idx));
        assert(get_io_api_register_rid(l[reg_idx]) == new_rid);
        assert(no_prior_io_api_registration(l, new_rid, n));
        let d = choose |d: int| reg_idx < d < n &&
          io_api_deregistered_at(l, d) && get_io_api_deregister_rid(l[d]) == new_rid;
        assert(l2[d] == l[d]);
        assert(io_api_deregistered_at(l2, d));
        assert(get_io_api_deregister_rid(l2[d]) == get_io_api_register_rid(l2[reg_idx]));
        assert(false);
      }
    }
    assert(rid != new_rid);
    assert(reg_idx < n);
    assert(sw_idx < n);
    assert(l2[reg_idx] == l[reg_idx]);
    assert(l2[sw_idx] == l[sw_idx]);
    assert forall |k: int| reg_idx < k < n implies !(
      io_api_deregistered_at(l, k) &&
      get_io_api_deregister_rid(l[k]) == get_io_api_register_rid(l[reg_idx])
    ) by {
      assert(l2[k] == l[k]);
      assert(!(io_api_deregistered_at(l2, k) &&
        get_io_api_deregister_rid(l2[k]) == get_io_api_register_rid(l2[reg_idx])));
    }
    assert(io_api_active_at(l, reg_idx, n));
    assert(is_succ_set_waker_at(l, sw_idx));
    assert(get_set_waker_rid(l[sw_idx]) == rid);
    assert(get_set_waker_interest(l[sw_idx]).1);
    assert(reg_idx < sw_idx && sw_idx < l.len());
    assert(has_active_writable_set_waker(l, rid));
  }
}

pub proof fn data_inv_preserved_by_fresh_register_io(
  timers_view: Set<(InstantView, ResourceIdView, int)>,
  by_rid_view: Map<ResourceIdView, (InstantView, ResourceIdView, int)>,
  timer_wakers_view: Map<ResourceIdView, WakerView>,
  read_wakers_view: Map<ResourceIdView, WakerView>,
  write_wakers_view: Map<ResourceIdView, WakerView>,
  l: Log,
  e: ReactorEvent,
  new_rid: ResourceIdView,
)
  requires
    data_inv(timers_view, by_rid_view, timer_wakers_view, read_wakers_view, write_wakers_view, l),
    io_api_registered_at(l.push(e), l.len() as int),
    get_io_api_register_rid(l.push(e)[l.len() as int]) == new_rid,
    no_prior_io_api_registration(l, new_rid, l.len() as int),
    !is_succ_register_timer_at(l.push(e), l.len() as int),
    !is_deregister_timer_at(l.push(e), l.len() as int),
    !is_wake_task_at(l.push(e), l.len() as int),
    !is_succ_set_waker_at(l.push(e), l.len() as int),
    !io_api_deregistered_at(l.push(e), l.len() as int),
  ensures
    data_inv(timers_view, by_rid_view, timer_wakers_view, read_wakers_view, write_wakers_view, l.push(e)),
{
  timer_heap_entries_valid_preserved_by_non_timer_event(timers_view, l, e);
  active_timers_in_heap_preserved_by_non_timer_event(by_rid_view, l, e);
  timer_wakers_match_preserved_by_append(timer_wakers_view, by_rid_view, l, e);
  read_wakers_valid_preserved_by_non_set_waker(read_wakers_view, l, e);
  write_wakers_valid_preserved_by_non_set_waker(write_wakers_view, l, e);
  read_wakers_complete_preserved_by_fresh_register_io(read_wakers_view, l, e, new_rid);
  write_wakers_complete_preserved_by_fresh_register_io(write_wakers_view, l, e, new_rid);
}

pub proof fn read_wakers_valid_remove_key(
  read_wakers_view: Map<ResourceIdView, WakerView>,
  log: Log,
  rid: ResourceIdView,
)
  requires
    read_wakers_valid(read_wakers_view, log),
  ensures
    read_wakers_valid(read_wakers_view.remove(rid), log),
{
  let new_rw = read_wakers_view.remove(rid);
  assert forall |r: ResourceIdView| #![auto]
    new_rw.contains_key(r) implies
    io_currently_active(log, r) &&
    exists |sw_idx: int| 0 <= sw_idx < log.len() &&
      is_succ_set_waker_at(log, sw_idx) &&
      get_set_waker_rid(log[sw_idx]) == r &&
      get_set_waker_interest(log[sw_idx]).0 &&
      get_set_waker_waker(log[sw_idx]) == new_rw[r] &&
      io_api_active_at_set_waker(log, r, sw_idx) &&
      forall |k: int| sw_idx < k < log.len() ==> !(
        is_succ_set_waker_at(log, k) &&
        get_set_waker_rid(log[k]) == r &&
        get_set_waker_interest(log[k]).0
      )
  by {
    assert(read_wakers_view.contains_key(r));
    assert(new_rw[r] == read_wakers_view[r]);
  }
}

pub proof fn write_wakers_valid_remove_key(
  write_wakers_view: Map<ResourceIdView, WakerView>,
  log: Log,
  rid: ResourceIdView,
)
  requires
    write_wakers_valid(write_wakers_view, log),
  ensures
    write_wakers_valid(write_wakers_view.remove(rid), log),
{
  let new_ww = write_wakers_view.remove(rid);
  assert forall |r: ResourceIdView| #![auto]
    new_ww.contains_key(r) implies
    io_currently_active(log, r) &&
    exists |sw_idx: int| 0 <= sw_idx < log.len() &&
      is_succ_set_waker_at(log, sw_idx) &&
      get_set_waker_rid(log[sw_idx]) == r &&
      get_set_waker_interest(log[sw_idx]).1 &&
      get_set_waker_waker(log[sw_idx]) == new_ww[r] &&
      io_api_active_at_set_waker(log, r, sw_idx) &&
      forall |k: int| sw_idx < k < log.len() ==> !(
        is_succ_set_waker_at(log, k) &&
        get_set_waker_rid(log[k]) == r &&
        get_set_waker_interest(log[k]).1
      )
  by {
    assert(write_wakers_view.contains_key(r));
    assert(new_ww[r] == write_wakers_view[r]);
  }
}

#[verifier::rlimit(30)]
pub proof fn read_wakers_complete_remove_deregistered(
  read_wakers_view: Map<ResourceIdView, WakerView>,
  log: Log,
  deregistered_rid: ResourceIdView,
)
  requires
    read_wakers_complete(read_wakers_view, log),
    !io_currently_active(log, deregistered_rid),
  ensures
    read_wakers_complete(read_wakers_view.remove(deregistered_rid), log),
{
  let new_rw = read_wakers_view.remove(deregistered_rid);
  assert forall |rid: ResourceIdView| #![auto]
    has_active_readable_set_waker(log, rid) implies
    new_rw.contains_key(rid)
  by {
    if rid == deregistered_rid {
      assert(!io_currently_active(log, deregistered_rid));
      assert(!has_active_readable_set_waker(log, rid));
    } else {
      assert(read_wakers_view.contains_key(rid));
    }
  }
}

#[verifier::rlimit(30)]
pub proof fn write_wakers_complete_remove_deregistered(
  write_wakers_view: Map<ResourceIdView, WakerView>,
  log: Log,
  deregistered_rid: ResourceIdView,
)
  requires
    write_wakers_complete(write_wakers_view, log),
    !io_currently_active(log, deregistered_rid),
  ensures
    write_wakers_complete(write_wakers_view.remove(deregistered_rid), log),
{
  let new_ww = write_wakers_view.remove(deregistered_rid);
  assert forall |rid: ResourceIdView| #![auto]
    has_active_writable_set_waker(log, rid) implies
    new_ww.contains_key(rid)
  by {
    if rid == deregistered_rid {
      assert(!io_currently_active(log, deregistered_rid));
      assert(!has_active_writable_set_waker(log, rid));
    } else {
      assert(write_wakers_view.contains_key(rid));
    }
  }
}

#[verifier::rlimit(80)]
pub proof fn io_currently_active_preserved_by_deregister_other(
  l: Log,
  e: ReactorEvent,
  rid: ResourceIdView,
  deregistered_rid: ResourceIdView,
)
  requires
    io_currently_active(l, rid),
    rid != deregistered_rid,
    io_api_deregistered_at(l.push(e), l.len() as int),
    get_io_api_deregister_rid(l.push(e)[l.len() as int]) == deregistered_rid,
  ensures
    io_currently_active(l.push(e), rid),
{
  let l2 = l.push(e);
  let n = l.len() as int;
  let reg_idx = choose |reg_idx: int| 0 <= reg_idx < l.len() &&
    io_api_registered_at(l, reg_idx) &&
    get_io_api_register_rid(l[reg_idx]) == rid &&
    io_api_active_at(l, reg_idx, l.len() as int);
  assert(l2[reg_idx] == l[reg_idx]);
  assert(io_api_registered_at(l2, reg_idx));
  assert(get_io_api_register_rid(l2[reg_idx]) == rid);
  assert forall |k: int| reg_idx < k < l2.len() as int implies !(
    io_api_deregistered_at(l2, k) &&
    get_io_api_deregister_rid(l2[k]) == get_io_api_register_rid(l2[reg_idx])
  ) by {
    if k < n {
      assert(l2[k] == l[k]);
      assert(!(io_api_deregistered_at(l, k) &&
        get_io_api_deregister_rid(l[k]) == get_io_api_register_rid(l[reg_idx])));
    } else {
      assert(get_io_api_deregister_rid(l2[k]) == deregistered_rid);
      assert(deregistered_rid != rid);
    }
  }
  assert(io_api_active_at(l2, reg_idx, l2.len() as int));
}

#[verifier::rlimit(200)]
pub proof fn read_wakers_valid_after_deregister_io_and_remove(
  read_wakers_view: Map<ResourceIdView, WakerView>,
  l: Log,
  e: ReactorEvent,
  deregistered_rid: ResourceIdView,
)
  requires
    read_wakers_valid(read_wakers_view, l),
    io_api_deregistered_at(l.push(e), l.len() as int),
    get_io_api_deregister_rid(l.push(e)[l.len() as int]) == deregistered_rid,
  ensures
    read_wakers_valid(read_wakers_view.remove(deregistered_rid), l.push(e)),
{
  let l2 = l.push(e);
  let n = l.len() as int;
  let new_rw = read_wakers_view.remove(deregistered_rid);
  assert forall |rid: ResourceIdView| #![auto]
    new_rw.contains_key(rid) implies
    io_currently_active(l2, rid) &&
    exists |sw_idx: int| 0 <= sw_idx < l2.len() &&
      is_succ_set_waker_at(l2, sw_idx) &&
      get_set_waker_rid(l2[sw_idx]) == rid &&
      get_set_waker_interest(l2[sw_idx]).0 &&
      get_set_waker_waker(l2[sw_idx]) == new_rw[rid] &&
      io_api_active_at_set_waker(l2, rid, sw_idx) &&
      forall |k: int| sw_idx < k < l2.len() ==> !(
        is_succ_set_waker_at(l2, k) &&
        get_set_waker_rid(l2[k]) == rid &&
        get_set_waker_interest(l2[k]).0
      )
  by {
    assert(read_wakers_view.contains_key(rid));
    assert(rid != deregistered_rid);
    assert(new_rw[rid] == read_wakers_view[rid]);
    assert(io_currently_active(l, rid));

    io_currently_active_preserved_by_deregister_other(l, e, rid, deregistered_rid);
    assert(io_currently_active(l2, rid));

    let sw_idx = choose |sw_idx: int| 0 <= sw_idx < l.len() &&
      is_succ_set_waker_at(l, sw_idx) &&
      get_set_waker_rid(l[sw_idx]) == rid &&
      get_set_waker_interest(l[sw_idx]).0 &&
      get_set_waker_waker(l[sw_idx]) == read_wakers_view[rid] &&
      io_api_active_at_set_waker(l, rid, sw_idx) &&
      forall |k: int| sw_idx < k < l.len() ==> !(
        is_succ_set_waker_at(l, k) &&
        get_set_waker_rid(l[k]) == rid &&
        get_set_waker_interest(l[k]).0
      );
    assert(0 <= sw_idx < l.len() as int);
    assert(l2[sw_idx] == l[sw_idx]);
    let reg_idx = choose |reg_idx: int| 0 <= reg_idx < sw_idx &&
      io_api_registered_at(l, reg_idx) &&
      get_io_api_register_rid(l[reg_idx]) == rid &&
      io_api_active_at(l, reg_idx, sw_idx);
    assert(l2[reg_idx] == l[reg_idx]);
    assert forall |k: int| reg_idx < k < sw_idx implies !(
      io_api_deregistered_at(l2, k) &&
      get_io_api_deregister_rid(l2[k]) == get_io_api_register_rid(l2[reg_idx])
    ) by {
      assert(l2[k] == l[k]);
      assert(!(io_api_deregistered_at(l, k) &&
        get_io_api_deregister_rid(l[k]) == get_io_api_register_rid(l[reg_idx])));
    }
    assert(io_api_active_at(l2, reg_idx, sw_idx));
    assert(io_api_active_at_set_waker(l2, rid, sw_idx));
    assert(is_succ_set_waker_at(l2, sw_idx));
    assert(get_set_waker_rid(l2[sw_idx]) == rid);
    assert(get_set_waker_interest(l2[sw_idx]).0);
    assert(get_set_waker_waker(l2[sw_idx]) == new_rw[rid]);
    assert forall |k: int| sw_idx < k < l2.len() implies !(
      is_succ_set_waker_at(l2, k) &&
      get_set_waker_rid(l2[k]) == rid &&
      get_set_waker_interest(l2[k]).0
    ) by {
      if k < n {
        assert(l2[k] == l[k]);
        assert(!(is_succ_set_waker_at(l, k) &&
          get_set_waker_rid(l[k]) == rid &&
          get_set_waker_interest(l[k]).0));
      }
    }
  }
}

#[verifier::rlimit(200)]
pub proof fn write_wakers_valid_after_deregister_io_and_remove(
  write_wakers_view: Map<ResourceIdView, WakerView>,
  l: Log,
  e: ReactorEvent,
  deregistered_rid: ResourceIdView,
)
  requires
    write_wakers_valid(write_wakers_view, l),
    io_api_deregistered_at(l.push(e), l.len() as int),
    get_io_api_deregister_rid(l.push(e)[l.len() as int]) == deregistered_rid,
  ensures
    write_wakers_valid(write_wakers_view.remove(deregistered_rid), l.push(e)),
{
  let l2 = l.push(e);
  let n = l.len() as int;
  let new_ww = write_wakers_view.remove(deregistered_rid);
  assert forall |rid: ResourceIdView| #![auto]
    new_ww.contains_key(rid) implies
    io_currently_active(l2, rid) &&
    exists |sw_idx: int| 0 <= sw_idx < l2.len() &&
      is_succ_set_waker_at(l2, sw_idx) &&
      get_set_waker_rid(l2[sw_idx]) == rid &&
      get_set_waker_interest(l2[sw_idx]).1 &&
      get_set_waker_waker(l2[sw_idx]) == new_ww[rid] &&
      io_api_active_at_set_waker(l2, rid, sw_idx) &&
      forall |k: int| sw_idx < k < l2.len() ==> !(
        is_succ_set_waker_at(l2, k) &&
        get_set_waker_rid(l2[k]) == rid &&
        get_set_waker_interest(l2[k]).1
      )
  by {
    assert(write_wakers_view.contains_key(rid));
    assert(rid != deregistered_rid);
    assert(new_ww[rid] == write_wakers_view[rid]);
    assert(io_currently_active(l, rid));

    io_currently_active_preserved_by_deregister_other(l, e, rid, deregistered_rid);
    assert(io_currently_active(l2, rid));

    let sw_idx = choose |sw_idx: int| 0 <= sw_idx < l.len() &&
      is_succ_set_waker_at(l, sw_idx) &&
      get_set_waker_rid(l[sw_idx]) == rid &&
      get_set_waker_interest(l[sw_idx]).1 &&
      get_set_waker_waker(l[sw_idx]) == write_wakers_view[rid] &&
      io_api_active_at_set_waker(l, rid, sw_idx) &&
      forall |k: int| sw_idx < k < l.len() ==> !(
        is_succ_set_waker_at(l, k) &&
        get_set_waker_rid(l[k]) == rid &&
        get_set_waker_interest(l[k]).1
      );
    assert(0 <= sw_idx < l.len() as int);
    assert(l2[sw_idx] == l[sw_idx]);
    let reg_idx = choose |reg_idx: int| 0 <= reg_idx < sw_idx &&
      io_api_registered_at(l, reg_idx) &&
      get_io_api_register_rid(l[reg_idx]) == rid &&
      io_api_active_at(l, reg_idx, sw_idx);
    assert(l2[reg_idx] == l[reg_idx]);
    assert forall |k: int| reg_idx < k < sw_idx implies !(
      io_api_deregistered_at(l2, k) &&
      get_io_api_deregister_rid(l2[k]) == get_io_api_register_rid(l2[reg_idx])
    ) by {
      assert(l2[k] == l[k]);
      assert(!(io_api_deregistered_at(l, k) &&
        get_io_api_deregister_rid(l[k]) == get_io_api_register_rid(l[reg_idx])));
    }
    assert(io_api_active_at(l2, reg_idx, sw_idx));
    assert(io_api_active_at_set_waker(l2, rid, sw_idx));
    assert(is_succ_set_waker_at(l2, sw_idx));
    assert(get_set_waker_rid(l2[sw_idx]) == rid);
    assert(get_set_waker_interest(l2[sw_idx]).1);
    assert(get_set_waker_waker(l2[sw_idx]) == new_ww[rid]);
    assert forall |k: int| sw_idx < k < l2.len() implies !(
      is_succ_set_waker_at(l2, k) &&
      get_set_waker_rid(l2[k]) == rid &&
      get_set_waker_interest(l2[k]).1
    ) by {
      if k < n {
        assert(l2[k] == l[k]);
        assert(!(is_succ_set_waker_at(l, k) &&
          get_set_waker_rid(l[k]) == rid &&
          get_set_waker_interest(l[k]).1));
      }
    }
  }
}

#[verifier::rlimit(30)]
pub proof fn io_not_active_after_deregister(
  log: Log,
  deregister_event: ReactorEvent,
  rid: ResourceIdView,
)
  requires
    io_api_deregistered_at(log.push(deregister_event), log.len() as int),
    get_io_api_deregister_rid(log.push(deregister_event)[log.len() as int]) == rid,
  ensures
    !io_currently_active(log.push(deregister_event), rid),
{
  let l2 = log.push(deregister_event);
  let n = log.len() as int;
  if io_currently_active(l2, rid) {
    let reg_idx = choose |reg_idx: int| 0 <= reg_idx < l2.len() &&
      io_api_registered_at(l2, reg_idx) &&
      get_io_api_register_rid(l2[reg_idx]) == rid &&
      io_api_active_at(l2, reg_idx, l2.len() as int);
    assert(reg_idx < n);
    assert(io_api_active_at(l2, reg_idx, l2.len() as int));
    assert(reg_idx < n);
    assert(!(io_api_deregistered_at(l2, n) &&
      get_io_api_deregister_rid(l2[n]) == get_io_api_register_rid(l2[reg_idx])));
    assert(l2[n] == deregister_event);
    assert(io_api_deregistered_at(l2, n));
    assert(get_io_api_deregister_rid(l2[n]) == rid);
    assert(get_io_api_register_rid(l2[reg_idx]) == rid);
    assert(false);
  }
}

#[verifier::rlimit(80)]
pub proof fn read_wakers_valid_after_set_waker_event(
  old_read_wakers: Map<ResourceIdView, WakerView>,
  l: Log,
  e: ReactorEvent,
  rid: ResourceIdView,
  waker_val: WakerView,
  readable: bool,
)
  requires
    read_wakers_valid(old_read_wakers, l),
    is_succ_set_waker_at(l.push(e), l.len() as int),
    get_set_waker_rid(l.push(e)[l.len() as int]) == rid,
    get_set_waker_waker(l.push(e)[l.len() as int]) == waker_val,
    get_set_waker_interest(l.push(e)[l.len() as int]).0 == readable,
    readable ==> io_api_active_at_set_waker(l.push(e), rid, l.len() as int),
    readable ==> io_currently_active(l.push(e), rid),
  ensures
    read_wakers_valid(
      if readable { old_read_wakers.insert(rid, waker_val) } else { old_read_wakers },
      l.push(e),
    ),
{
  let l2 = l.push(e);
  let n = l.len() as int;
  let new_rw = if readable { old_read_wakers.insert(rid, waker_val) } else { old_read_wakers };
  assert(!io_api_deregistered_at(l2, n));
  assert forall |r: ResourceIdView| #![auto]
    new_rw.contains_key(r) implies
    io_currently_active(l2, r) &&
    exists |sw_idx: int| 0 <= sw_idx < l2.len() &&
      is_succ_set_waker_at(l2, sw_idx) &&
      get_set_waker_rid(l2[sw_idx]) == r &&
      get_set_waker_interest(l2[sw_idx]).0 &&
      get_set_waker_waker(l2[sw_idx]) == new_rw[r] &&
      io_api_active_at_set_waker(l2, r, sw_idx) &&
      forall |k: int| sw_idx < k < l2.len() ==> !(
        is_succ_set_waker_at(l2, k) &&
        get_set_waker_rid(l2[k]) == r &&
        get_set_waker_interest(l2[k]).0
      )
  by {
    if readable && r == rid {
      assert(io_currently_active(l2, r));
      assert(new_rw[r] == waker_val);
      assert(is_succ_set_waker_at(l2, n));
      assert(get_set_waker_rid(l2[n]) == r);
      assert(get_set_waker_interest(l2[n]).0);
      assert(get_set_waker_waker(l2[n]) == waker_val);
      assert(io_api_active_at_set_waker(l2, r, n));
    } else {
      assert(old_read_wakers.contains_key(r));
      assert(new_rw[r] == old_read_wakers[r]);

      assert(io_currently_active(l, r));
      let reg_idx_outer = choose |reg_idx: int| 0 <= reg_idx < l.len() &&
        io_api_registered_at(l, reg_idx) &&
        get_io_api_register_rid(l[reg_idx]) == r &&
        io_api_active_at(l, reg_idx, l.len() as int);
      assert(l2[reg_idx_outer] == l[reg_idx_outer]);
      assert forall |k: int| reg_idx_outer < k < l2.len() as int implies !(
        io_api_deregistered_at(l2, k) &&
        get_io_api_deregister_rid(l2[k]) == get_io_api_register_rid(l2[reg_idx_outer])
      ) by {
        if k < n {
          assert(l2[k] == l[k]);
          assert(!(io_api_deregistered_at(l, k) &&
            get_io_api_deregister_rid(l[k]) == get_io_api_register_rid(l[reg_idx_outer])));
        } else {
          assert(!io_api_deregistered_at(l2, k));
        }
      }
      assert(io_api_active_at(l2, reg_idx_outer, l2.len() as int));
      assert(io_currently_active(l2, r));

      let sw_idx = choose |sw_idx: int| 0 <= sw_idx < l.len() &&
        is_succ_set_waker_at(l, sw_idx) &&
        get_set_waker_rid(l[sw_idx]) == r &&
        get_set_waker_interest(l[sw_idx]).0 &&
        get_set_waker_waker(l[sw_idx]) == old_read_wakers[r] &&
        io_api_active_at_set_waker(l, r, sw_idx) &&
        forall |k: int| sw_idx < k < l.len() ==> !(
          is_succ_set_waker_at(l, k) &&
          get_set_waker_rid(l[k]) == r &&
          get_set_waker_interest(l[k]).0
        );
      assert(l2[sw_idx] == l[sw_idx]);
      let reg_idx = choose |reg_idx: int| 0 <= reg_idx < sw_idx &&
        io_api_registered_at(l, reg_idx) &&
        get_io_api_register_rid(l[reg_idx]) == r &&
        io_api_active_at(l, reg_idx, sw_idx);
      assert(l2[reg_idx] == l[reg_idx]);
      assert forall |k: int| reg_idx < k < sw_idx implies !(
        io_api_deregistered_at(l2, k) &&
        get_io_api_deregister_rid(l2[k]) == get_io_api_register_rid(l2[reg_idx])
      ) by {
        assert(l2[k] == l[k]);
        assert(!(io_api_deregistered_at(l, k) &&
          get_io_api_deregister_rid(l[k]) == get_io_api_register_rid(l[reg_idx])));
      }
      assert(io_api_active_at(l2, reg_idx, sw_idx));
      assert(io_api_active_at_set_waker(l2, r, sw_idx));
      assert(is_succ_set_waker_at(l2, sw_idx));
      assert(get_set_waker_rid(l2[sw_idx]) == r);
      assert(get_set_waker_interest(l2[sw_idx]).0);
      assert(get_set_waker_waker(l2[sw_idx]) == new_rw[r]);
      assert forall |k: int| sw_idx < k < l2.len() implies !(
        is_succ_set_waker_at(l2, k) &&
        get_set_waker_rid(l2[k]) == r &&
        get_set_waker_interest(l2[k]).0
      ) by {
        if k < n {
          assert(l2[k] == l[k]);
          assert(!(is_succ_set_waker_at(l, k) &&
            get_set_waker_rid(l[k]) == r &&
            get_set_waker_interest(l[k]).0));
        } else {
          assert(k == n);
          if readable {
            assert(get_set_waker_rid(l2[n]) == rid);
            assert(r != rid);
          } else {
            assert(!get_set_waker_interest(l2[n]).0);
          }
        }
      }
    }
  }
}

#[verifier::rlimit(80)]
pub proof fn write_wakers_valid_after_set_waker_event(
  old_write_wakers: Map<ResourceIdView, WakerView>,
  l: Log,
  e: ReactorEvent,
  rid: ResourceIdView,
  waker_val: WakerView,
  writable: bool,
)
  requires
    write_wakers_valid(old_write_wakers, l),
    is_succ_set_waker_at(l.push(e), l.len() as int),
    get_set_waker_rid(l.push(e)[l.len() as int]) == rid,
    get_set_waker_waker(l.push(e)[l.len() as int]) == waker_val,
    get_set_waker_interest(l.push(e)[l.len() as int]).1 == writable,
    writable ==> io_api_active_at_set_waker(l.push(e), rid, l.len() as int),
    writable ==> io_currently_active(l.push(e), rid),
  ensures
    write_wakers_valid(
      if writable { old_write_wakers.insert(rid, waker_val) } else { old_write_wakers },
      l.push(e),
    ),
{
  let l2 = l.push(e);
  let n = l.len() as int;
  let new_ww = if writable { old_write_wakers.insert(rid, waker_val) } else { old_write_wakers };
  assert(!io_api_deregistered_at(l2, n));
  assert forall |r: ResourceIdView| #![auto]
    new_ww.contains_key(r) implies
    io_currently_active(l2, r) &&
    exists |sw_idx: int| 0 <= sw_idx < l2.len() &&
      is_succ_set_waker_at(l2, sw_idx) &&
      get_set_waker_rid(l2[sw_idx]) == r &&
      get_set_waker_interest(l2[sw_idx]).1 &&
      get_set_waker_waker(l2[sw_idx]) == new_ww[r] &&
      io_api_active_at_set_waker(l2, r, sw_idx) &&
      forall |k: int| sw_idx < k < l2.len() ==> !(
        is_succ_set_waker_at(l2, k) &&
        get_set_waker_rid(l2[k]) == r &&
        get_set_waker_interest(l2[k]).1
      )
  by {
    if writable && r == rid {
      assert(io_currently_active(l2, r));
      assert(new_ww[r] == waker_val);
      assert(is_succ_set_waker_at(l2, n));
      assert(get_set_waker_rid(l2[n]) == r);
      assert(get_set_waker_interest(l2[n]).1);
      assert(get_set_waker_waker(l2[n]) == waker_val);
      assert(io_api_active_at_set_waker(l2, r, n));
    } else {
      assert(old_write_wakers.contains_key(r));
      assert(new_ww[r] == old_write_wakers[r]);

      assert(io_currently_active(l, r));
      let reg_idx_outer = choose |reg_idx: int| 0 <= reg_idx < l.len() &&
        io_api_registered_at(l, reg_idx) &&
        get_io_api_register_rid(l[reg_idx]) == r &&
        io_api_active_at(l, reg_idx, l.len() as int);
      assert(l2[reg_idx_outer] == l[reg_idx_outer]);
      assert forall |k: int| reg_idx_outer < k < l2.len() as int implies !(
        io_api_deregistered_at(l2, k) &&
        get_io_api_deregister_rid(l2[k]) == get_io_api_register_rid(l2[reg_idx_outer])
      ) by {
        if k < n {
          assert(l2[k] == l[k]);
          assert(!(io_api_deregistered_at(l, k) &&
            get_io_api_deregister_rid(l[k]) == get_io_api_register_rid(l[reg_idx_outer])));
        } else {
          assert(!io_api_deregistered_at(l2, k));
        }
      }
      assert(io_api_active_at(l2, reg_idx_outer, l2.len() as int));
      assert(io_currently_active(l2, r));

      let sw_idx = choose |sw_idx: int| 0 <= sw_idx < l.len() &&
        is_succ_set_waker_at(l, sw_idx) &&
        get_set_waker_rid(l[sw_idx]) == r &&
        get_set_waker_interest(l[sw_idx]).1 &&
        get_set_waker_waker(l[sw_idx]) == old_write_wakers[r] &&
        io_api_active_at_set_waker(l, r, sw_idx) &&
        forall |k: int| sw_idx < k < l.len() ==> !(
          is_succ_set_waker_at(l, k) &&
          get_set_waker_rid(l[k]) == r &&
          get_set_waker_interest(l[k]).1
        );
      assert(l2[sw_idx] == l[sw_idx]);
      let reg_idx = choose |reg_idx: int| 0 <= reg_idx < sw_idx &&
        io_api_registered_at(l, reg_idx) &&
        get_io_api_register_rid(l[reg_idx]) == r &&
        io_api_active_at(l, reg_idx, sw_idx);
      assert(l2[reg_idx] == l[reg_idx]);
      assert forall |k: int| reg_idx < k < sw_idx implies !(
        io_api_deregistered_at(l2, k) &&
        get_io_api_deregister_rid(l2[k]) == get_io_api_register_rid(l2[reg_idx])
      ) by {
        assert(l2[k] == l[k]);
        assert(!(io_api_deregistered_at(l, k) &&
          get_io_api_deregister_rid(l[k]) == get_io_api_register_rid(l[reg_idx])));
      }
      assert(io_api_active_at(l2, reg_idx, sw_idx));
      assert(io_api_active_at_set_waker(l2, r, sw_idx));
      assert(is_succ_set_waker_at(l2, sw_idx));
      assert(get_set_waker_rid(l2[sw_idx]) == r);
      assert(get_set_waker_interest(l2[sw_idx]).1);
      assert(get_set_waker_waker(l2[sw_idx]) == new_ww[r]);
      assert forall |k: int| sw_idx < k < l2.len() implies !(
        is_succ_set_waker_at(l2, k) &&
        get_set_waker_rid(l2[k]) == r &&
        get_set_waker_interest(l2[k]).1
      ) by {
        if k < n {
          assert(l2[k] == l[k]);
          assert(!(is_succ_set_waker_at(l, k) &&
            get_set_waker_rid(l[k]) == r &&
            get_set_waker_interest(l[k]).1));
        } else {
          assert(k == n);
          if writable {
            assert(get_set_waker_rid(l2[n]) == rid);
            assert(r != rid);
          } else {
            assert(!get_set_waker_interest(l2[n]).1);
          }
        }
      }
    }
  }
}

#[verifier::rlimit(30)]
pub proof fn read_wakers_complete_after_set_waker_event(
  old_read_wakers: Map<ResourceIdView, WakerView>,
  l: Log,
  e: ReactorEvent,
  rid: ResourceIdView,
  readable: bool,
)
  requires
    read_wakers_complete(old_read_wakers, l),
    is_succ_set_waker_at(l.push(e), l.len() as int),
    get_set_waker_rid(l.push(e)[l.len() as int]) == rid,
    get_set_waker_interest(l.push(e)[l.len() as int]).0 == readable,
  ensures
    read_wakers_complete(
      if readable { old_read_wakers.insert(rid, get_set_waker_waker(l.push(e)[l.len() as int])) } else { old_read_wakers },
      l.push(e),
    ),
{
  let l2 = l.push(e);
  let n = l.len() as int;
  let waker_val = get_set_waker_waker(l2[n]);
  let new_rw = if readable { old_read_wakers.insert(rid, waker_val) } else { old_read_wakers };
  assert forall |r: ResourceIdView| #![auto]
    has_active_readable_set_waker(l2, r) implies
    new_rw.contains_key(r)
  by {
    let reg_idx = choose |reg_idx: int| 0 <= reg_idx < l2.len() &&
      io_api_registered_at(l2, reg_idx) &&
      get_io_api_register_rid(l2[reg_idx]) == r &&
      io_api_active_at(l2, reg_idx, l2.len() as int) &&
      exists |sw_idx: int| reg_idx < sw_idx < l2.len() &&
        is_succ_set_waker_at(l2, sw_idx) &&
        get_set_waker_rid(l2[sw_idx]) == r &&
        get_set_waker_interest(l2[sw_idx]).0;
    let sw_idx = choose |sw_idx: int| reg_idx < sw_idx < l2.len() &&
      is_succ_set_waker_at(l2, sw_idx) &&
      get_set_waker_rid(l2[sw_idx]) == r &&
      get_set_waker_interest(l2[sw_idx]).0;
    assert(reg_idx < n);
    if sw_idx == n {
      assert(get_set_waker_rid(l2[n]) == rid);
      assert(r == rid);
      assert(get_set_waker_interest(l2[n]).0 == readable);
      assert(readable);
      assert(new_rw.contains_key(r));
    } else {
      assert(sw_idx < n);
      assert(l2[reg_idx] == l[reg_idx]);
      assert(l2[sw_idx] == l[sw_idx]);
      assert forall |k: int| reg_idx < k < n implies !(
        io_api_deregistered_at(l, k) &&
        get_io_api_deregister_rid(l[k]) == get_io_api_register_rid(l[reg_idx])
      ) by {
        assert(l2[k] == l[k]);
        assert(!(io_api_deregistered_at(l2, k) &&
          get_io_api_deregister_rid(l2[k]) == get_io_api_register_rid(l2[reg_idx])));
      }
      assert(io_api_active_at(l, reg_idx, n));
      assert(is_succ_set_waker_at(l, sw_idx));
      assert(get_set_waker_rid(l[sw_idx]) == r);
      assert(get_set_waker_interest(l[sw_idx]).0);
      assert(reg_idx < sw_idx && sw_idx < l.len());
      assert(has_active_readable_set_waker(l, r));
      assert(old_read_wakers.contains_key(r));
    }
  }
}

#[verifier::rlimit(30)]
pub proof fn write_wakers_complete_after_set_waker_event(
  old_write_wakers: Map<ResourceIdView, WakerView>,
  l: Log,
  e: ReactorEvent,
  rid: ResourceIdView,
  writable: bool,
)
  requires
    write_wakers_complete(old_write_wakers, l),
    is_succ_set_waker_at(l.push(e), l.len() as int),
    get_set_waker_rid(l.push(e)[l.len() as int]) == rid,
    get_set_waker_interest(l.push(e)[l.len() as int]).1 == writable,
  ensures
    write_wakers_complete(
      if writable { old_write_wakers.insert(rid, get_set_waker_waker(l.push(e)[l.len() as int])) } else { old_write_wakers },
      l.push(e),
    ),
{
  let l2 = l.push(e);
  let n = l.len() as int;
  let waker_val = get_set_waker_waker(l2[n]);
  let new_ww = if writable { old_write_wakers.insert(rid, waker_val) } else { old_write_wakers };
  assert forall |r: ResourceIdView| #![auto]
    has_active_writable_set_waker(l2, r) implies
    new_ww.contains_key(r)
  by {
    let reg_idx = choose |reg_idx: int| 0 <= reg_idx < l2.len() &&
      io_api_registered_at(l2, reg_idx) &&
      get_io_api_register_rid(l2[reg_idx]) == r &&
      io_api_active_at(l2, reg_idx, l2.len() as int) &&
      exists |sw_idx: int| reg_idx < sw_idx < l2.len() &&
        is_succ_set_waker_at(l2, sw_idx) &&
        get_set_waker_rid(l2[sw_idx]) == r &&
        get_set_waker_interest(l2[sw_idx]).1;
    let sw_idx = choose |sw_idx: int| reg_idx < sw_idx < l2.len() &&
      is_succ_set_waker_at(l2, sw_idx) &&
      get_set_waker_rid(l2[sw_idx]) == r &&
      get_set_waker_interest(l2[sw_idx]).1;
    assert(reg_idx < n);
    if sw_idx == n {
      assert(get_set_waker_rid(l2[n]) == rid);
      assert(r == rid);
      assert(get_set_waker_interest(l2[n]).1 == writable);
      assert(writable);
      assert(new_ww.contains_key(r));
    } else {
      assert(sw_idx < n);
      assert(l2[reg_idx] == l[reg_idx]);
      assert(l2[sw_idx] == l[sw_idx]);
      assert forall |k: int| reg_idx < k < n implies !(
        io_api_deregistered_at(l, k) &&
        get_io_api_deregister_rid(l[k]) == get_io_api_register_rid(l[reg_idx])
      ) by {
        assert(l2[k] == l[k]);
        assert(!(io_api_deregistered_at(l2, k) &&
          get_io_api_deregister_rid(l2[k]) == get_io_api_register_rid(l2[reg_idx])));
      }
      assert(io_api_active_at(l, reg_idx, n));
      assert(is_succ_set_waker_at(l, sw_idx));
      assert(get_set_waker_rid(l[sw_idx]) == r);
      assert(get_set_waker_interest(l[sw_idx]).1);
      assert(reg_idx < sw_idx && sw_idx < l.len());
      assert(has_active_writable_set_waker(l, r));
      assert(old_write_wakers.contains_key(r));
    }
  }
}

#[verifier::rlimit(50)]
pub proof fn timer_heap_entries_valid_after_fresh_register_timer(
  old_timers: Set<(InstantView, ResourceIdView, int)>,
  l: Log,
  e: ReactorEvent,
  new_rid: ResourceIdView,
  deadline_val: InstantView,
)
  requires
    timer_heap_entries_valid(old_timers, l),
    is_succ_register_timer_at(l.push(e), l.len() as int),
    get_register_timer_rid(l.push(e)[l.len() as int]) == new_rid,
    get_register_timer_deadline(l.push(e)[l.len() as int]) == deadline_val,
    !is_deregister_timer_at(l.push(e), l.len() as int),
    !is_wake_task_at(l.push(e), l.len() as int),
  ensures
    timer_heap_entries_valid(
      old_timers.insert((deadline_val, new_rid, l.len() as int)),
      l.push(e),
    ),
{
  let l2 = l.push(e);
  let n = l.len() as int;
  let new_timers = old_timers.insert((deadline_val, new_rid, n));

  timer_heap_entries_valid_preserved_by_non_timer_event(old_timers, l, e);

  assert forall |d: InstantView, rid: ResourceIdView, log_idx: int|
    #![auto] new_timers.contains((d, rid, log_idx)) implies {
      timer_awaiting_wake(l2, log_idx) &&
      get_register_timer_rid(l2[log_idx]) == rid &&
      get_register_timer_deadline(l2[log_idx]) == d
    }
  by {
    if d == deadline_val && rid == new_rid && log_idx == n {
      assert(l2[n] == e);
      assert(is_succ_register_timer_at(l2, n));
      assert(get_register_timer_rid(l2[n]) == new_rid);
      assert(get_register_timer_deadline(l2[n]) == deadline_val);
      assert(timer_active_at(l2, n, l2.len() as int));
    } else {
      assert(old_timers.contains((d, rid, log_idx)));
      assert(timer_heap_entries_valid(old_timers, l2));
    }
  }
}

#[verifier::rlimit(50)]
pub proof fn active_timers_in_heap_after_fresh_register_timer(
  old_by_rid: Map<ResourceIdView, (InstantView, ResourceIdView, int)>,
  l: Log,
  e: ReactorEvent,
  new_rid: ResourceIdView,
  deadline_val: InstantView,
)
  requires
    active_timers_in_heap(old_by_rid, l),
    is_succ_register_timer_at(l.push(e), l.len() as int),
    get_register_timer_rid(l.push(e)[l.len() as int]) == new_rid,
    get_register_timer_deadline(l.push(e)[l.len() as int]) == deadline_val,
    !is_deregister_timer_at(l.push(e), l.len() as int),
    !is_wake_task_at(l.push(e), l.len() as int),
    no_prior_timer_registration(l, new_rid, l.len() as int),
    !old_by_rid.contains_key(new_rid),
  ensures
    active_timers_in_heap(
      old_by_rid.insert(new_rid, (deadline_val, new_rid, l.len() as int)),
      l.push(e),
    ),
{
  let l2 = l.push(e);
  let n = l.len() as int;
  let new_by_rid = old_by_rid.insert(new_rid, (deadline_val, new_rid, n));

  assert forall |log_idx: int| #![auto] timer_awaiting_wake(l2, log_idx) implies {
    let rid = get_register_timer_rid(l2[log_idx]);
    new_by_rid.contains_key(rid) &&
    new_by_rid[rid].2 == log_idx
  } by {
    let rid = get_register_timer_rid(l2[log_idx]);
    if log_idx == n {
      assert(rid == new_rid);
      assert(new_by_rid.contains_key(new_rid));
      assert(new_by_rid[new_rid].2 == n);
    } else {
      assert(log_idx < n);
      assert(l2[log_idx] == l[log_idx]);
      assert(is_succ_register_timer_at(l, log_idx));
      assert(rid != new_rid) by {
        if rid == new_rid {
          assert(no_prior_timer_registration(l, new_rid, n));
          assert(is_succ_register_timer_at(l, log_idx));
          assert(get_register_timer_rid(l[log_idx]) == new_rid);
          assert(0 <= log_idx && log_idx < n);
          let d = choose |d: int| log_idx < d < n && timer_retired_at(l, new_rid, d);
          timer_retired_preserved(l, e, new_rid, d);
          timer_retired_implies(l2, new_rid, d);
          assert(timer_awaiting_wake(l2, log_idx));
          assert(log_idx < d && d < l2.len());
          assert(false);
        }
      }
      assert forall |k: int| log_idx < k < n implies
        !timer_retired_at(l, rid, k)
      by {
        assert(l2[k] == l[k]);
        not_timer_retired_shrink(l, e, rid, k);
      };
      assert(timer_active_at(l, log_idx, n));
      assert forall |k: int| log_idx < k < n implies !(
        is_wake_task_at(l, k) && get_wake_task_source_rid(l[k]) == rid
      ) by {
        not_timer_retired_implies(l, rid, k);
      };
      assert(timer_awaiting_wake(l, log_idx));
      assert(old_by_rid.contains_key(rid));
      assert(old_by_rid[rid].2 == log_idx);
      assert(new_by_rid[rid] == old_by_rid[rid]);
    }
  }
}

#[verifier::rlimit(30)]
pub proof fn timer_wakers_match_after_fresh_register_timer(
  old_timer_wakers: Map<ResourceIdView, WakerView>,
  old_by_rid: Map<ResourceIdView, (InstantView, ResourceIdView, int)>,
  l: Log,
  e: ReactorEvent,
  new_rid: ResourceIdView,
  deadline_val: InstantView,
  waker_val: WakerView,
)
  requires
    timer_wakers_match(old_timer_wakers, old_by_rid, l),
    is_succ_register_timer_at(l.push(e), l.len() as int),
    get_register_timer_rid(l.push(e)[l.len() as int]) == new_rid,
    get_register_timer_deadline(l.push(e)[l.len() as int]) == deadline_val,
    get_register_timer_waker(l.push(e)[l.len() as int]) == waker_val,
    !old_by_rid.contains_key(new_rid),
  ensures
    timer_wakers_match(
      old_timer_wakers.insert(new_rid, waker_val),
      old_by_rid.insert(new_rid, (deadline_val, new_rid, l.len() as int)),
      l.push(e),
    ),
{
  let l2 = l.push(e);
  let n = l.len() as int;
  let new_tw = old_timer_wakers.insert(new_rid, waker_val);
  let new_by_rid = old_by_rid.insert(new_rid, (deadline_val, new_rid, n));

  assert forall |rid: ResourceIdView| #![auto]
    new_tw.contains_key(rid) implies {
      new_by_rid.contains_key(rid) && {
        let log_idx = new_by_rid[rid].2;
        0 <= log_idx < l2.len() &&
        is_succ_register_timer_at(l2, log_idx) &&
        get_register_timer_rid(l2[log_idx]) == rid &&
        new_tw[rid] == get_register_timer_waker(l2[log_idx])
      }
    }
  by {
    if rid == new_rid {
      assert(new_by_rid.contains_key(new_rid));
      let log_idx = new_by_rid[new_rid].2;
      assert(log_idx == n);
      assert(l2[n] == e);
      assert(is_succ_register_timer_at(l2, n));
      assert(get_register_timer_rid(l2[n]) == new_rid);
      assert(new_tw[new_rid] == waker_val);
      assert(get_register_timer_waker(l2[n]) == waker_val);
    } else {
      assert(old_timer_wakers.contains_key(rid));
      assert(new_tw[rid] == old_timer_wakers[rid]);
      assert(new_by_rid[rid] == old_by_rid[rid]);
      let log_idx = old_by_rid[rid].2;
      assert(log_idx < l.len());
      assert(l2[log_idx] == l[log_idx]);
    }
  }
}

pub proof fn timer_heap_has_wakers_after_insert(
  old_timer_wakers: Map<ResourceIdView, WakerView>,
  old_by_rid: Map<ResourceIdView, (InstantView, ResourceIdView, int)>,
  new_rid: ResourceIdView,
  waker_val: WakerView,
  entry: (InstantView, ResourceIdView, int),
)
  requires
    timer_heap_has_wakers(old_timer_wakers, old_by_rid),
  ensures
    timer_heap_has_wakers(
      old_timer_wakers.insert(new_rid, waker_val),
      old_by_rid.insert(new_rid, entry),
    ),
{
  let new_tw = old_timer_wakers.insert(new_rid, waker_val);
  let new_by_rid = old_by_rid.insert(new_rid, entry);
  assert forall |rid: ResourceIdView| #![auto]
    new_by_rid.contains_key(rid) implies new_tw.contains_key(rid)
  by {
    if rid == new_rid {
      assert(new_tw.contains_key(new_rid));
    } else {
      assert(old_by_rid.contains_key(rid));
      assert(old_timer_wakers.contains_key(rid));
    }
  }
}

#[verifier::rlimit(30)]
pub proof fn read_wakers_complete_preserved_by_fresh_register_timer(
  read_wakers_view: Map<ResourceIdView, WakerView>,
  l: Log,
  e: ReactorEvent,
  new_rid: ResourceIdView,
)
  requires
    read_wakers_complete(read_wakers_view, l),
    is_succ_register_timer_at(l.push(e), l.len() as int),
    !io_api_registered_at(l.push(e), l.len() as int),
    !is_succ_set_waker_at(l.push(e), l.len() as int),
  ensures
    read_wakers_complete(read_wakers_view, l.push(e)),
{
  read_wakers_complete_preserved_by_non_trigger(read_wakers_view, l, e);
}

#[verifier::rlimit(30)]
pub proof fn write_wakers_complete_preserved_by_fresh_register_timer(
  write_wakers_view: Map<ResourceIdView, WakerView>,
  l: Log,
  e: ReactorEvent,
  new_rid: ResourceIdView,
)
  requires
    write_wakers_complete(write_wakers_view, l),
    is_succ_register_timer_at(l.push(e), l.len() as int),
    !io_api_registered_at(l.push(e), l.len() as int),
    !is_succ_set_waker_at(l.push(e), l.len() as int),
  ensures
    write_wakers_complete(write_wakers_view, l.push(e)),
{
  write_wakers_complete_preserved_by_non_trigger(write_wakers_view, l, e);
}

#[verifier::rlimit(30)]
pub proof fn data_inv_preserved_by_fresh_register_timer(
  old_timers: Set<(InstantView, ResourceIdView, int)>,
  old_by_rid: Map<ResourceIdView, (InstantView, ResourceIdView, int)>,
  old_timer_wakers: Map<ResourceIdView, WakerView>,
  read_wakers: Map<ResourceIdView, WakerView>,
  write_wakers: Map<ResourceIdView, WakerView>,
  l: Log,
  e: ReactorEvent,
  new_rid: ResourceIdView,
  deadline_val: InstantView,
  waker_val: WakerView,
)
  requires
    data_inv(old_timers, old_by_rid, old_timer_wakers, read_wakers, write_wakers, l),
    is_succ_register_timer_at(l.push(e), l.len() as int),
    get_register_timer_rid(l.push(e)[l.len() as int]) == new_rid,
    get_register_timer_deadline(l.push(e)[l.len() as int]) == deadline_val,
    get_register_timer_waker(l.push(e)[l.len() as int]) == waker_val,
    no_prior_timer_registration(l, new_rid, l.len() as int),
    !old_by_rid.contains_key(new_rid),
    !is_deregister_timer_at(l.push(e), l.len() as int),
    !is_wake_task_at(l.push(e), l.len() as int),
    !is_succ_set_waker_at(l.push(e), l.len() as int),
    !io_api_registered_at(l.push(e), l.len() as int),
    old_timers.finite(),
  ensures
    data_inv(
      old_timers.insert((deadline_val, new_rid, l.len() as int)),
      old_by_rid.insert(new_rid, (deadline_val, new_rid, l.len() as int)),
      old_timer_wakers.insert(new_rid, waker_val),
      read_wakers,
      write_wakers,
      l.push(e),
    ),
{
  let log_idx = l.len() as int;
  let new_entry = (deadline_val, new_rid, log_idx);

  timer_heap_entries_valid_after_fresh_register_timer(old_timers, l, e, new_rid, deadline_val);
  active_timers_in_heap_after_fresh_register_timer(old_by_rid, l, e, new_rid, deadline_val);
  timer_wakers_match_after_fresh_register_timer(old_timer_wakers, old_by_rid, l, e, new_rid, deadline_val, waker_val);
  timer_heap_has_wakers_after_insert(old_timer_wakers, old_by_rid, new_rid, waker_val, new_entry);
  read_wakers_valid_preserved_by_non_set_waker(read_wakers, l, e);
  write_wakers_valid_preserved_by_non_set_waker(write_wakers, l, e);
  read_wakers_complete_preserved_by_fresh_register_timer(read_wakers, l, e, new_rid);
  write_wakers_complete_preserved_by_fresh_register_timer(write_wakers, l, e, new_rid);
}

pub proof fn timer_heap_entries_valid_after_deregister(
  old_timers: Set<(InstantView, ResourceIdView, int)>,
  new_timers: Set<(InstantView, ResourceIdView, int)>,
  removed_rid: ResourceIdView,
  l: Log,
  begin_event: ReactorEvent,
)
  requires
    timer_heap_entries_valid(old_timers, l),
    is_succ_deregister_timer_at(l.push(begin_event), l.len() as int),
    get_deregister_timer_rid(l.push(begin_event)[l.len() as int]) == removed_rid,
    !is_wake_task_at(l.push(begin_event), l.len() as int),
    forall |d: InstantView, r: ResourceIdView, i: int|
      #![auto] new_timers.contains((d, r, i)) ==> old_timers.contains((d, r, i)) && r != removed_rid,
  ensures
    timer_heap_entries_valid(new_timers, l.push(begin_event)),
{
  let l2 = l.push(begin_event);
  let n = l.len() as int;
  assert forall |d: InstantView, rid: ResourceIdView, log_idx: int|
    #![auto] new_timers.contains((d, rid, log_idx)) implies {
      timer_awaiting_wake(l2, log_idx) &&
      get_register_timer_rid(l2[log_idx]) == rid &&
      get_register_timer_deadline(l2[log_idx]) == d
    }
  by {
    assert(old_timers.contains((d, rid, log_idx)));
    assert(rid != removed_rid);
    assert(timer_awaiting_wake(l, log_idx));
    assert(log_idx < n);
    assert(l2[log_idx] == l[log_idx]);
    assert(is_succ_register_timer_at(l2, log_idx));
    let the_rid = get_register_timer_rid(l2[log_idx]);
    assert forall |k: int| log_idx < k < l2.len() implies
      !timer_retired_at(l2, the_rid, k)
    by {
      if k < n {
        assert(l2[k] == l[k]);
        not_timer_retired_preserved(l, begin_event, the_rid, k);
      } else {
        assert(k == n);
        assert(get_deregister_timer_rid(l2[n]) == removed_rid);
        assert(the_rid == rid);
        assert(rid != removed_rid);
        assert(!is_wake_task_at(l2, n));
        not_timer_retired(l2, the_rid, k);
      }
    };
    assert(timer_active_at(l2, log_idx, l2.len() as int));
    assert forall |k: int| log_idx < k < l2.len() implies !(
      is_wake_task_at(l2, k) && get_wake_task_source_rid(l2[k]) == the_rid
    ) by {
      not_timer_retired_implies(l2, the_rid, k);
    };
    assert(timer_awaiting_wake(l2, log_idx));
  }
}

pub proof fn timer_heap_has_wakers_after_remove(
  old_timer_wakers: Map<ResourceIdView, WakerView>,
  old_by_rid: Map<ResourceIdView, (InstantView, ResourceIdView, int)>,
  removed_rid: ResourceIdView,
)
  requires
    timer_heap_has_wakers(old_timer_wakers, old_by_rid),
  ensures
    timer_heap_has_wakers(
      old_timer_wakers.remove(removed_rid),
      old_by_rid.remove(removed_rid),
    ),
{
  let new_tw = old_timer_wakers.remove(removed_rid);
  let new_by_rid = old_by_rid.remove(removed_rid);
  assert forall |rid: ResourceIdView| #![auto]
    new_by_rid.contains_key(rid) implies new_tw.contains_key(rid)
  by {
    assert(old_by_rid.contains_key(rid));
    assert(old_timer_wakers.contains_key(rid));
    assert(rid != removed_rid);
  }
}

pub proof fn timer_wakers_match_after_remove(
  old_timer_wakers: Map<ResourceIdView, WakerView>,
  old_by_rid: Map<ResourceIdView, (InstantView, ResourceIdView, int)>,
  removed_rid: ResourceIdView,
  l: Log,
)
  requires
    timer_wakers_match(old_timer_wakers, old_by_rid, l),
  ensures
    timer_wakers_match(
      old_timer_wakers.remove(removed_rid),
      old_by_rid.remove(removed_rid),
      l,
    ),
{
  let new_tw = old_timer_wakers.remove(removed_rid);
  let new_by_rid = old_by_rid.remove(removed_rid);
  assert forall |rid: ResourceIdView| #![auto]
    new_tw.contains_key(rid) implies {
      new_by_rid.contains_key(rid) && {
        let log_idx = new_by_rid[rid].2;
        0 <= log_idx < l.len() &&
        is_succ_register_timer_at(l, log_idx) &&
        get_register_timer_rid(l[log_idx]) == rid &&
        new_tw[rid] == get_register_timer_waker(l[log_idx])
      }
    }
  by {
    assert(old_timer_wakers.contains_key(rid));
    assert(rid != removed_rid);
    assert(new_tw[rid] == old_timer_wakers[rid]);
    assert(new_by_rid[rid] == old_by_rid[rid]);
  }
}

#[verifier::rlimit(50)]
pub proof fn active_timers_in_heap_after_remove(
  old_by_rid: Map<ResourceIdView, (InstantView, ResourceIdView, int)>,
  removed_rid: ResourceIdView,
  l: Log,
  begin_event: ReactorEvent,
)
  requires
    active_timers_in_heap(old_by_rid, l),
    is_succ_deregister_timer_at(l.push(begin_event), l.len() as int),
    get_deregister_timer_rid(l.push(begin_event)[l.len() as int]) == removed_rid,
  ensures
    active_timers_in_heap(
      old_by_rid.remove(removed_rid),
      l.push(begin_event),
    ),
{
  let l2 = l.push(begin_event);
  let n = l.len() as int;
  let new_by_rid = old_by_rid.remove(removed_rid);
  assert forall |log_idx: int| #![auto] timer_awaiting_wake(l2, log_idx) implies {
    let rid = get_register_timer_rid(l2[log_idx]);
    new_by_rid.contains_key(rid) &&
    new_by_rid[rid].2 == log_idx
  } by {
    let rid = get_register_timer_rid(l2[log_idx]);
    assert(log_idx < n);
    assert(l2[log_idx] == l[log_idx]);
    assert(is_succ_register_timer_at(l, log_idx));
    assert(timer_active_at(l2, log_idx, l2.len() as int));
    not_timer_retired_implies(l2, rid, n as int);
    assert(rid != removed_rid);
    assert forall |k: int| log_idx < k < n implies
      !timer_retired_at(l, rid, k)
    by {
      assert(l2[k] == l[k]);
      not_timer_retired_shrink(l, begin_event, rid, k);
    };
    assert(timer_active_at(l, log_idx, n));
    assert forall |k: int| log_idx < k < n implies !(
      is_wake_task_at(l, k) && get_wake_task_source_rid(l[k]) == rid
    ) by {
      not_timer_retired_implies(l, rid, k);
    };
    assert(timer_awaiting_wake(l, log_idx));
    assert(old_by_rid.contains_key(rid));
    assert(old_by_rid[rid].2 == log_idx);
    assert(new_by_rid[rid] == old_by_rid[rid]);
  }
}

pub proof fn data_inv_preserved_by_deregister_timer(
  old_timers: Set<(InstantView, ResourceIdView, int)>,
  new_timers: Set<(InstantView, ResourceIdView, int)>,
  old_by_rid: Map<ResourceIdView, (InstantView, ResourceIdView, int)>,
  old_timer_wakers: Map<ResourceIdView, WakerView>,
  read_wakers: Map<ResourceIdView, WakerView>,
  write_wakers: Map<ResourceIdView, WakerView>,
  l: Log,
  begin_event: ReactorEvent,
  removed_rid: ResourceIdView,
)
  requires
    data_inv(old_timers, old_by_rid, old_timer_wakers, read_wakers, write_wakers, l),
    is_succ_deregister_timer_at(l.push(begin_event), l.len() as int),
    get_deregister_timer_rid(l.push(begin_event)[l.len() as int]) == removed_rid,
    !is_succ_register_timer_at(l.push(begin_event), l.len() as int),
    !is_wake_task_at(l.push(begin_event), l.len() as int),
    !is_succ_set_waker_at(l.push(begin_event), l.len() as int),
    !io_api_registered_at(l.push(begin_event), l.len() as int),
    forall |d: InstantView, r: ResourceIdView, i: int|
      #![auto] new_timers.contains((d, r, i)) ==> old_timers.contains((d, r, i)) && r != removed_rid,
    forall |d: InstantView, r: ResourceIdView, i: int|
      #![auto] old_timers.contains((d, r, i)) && r != removed_rid ==> new_timers.contains((d, r, i)),
  ensures
    data_inv(
      new_timers,
      old_by_rid.remove(removed_rid),
      old_timer_wakers.remove(removed_rid),
      read_wakers,
      write_wakers,
      l.push(begin_event),
    ),
{
  timer_heap_entries_valid_after_deregister(old_timers, new_timers, removed_rid, l, begin_event);

  active_timers_in_heap_after_remove(old_by_rid, removed_rid, l, begin_event);

  timer_wakers_match_after_remove(old_timer_wakers, old_by_rid, removed_rid, l);
  timer_wakers_match_preserved_by_append(
    old_timer_wakers.remove(removed_rid),
    old_by_rid.remove(removed_rid),
    l, begin_event,
  );

  timer_heap_has_wakers_after_remove(old_timer_wakers, old_by_rid, removed_rid);

  read_wakers_valid_preserved_by_non_set_waker(read_wakers, l, begin_event);
  write_wakers_valid_preserved_by_non_set_waker(write_wakers, l, begin_event);
  read_wakers_complete_preserved_by_non_trigger(read_wakers, l, begin_event);
  write_wakers_complete_preserved_by_non_trigger(write_wakers, l, begin_event);
}

}
