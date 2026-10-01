use vstd::prelude::*;
use crate::spec::log::*;
use crate::spec::types::*;
use crate::spec::predicates::*;
use crate::invariants::*;
use crate::invariants::timer_waker_validity::*;
use crate::invariants::io_waker_validity::*;
use crate::invariants::data_inv::*;

verus! {

proof fn least_timeout_exists(l: Log, register_idx: int, t: int)
  requires
    t > register_idx,
    is_get_current_time_at(l, t),
    get_current_timestamp(l[t]) >= get_register_timer_deadline(l[register_idx]),
    timer_active_at(l, register_idx, t),
  ensures
    exists |min_t: int| min_t > register_idx &&
      is_get_current_time_at(l, min_t) &&
      get_current_timestamp(l[min_t]) >= get_register_timer_deadline(l[register_idx]) &&
      timer_active_at(l, register_idx, min_t) &&
      forall |k: int| register_idx < k < min_t ==> !(
        is_get_current_time_at(l, k) &&
        get_current_timestamp(l[k]) >= get_register_timer_deadline(l[register_idx]) &&
        timer_active_at(l, register_idx, k)
      ),
  decreases t - register_idx,
{
  if forall |k: int| register_idx < k < t ==> !(
    is_get_current_time_at(l, k) &&
    get_current_timestamp(l[k]) >= get_register_timer_deadline(l[register_idx]) &&
    timer_active_at(l, register_idx, k)
  ) {
    assert(t > register_idx &&
      is_get_current_time_at(l, t) &&
      get_current_timestamp(l[t]) >= get_register_timer_deadline(l[register_idx]) &&
      timer_active_at(l, register_idx, t) &&
      forall |k: int| register_idx < k < t ==> !(
        is_get_current_time_at(l, k) &&
        get_current_timestamp(l[k]) >= get_register_timer_deadline(l[register_idx]) &&
        timer_active_at(l, register_idx, k)
      ));
  } else {
    let k0 = choose |k: int| register_idx < k < t &&
      is_get_current_time_at(l, k) &&
      get_current_timestamp(l[k]) >= get_register_timer_deadline(l[register_idx]) &&
      timer_active_at(l, register_idx, k);
    least_timeout_exists(l, register_idx, k0);
  }
}

proof fn greatest_set_waker_readable_exists(l: Log, rid: ResourceIdView, before: int, t: int)
  requires
    0 <= t < before,
    is_succ_set_waker_at(l, t),
    get_set_waker_rid(l[t]) == rid,
    get_set_waker_interest(l[t]).0,
  ensures
    exists |max_t: int| 0 <= max_t < before &&
      is_succ_set_waker_at(l, max_t) &&
      get_set_waker_rid(l[max_t]) == rid &&
      get_set_waker_interest(l[max_t]).0 &&
      forall |k: int| max_t < k < before ==> !(
        is_succ_set_waker_at(l, k) &&
        get_set_waker_rid(l[k]) == rid &&
        get_set_waker_interest(l[k]).0
      ),
  decreases before - t,
{
  if forall |k: int| t < k < before ==> !(
    is_succ_set_waker_at(l, k) &&
    get_set_waker_rid(l[k]) == rid &&
    get_set_waker_interest(l[k]).0
  ) {
    assert(0 <= t && t < before &&
      is_succ_set_waker_at(l, t) &&
      get_set_waker_rid(l[t]) == rid &&
      get_set_waker_interest(l[t]).0 &&
      forall |k: int| t < k < before ==> !(
        is_succ_set_waker_at(l, k) &&
        get_set_waker_rid(l[k]) == rid &&
        get_set_waker_interest(l[k]).0
      ));
  } else {
    let k0 = choose |k: int| t < k < before &&
      is_succ_set_waker_at(l, k) &&
      get_set_waker_rid(l[k]) == rid &&
      get_set_waker_interest(l[k]).0;
    greatest_set_waker_readable_exists(l, rid, before, k0);
  }
}

proof fn find_last_set_waker_readable_properties(l: Log, rid: ResourceIdView, before: int)
  requires
    exists |j: int| 0 <= j < before &&
      is_succ_set_waker_at(l, j) &&
      get_set_waker_rid(l[j]) == rid &&
      get_set_waker_interest(l[j]).0,
  ensures ({
    let sw = find_last_set_waker_for_rid_readable(l, rid, before);
    0 <= sw && sw < before &&
    is_succ_set_waker_at(l, sw) &&
    get_set_waker_rid(l[sw]) == rid &&
    get_set_waker_interest(l[sw]).0 &&
    forall |k: int| sw < k < before ==> !(
      is_succ_set_waker_at(l, k) &&
      get_set_waker_rid(l[k]) == rid &&
      get_set_waker_interest(l[k]).0
    )
  })
{
  let t0 = choose |j: int| 0 <= j < before &&
    is_succ_set_waker_at(l, j) &&
    get_set_waker_rid(l[j]) == rid &&
    get_set_waker_interest(l[j]).0;
  greatest_set_waker_readable_exists(l, rid, before, t0);
}

proof fn greatest_set_waker_writable_exists(l: Log, rid: ResourceIdView, before: int, t: int)
  requires
    0 <= t < before,
    is_succ_set_waker_at(l, t),
    get_set_waker_rid(l[t]) == rid,
    get_set_waker_interest(l[t]).1,
  ensures
    exists |max_t: int| 0 <= max_t < before &&
      is_succ_set_waker_at(l, max_t) &&
      get_set_waker_rid(l[max_t]) == rid &&
      get_set_waker_interest(l[max_t]).1 &&
      forall |k: int| max_t < k < before ==> !(
        is_succ_set_waker_at(l, k) &&
        get_set_waker_rid(l[k]) == rid &&
        get_set_waker_interest(l[k]).1
      ),
  decreases before - t,
{
  if forall |k: int| t < k < before ==> !(
    is_succ_set_waker_at(l, k) &&
    get_set_waker_rid(l[k]) == rid &&
    get_set_waker_interest(l[k]).1
  ) {
    assert(0 <= t && t < before &&
      is_succ_set_waker_at(l, t) &&
      get_set_waker_rid(l[t]) == rid &&
      get_set_waker_interest(l[t]).1 &&
      forall |k: int| t < k < before ==> !(
        is_succ_set_waker_at(l, k) &&
        get_set_waker_rid(l[k]) == rid &&
        get_set_waker_interest(l[k]).1
      ));
  } else {
    let k0 = choose |k: int| t < k < before &&
      is_succ_set_waker_at(l, k) &&
      get_set_waker_rid(l[k]) == rid &&
      get_set_waker_interest(l[k]).1;
    greatest_set_waker_writable_exists(l, rid, before, k0);
  }
}

proof fn find_last_set_waker_writable_properties(l: Log, rid: ResourceIdView, before: int)
  requires
    exists |j: int| 0 <= j < before &&
      is_succ_set_waker_at(l, j) &&
      get_set_waker_rid(l[j]) == rid &&
      get_set_waker_interest(l[j]).1,
  ensures ({
    let sw = find_last_set_waker_for_rid_writable(l, rid, before);
    0 <= sw && sw < before &&
    is_succ_set_waker_at(l, sw) &&
    get_set_waker_rid(l[sw]) == rid &&
    get_set_waker_interest(l[sw]).1 &&
    forall |k: int| sw < k < before ==> !(
      is_succ_set_waker_at(l, k) &&
      get_set_waker_rid(l[k]) == rid &&
      get_set_waker_interest(l[k]).1
    )
  })
{
  let t0 = choose |j: int| 0 <= j < before &&
    is_succ_set_waker_at(l, j) &&
    get_set_waker_rid(l[j]) == rid &&
    get_set_waker_interest(l[j]).1;
  greatest_set_waker_writable_exists(l, rid, before, t0);
}

pub proof fn first_timeout_point_properties(l: Log, register_idx: int)
  requires has_timeout_point(l, register_idx),
  ensures ({
    let tp = first_timeout_point(l, register_idx);
    tp > register_idx &&
    is_get_current_time_at(l, tp) &&
    get_current_timestamp(l[tp]) >= get_register_timer_deadline(l[register_idx]) &&
    timer_active_at(l, register_idx, tp) &&
    forall |k: int| register_idx < k < tp ==> !(
      is_get_current_time_at(l, k) &&
      get_current_timestamp(l[k]) >= get_register_timer_deadline(l[register_idx]) &&
      timer_active_at(l, register_idx, k)
    )
  })
{
  let t0 = choose |t: int| t > register_idx &&
    is_get_current_time_at(l, t) &&
    get_current_timestamp(l[t]) >= get_register_timer_deadline(l[register_idx]) &&
    timer_active_at(l, register_idx, t);
  least_timeout_exists(l, register_idx, t0);
}

#[verifier::rlimit(30)]
proof fn flat_r5_preserved(l: Log, e: ReactorEvent, i: int)
  requires
    reactor_inv(l),
    !is_wake_task_at(l.push(e), l.len() as int),
    is_timer_wake_at(l.push(e), i),
    i < l.len() as int,
  ensures ({
    let l2 = l.push(e);
    let rid = get_wake_task_source_rid(l2[i]);
    let waker = get_wake_task_waker(l2[i]);
    exists |j: int| 0 <= j < i &&
      is_succ_register_timer_at(l2, j) &&
      get_register_timer_rid(l2[j]) == rid &&
      get_register_timer_waker(l2[j]) == waker &&
      timer_active_at(l2, j, i)
  })
{
  let l2 = l.push(e);
  let n = l.len() as int;
  assert(l2[i] == l[i]);
  assert(is_wake_task_at(l, i));

  let j_wit = choose |j: int| #![trigger is_succ_register_timer_at(l2, j)]
    0 <= j < i &&
    is_succ_register_timer_at(l2, j) &&
    get_register_timer_rid(l2[j]) == get_wake_task_source_rid(l2[i]) &&
    timer_active_at(l2, j, i);
  assert(j_wit < n);
  assert(l2[j_wit] == l[j_wit]);
  assert(is_succ_register_timer_at(l, j_wit));
  assert(get_register_timer_rid(l[j_wit]) == get_wake_task_source_rid(l[i]));
  assert(timer_active_at(l2, j_wit, i));
  let rid_wit = get_register_timer_rid(l2[j_wit]);
  assert forall |k: int| j_wit < k < i implies
    !timer_retired_at(l, rid_wit, k)
  by {
    assert(k < n);
    assert(l2[k] == l[k]);
    not_timer_retired_transfer(l2, l, rid_wit, k);
  };
  assert(timer_active_at(l, j_wit, i));
  assert(is_timer_wake_at(l, i));

  let rid = get_wake_task_source_rid(l[i]);
  let waker = get_wake_task_waker(l[i]);
  let j0 = choose |j: int| #![trigger is_succ_register_timer_at(l, j)]
    0 <= j < i &&
    is_succ_register_timer_at(l, j) &&
    get_register_timer_rid(l[j]) == rid &&
    get_register_timer_waker(l[j]) == waker &&
    timer_active_at(l, j, i);
  assert(j0 < n);
  assert(l2[j0] == l[j0]);
  assert(is_succ_register_timer_at(l2, j0));
  assert(get_register_timer_rid(l2[j0]) == rid);
  assert(get_register_timer_waker(l2[j0]) == waker);
  assert forall |k: int| j0 < k < i implies
    !timer_retired_at(l2, rid, k)
  by {
    assert(k < n);
    assert(l2[k] == l[k]);
    not_timer_retired_transfer(l, l2, rid, k);
  };
}

#[verifier::rlimit(30)]
proof fn flat_r6_preserved(l: Log, e: ReactorEvent, i: int)
  requires
    reactor_inv(l),
    !is_wake_task_at(l.push(e), l.len() as int),
    is_io_api_wake_at(l.push(e), i),
    i < l.len() as int,
  ensures ({
    let l2 = l.push(e);
    let rid = get_wake_task_source_rid(l2[i]);
    let waker = get_wake_task_waker(l2[i]);
    exists |sw_idx: int| 0 <= sw_idx < i &&
      is_succ_set_waker_at(l2, sw_idx) &&
      get_set_waker_rid(l2[sw_idx]) == rid &&
      get_set_waker_waker(l2[sw_idx]) == waker &&
      io_api_active_at_set_waker(l2, rid, sw_idx)
  })
{
  let l2 = l.push(e);
  let n = l.len() as int;
  assert(l2[i] == l[i]);
  assert(is_wake_task_at(l, i));

  let j_wit = choose |j: int| #![trigger io_api_registered_at(l2, j)]
    0 <= j < i &&
    io_api_registered_at(l2, j) &&
    get_io_api_register_rid(l2[j]) == get_wake_task_source_rid(l2[i]) &&
    io_api_active_at(l2, j, i);
  assert(j_wit < n);
  assert(l2[j_wit] == l[j_wit]);
  assert(io_api_registered_at(l, j_wit));
  assert(get_io_api_register_rid(l[j_wit]) == get_wake_task_source_rid(l[i]));
  assert(io_api_active_at(l2, j_wit, i));
  assert forall |k: int| j_wit < k < i implies !(
    io_api_deregistered_at(l, k) &&
    get_io_api_deregister_rid(l[k]) == get_io_api_register_rid(l[j_wit])
  ) by {
    assert(k < n);
    assert(l2[k] == l[k]);
    assert(l2[j_wit] == l[j_wit]);
    if io_api_deregistered_at(l, k) && get_io_api_deregister_rid(l[k]) == get_io_api_register_rid(l[j_wit]) {
      assert(io_api_deregistered_at(l2, k));
      assert(get_io_api_deregister_rid(l2[k]) == get_io_api_register_rid(l2[j_wit]));
    }
  };
  assert(io_api_active_at(l, j_wit, i));
  assert(is_io_api_wake_at(l, i));

  // From old R6 on l
  let rid = get_wake_task_source_rid(l[i]);
  let waker = get_wake_task_waker(l[i]);
  let sw = choose |sw_idx: int| #![trigger is_succ_set_waker_at(l, sw_idx)]
    0 <= sw_idx < i &&
    is_succ_set_waker_at(l, sw_idx) &&
    get_set_waker_rid(l[sw_idx]) == rid &&
    get_set_waker_waker(l[sw_idx]) == waker &&
    io_api_active_at_set_waker(l, rid, sw_idx);
  assert(sw < n);
  assert(l2[sw] == l[sw]);
  assert(is_succ_set_waker_at(l2, sw));
  assert(get_set_waker_rid(l2[sw]) == rid);
  assert(get_set_waker_waker(l2[sw]) == waker);

  // Transport io_api_active_at_set_waker(l, rid, sw) to (l2, rid, sw)
  let reg = choose |reg_idx: int| #![trigger io_api_registered_at(l, reg_idx)]
    0 <= reg_idx < sw &&
    io_api_registered_at(l, reg_idx) &&
    get_io_api_register_rid(l[reg_idx]) == rid &&
    io_api_active_at(l, reg_idx, sw);
  assert(reg < n);
  assert(l2[reg] == l[reg]);
  assert(io_api_registered_at(l2, reg));
  assert(get_io_api_register_rid(l2[reg]) == rid);
  // Transport io_api_active_at(l, reg, sw) to (l2, reg, sw)
  assert forall |k: int| reg < k < sw implies !(
    io_api_deregistered_at(l2, k) &&
    get_io_api_deregister_rid(l2[k]) == get_io_api_register_rid(l2[reg])
  ) by {
    assert(k < n);
    assert(l2[k] == l[k]);
    if io_api_deregistered_at(l2, k) {
      assert(io_api_deregistered_at(l, k));
    }
  };
}

#[verifier::rlimit(20)]
proof fn flat_r14_preserved(l: Log, e: ReactorEvent, i: int)
  requires
    reactor_inv(l),
    !is_wake_task_at(l.push(e), l.len() as int),
    is_wake_task_at(l.push(e), i),
    i < l.len() as int,
  ensures ({
    let l2 = l.push(e);
    let rid = get_wake_task_source_rid(l2[i]);
    (exists |j: int| 0 <= j < i &&
      is_succ_register_timer_at(l2, j) &&
      get_register_timer_rid(l2[j]) == rid)
    ||
    (exists |j: int| 0 <= j < i &&
      io_api_registered_at(l2, j) &&
      get_io_api_register_rid(l2[j]) == rid)
  })
{
  let l2 = l.push(e);
  let n = l.len() as int;
  assert(l2[i] == l[i]);
  assert(is_wake_task_at(l, i));
  let rid = get_wake_task_source_rid(l[i]);

  // From old R14 on l, we have the disjunction
  if exists |j: int| 0 <= j < i &&
    is_succ_register_timer_at(l, j) &&
    get_register_timer_rid(l[j]) == rid
  {
    let j = choose |j: int| #![trigger is_succ_register_timer_at(l, j)]
      0 <= j < i &&
      is_succ_register_timer_at(l, j) &&
      get_register_timer_rid(l[j]) == rid;
    assert(j < n);
    assert(l2[j] == l[j]);
    assert(is_succ_register_timer_at(l2, j));
    assert(get_register_timer_rid(l2[j]) == rid);
  } else {
    let j = choose |j: int| #![trigger io_api_registered_at(l, j)]
      0 <= j < i &&
      io_api_registered_at(l, j) &&
      get_io_api_register_rid(l[j]) == rid;
    assert(j < n);
    assert(l2[j] == l[j]);
    assert(io_api_registered_at(l2, j));
    assert(get_io_api_register_rid(l2[j]) == rid);
  }
}

#[verifier::rlimit(30)]
proof fn flat_r16_preserved(l: Log, e: ReactorEvent, i: int)
  requires
    reactor_inv(l),
    !is_wake_task_at(l.push(e), l.len() as int),
    !is_get_current_time_at(l.push(e), l.len() as int),
    is_succ_register_timer_at(l.push(e), i),
    has_timeout_point(l.push(e), i),
    timer_awaiting_wake(l.push(e), i),
    i < l.len() as int,
  ensures ({
    let l2 = l.push(e);
    exists |j: int| j > i &&
      is_wake_task_at(l2, j) &&
      get_wake_task_source_rid(l2[j]) == get_register_timer_rid(l2[i]) &&
      get_wake_task_waker(l2[j]) == get_register_timer_waker(l2[i]) && {
      let timeout_idx = first_timeout_point(l2, i);
      j > timeout_idx &&
      forall |k: int| timeout_idx < k < j ==> !is_park_end_at(l2, k)
    }
  })
{
  let l2 = l.push(e);
  let n = l.len() as int;
  assert(l2[i] == l[i]);
  assert(is_succ_register_timer_at(l, i));

  // Transport has_timeout_point(l2, i) to has_timeout_point(l, i)
  // has_timeout_point(l2, i) = exists |tp| tp > i && is_get_current_time_at(l2, tp) && ...
  // Since !is_get_current_time_at(l2, n), tp must be < n
  let tp = choose |timeout_idx: int| timeout_idx > i &&
    is_get_current_time_at(l2, timeout_idx) &&
    get_current_timestamp(l2[timeout_idx]) >= get_register_timer_deadline(l2[i]) &&
    timer_active_at(l2, i, timeout_idx);
  assert(is_get_current_time_at(l2, tp));
  assert(tp != n as int); // because !is_get_current_time_at(l2, n)
  assert(tp < n);
  assert(l2[tp] == l[tp]);
  assert(is_get_current_time_at(l, tp));
  assert(get_current_timestamp(l[tp]) >= get_register_timer_deadline(l[i]));
  let rid = get_register_timer_rid(l[i]);
  assert forall |k: int| i < k < tp implies
    !timer_retired_at(l, rid, k)
  by {
    assert(k < n);
    assert(l2[k] == l[k]);
    not_timer_retired_transfer(l2, l, rid, k);
  };
  assert(has_timeout_point(l, i));

  assert(timer_active_at(l2, i, l2.len() as int));
  assert forall |k: int| i < k < n implies
    !timer_retired_at(l, rid, k)
  by {
    assert(k < l2.len());
    assert(l2[k] == l[k]);
    not_timer_retired_transfer(l2, l, rid, k);
  };
  assert(timer_active_at(l, i, n));
  assert forall |k: int| i < k < n implies !(
    is_wake_task_at(l, k) && get_wake_task_source_rid(l[k]) == rid
  ) by {
    not_timer_retired_implies(l, rid, k);
  };
  assert(timer_awaiting_wake(l, i));

  // From old R16 on l
  let j0 = choose |j: int| j > i &&
    is_wake_task_at(l, j) &&
    get_wake_task_source_rid(l[j]) == get_register_timer_rid(l[i]) &&
    get_wake_task_waker(l[j]) == get_register_timer_waker(l[i]) && {
    let timeout_idx = first_timeout_point(l, i);
    j > timeout_idx &&
    forall |k: int| timeout_idx < k < j ==> !is_park_end_at(l, k)
  };
  assert(is_wake_task_at(l, j0));
  assert(j0 < n);
  assert(l2[j0] == l[j0]);
  assert(is_wake_task_at(l2, j0));
  assert(get_wake_task_source_rid(l2[j0]) == get_register_timer_rid(l2[i]));
  assert(get_wake_task_waker(l2[j0]) == get_register_timer_waker(l2[i]));

  // Establish properties of first_timeout_point(l, i)
  first_timeout_point_properties(l, i);
  let tp1 = first_timeout_point(l, i);
  assert(tp1 > i);
  assert(tp1 < n);
  assert(l2[tp1] == l[tp1]);

  // Show tp1 satisfies the defining predicate of first_timeout_point on l2
  assert(is_get_current_time_at(l2, tp1));
  assert(get_current_timestamp(l2[tp1]) >= get_register_timer_deadline(l2[i]));
  assert forall |k: int| i < k < tp1 implies !(
    is_deregister_timer_at(l2, k) &&
    get_deregister_timer_rid(l2[k]) == get_register_timer_rid(l2[i])
  ) by {
    assert(k < n);
    assert(l2[k] == l[k]);
  };
  assert(timer_active_at(l2, i, tp1));
  assert forall |k: int| i < k < tp1 implies !(
    is_get_current_time_at(l2, k) &&
    get_current_timestamp(l2[k]) >= get_register_timer_deadline(l2[i]) &&
    timer_active_at(l2, i, k)
  ) by {
    assert(k < n);
    assert(l2[k] == l[k]);
    assert(l2[i] == l[i]);
    if is_get_current_time_at(l2, k) &&
       get_current_timestamp(l2[k]) >= get_register_timer_deadline(l2[i]) &&
       timer_active_at(l2, i, k)
    {
      assert(is_get_current_time_at(l, k));
      assert(get_current_timestamp(l[k]) >= get_register_timer_deadline(l[i]));
      assert forall |j: int| i < j < k implies !(
        is_deregister_timer_at(l, j) &&
        get_deregister_timer_rid(l[j]) == get_register_timer_rid(l[i])
      ) by {
        assert(j < n);
        assert(l2[j] == l[j]);
      };
      assert(timer_active_at(l, i, k));
    }
  };

  // Similarly, establish first_timeout_point(l2, i) properties
  first_timeout_point_properties(l2, i);
  let tp2 = first_timeout_point(l2, i);
  assert(tp2 > i);
  // tp2 < n because !is_get_current_time_at(l2, n)
  assert(tp2 < n);

  // Both tp1 and tp2 are minimum timeout points on l2 → they must be equal
  assert(tp1 == tp2);

  assert(j0 > tp2);
  assert forall |k: int| tp2 < k < j0 implies !is_park_end_at(l2, k) by {
    assert(k < n);
    assert(l2[k] == l[k]);
    assert(!is_park_end_at(l, k));
  };
}

#[verifier::rlimit(30)]
proof fn flat_r17a_preserved(l: Log, e: ReactorEvent, i: int)
  requires
    reactor_inv(l),
    !is_wake_task_at(l.push(e), l.len() as int),
    is_io_event_ready_at(l.push(e), i),
    has_valid_set_waker_readable_api(l.push(e), i),
    i < l.len() as int,
  ensures ({
    let l2 = l.push(e);
    exists |j: int| #![trigger is_wake_task_at(l2, j)] j > i && {
      let event = get_io_event(l2[i]);
      let rid = event.resource_id;
      let sw_idx = find_last_set_waker_for_rid_readable(l2, rid, i);
      let waker = get_set_waker_waker(l2[sw_idx]);
      is_wake_task_at(l2, j) &&
      get_wake_task_source_rid(l2[j]) == rid &&
      get_wake_task_waker(l2[j]) == waker &&
      j > i &&
      forall |k: int| i < k < j ==> !is_park_end_at(l2, k) && !is_poll_events_at(l2, k)
    }
  })
{
  let l2 = l.push(e);
  let n = l.len() as int;
  assert(l2[i] == l[i]);
  assert(is_io_event_ready_at(l, i));

  let rid_l = get_io_event(l[i]).resource_id;

  // Transport has_valid_set_waker_readable_api from l2 to l
  {
    let sw_w = choose |sw_idx: int| 0 <= sw_idx < i &&
      is_succ_set_waker_at(l2, sw_idx) &&
      get_set_waker_rid(l2[sw_idx]) == rid_l &&
      get_set_waker_interest(l2[sw_idx]).0 &&
      io_api_active_at_set_waker(l2, rid_l, sw_idx) &&
      forall |k: int| sw_idx < k < i ==> !(
        io_api_deregistered_at(l2, k) && get_io_api_deregister_rid(l2[k]) == rid_l
      );
    assert(sw_w < n);
    assert(l2[sw_w] == l[sw_w]);
    assert(is_succ_set_waker_at(l, sw_w));
    assert(get_set_waker_rid(l[sw_w]) == rid_l);
    assert(get_set_waker_interest(l[sw_w]).0);
    // Transport io_api_active_at_set_waker
    let reg_w = choose |reg_idx: int| 0 <= reg_idx < sw_w &&
      io_api_registered_at(l2, reg_idx) &&
      get_io_api_register_rid(l2[reg_idx]) == rid_l &&
      io_api_active_at(l2, reg_idx, sw_w);
    assert(reg_w < n);
    assert(l2[reg_w] == l[reg_w]);
    assert(io_api_registered_at(l, reg_w));
    assert(get_io_api_register_rid(l[reg_w]) == rid_l);
    assert forall |k: int| reg_w < k < sw_w implies !(
      io_api_deregistered_at(l, k) && get_io_api_deregister_rid(l[k]) == get_io_api_register_rid(l[reg_w])
    ) by {
      assert(k < n);
      assert(l2[k] == l[k]);
      assert(l2[reg_w] == l[reg_w]);
      if io_api_deregistered_at(l, k) {
        assert(io_api_deregistered_at(l2, k));
        assert(get_io_api_deregister_rid(l2[k]) != get_io_api_register_rid(l2[reg_w]));
      }
    };
    assert(io_api_active_at(l, reg_w, sw_w));
    assert(io_api_active_at_set_waker(l, rid_l, sw_w));
    assert forall |k: int| sw_w < k < i implies !(
      io_api_deregistered_at(l, k) && get_io_api_deregister_rid(l[k]) == rid_l
    ) by {
      assert(k < n);
      assert(l2[k] == l[k]);
      assert(!(io_api_deregistered_at(l2, k) && get_io_api_deregister_rid(l2[k]) == rid_l));
    };
    assert(has_valid_set_waker_readable_api(l, i));
  }

  // Establish find_last properties on l
  find_last_set_waker_readable_properties(l, rid_l, i);
  let sw_l = find_last_set_waker_for_rid_readable(l, rid_l, i);
  let waker_l = get_set_waker_waker(l[sw_l]);

  // From old R17a on l
  let j0 = choose |j: int| #![trigger is_wake_task_at(l, j)] j > i && {
    let event = get_io_event(l[i]);
    let rid = event.resource_id;
    let sw_idx = find_last_set_waker_for_rid_readable(l, rid, i);
    let waker = get_set_waker_waker(l[sw_idx]);
    is_wake_task_at(l, j) &&
    get_wake_task_source_rid(l[j]) == rid &&
    get_wake_task_waker(l[j]) == waker &&
    j > i &&
    forall |k: int| i < k < j ==> !is_park_end_at(l, k) && !is_poll_events_at(l, k)
  };
  assert(is_wake_task_at(l, j0));
  assert(0 <= j0 && j0 < n);
  assert(l2[j0] == l[j0]);
  assert(is_wake_task_at(l2, j0));

  // Show sw_l satisfies the find_last predicate on l2
  assert(sw_l < n);
  assert(l2[sw_l] == l[sw_l]);
  assert(is_succ_set_waker_at(l2, sw_l));
  assert(get_set_waker_rid(l2[sw_l]) == rid_l);
  assert(get_set_waker_interest(l2[sw_l]).0);
  assert forall |k: int| sw_l < k < i implies !(
    is_succ_set_waker_at(l2, k) &&
    get_set_waker_rid(l2[k]) == rid_l &&
    get_set_waker_interest(l2[k]).0
  ) by {
    assert(k < n);
    assert(l2[k] == l[k]);
    if is_succ_set_waker_at(l2, k) {
      assert(is_succ_set_waker_at(l, k));
    }
  };

  // Establish find_last properties on l2
  find_last_set_waker_readable_properties(l2, rid_l, i);
  let sw_l2 = find_last_set_waker_for_rid_readable(l2, rid_l, i);
  // Both sw_l and sw_l2 are maximum set_waker on l2 → equal
  assert(sw_l == sw_l2);
  assert(sw_l2 < n);
  assert(l2[sw_l2] == l[sw_l2]);
  let waker_l2 = get_set_waker_waker(l2[sw_l2]);
  assert(waker_l2 == waker_l);

  assert(get_wake_task_source_rid(l2[j0]) == rid_l);
  assert(get_wake_task_waker(l2[j0]) == waker_l2);

  assert forall |k: int| i < k < j0 implies
    !is_park_end_at(l2, k) && !is_poll_events_at(l2, k)
  by {
    assert(k < n);
    assert(l2[k] == l[k]);
    assert(!is_park_end_at(l, k));
    assert(!is_poll_events_at(l, k));
  };
}

#[verifier::rlimit(30)]
proof fn flat_r17b_preserved(l: Log, e: ReactorEvent, i: int)
  requires
    reactor_inv(l),
    !is_wake_task_at(l.push(e), l.len() as int),
    is_io_event_ready_at(l.push(e), i),
    has_valid_set_waker_writable_api(l.push(e), i),
    i < l.len() as int,
  ensures ({
    let l2 = l.push(e);
    exists |j: int| #![trigger is_wake_task_at(l2, j)] j > i && {
      let event = get_io_event(l2[i]);
      let rid = event.resource_id;
      let sw_idx = find_last_set_waker_for_rid_writable(l2, rid, i);
      let waker = get_set_waker_waker(l2[sw_idx]);
      is_wake_task_at(l2, j) &&
      get_wake_task_source_rid(l2[j]) == rid &&
      get_wake_task_waker(l2[j]) == waker &&
      j > i &&
      forall |k: int| i < k < j ==> !is_park_end_at(l2, k) && !is_poll_events_at(l2, k)
    }
  })
{
  let l2 = l.push(e);
  let n = l.len() as int;
  assert(l2[i] == l[i]);
  assert(is_io_event_ready_at(l, i));

  let rid_l = get_io_event(l[i]).resource_id;

  // Transport has_valid_set_waker_writable_api from l2 to l
  {
    let sw_w = choose |sw_idx: int| 0 <= sw_idx < i &&
      is_succ_set_waker_at(l2, sw_idx) &&
      get_set_waker_rid(l2[sw_idx]) == rid_l &&
      get_set_waker_interest(l2[sw_idx]).1 &&
      io_api_active_at_set_waker(l2, rid_l, sw_idx) &&
      forall |k: int| sw_idx < k < i ==> !(
        io_api_deregistered_at(l2, k) && get_io_api_deregister_rid(l2[k]) == rid_l
      );
    assert(sw_w < n);
    assert(l2[sw_w] == l[sw_w]);
    assert(is_succ_set_waker_at(l, sw_w));
    assert(get_set_waker_rid(l[sw_w]) == rid_l);
    assert(get_set_waker_interest(l[sw_w]).1);
    let reg_w = choose |reg_idx: int| 0 <= reg_idx < sw_w &&
      io_api_registered_at(l2, reg_idx) &&
      get_io_api_register_rid(l2[reg_idx]) == rid_l &&
      io_api_active_at(l2, reg_idx, sw_w);
    assert(reg_w < n);
    assert(l2[reg_w] == l[reg_w]);
    assert(io_api_registered_at(l, reg_w));
    assert(get_io_api_register_rid(l[reg_w]) == rid_l);
    assert forall |k: int| reg_w < k < sw_w implies !(
      io_api_deregistered_at(l, k) && get_io_api_deregister_rid(l[k]) == get_io_api_register_rid(l[reg_w])
    ) by {
      assert(k < n);
      assert(l2[k] == l[k]);
      assert(l2[reg_w] == l[reg_w]);
      if io_api_deregistered_at(l, k) {
        assert(io_api_deregistered_at(l2, k));
        assert(get_io_api_deregister_rid(l2[k]) != get_io_api_register_rid(l2[reg_w]));
      }
    };
    assert(io_api_active_at(l, reg_w, sw_w));
    assert(io_api_active_at_set_waker(l, rid_l, sw_w));
    assert forall |k: int| sw_w < k < i implies !(
      io_api_deregistered_at(l, k) && get_io_api_deregister_rid(l[k]) == rid_l
    ) by {
      assert(k < n);
      assert(l2[k] == l[k]);
      assert(!(io_api_deregistered_at(l2, k) && get_io_api_deregister_rid(l2[k]) == rid_l));
    };
    assert(has_valid_set_waker_writable_api(l, i));
  }

  find_last_set_waker_writable_properties(l, rid_l, i);
  let sw_l = find_last_set_waker_for_rid_writable(l, rid_l, i);
  let waker_l = get_set_waker_waker(l[sw_l]);

  let j0 = choose |j: int| #![trigger is_wake_task_at(l, j)] j > i && {
    let event = get_io_event(l[i]);
    let rid = event.resource_id;
    let sw_idx = find_last_set_waker_for_rid_writable(l, rid, i);
    let waker = get_set_waker_waker(l[sw_idx]);
    is_wake_task_at(l, j) &&
    get_wake_task_source_rid(l[j]) == rid &&
    get_wake_task_waker(l[j]) == waker &&
    j > i &&
    forall |k: int| i < k < j ==> !is_park_end_at(l, k) && !is_poll_events_at(l, k)
  };
  assert(is_wake_task_at(l, j0));
  assert(0 <= j0 && j0 < n);
  assert(l2[j0] == l[j0]);
  assert(is_wake_task_at(l2, j0));

  // Show sw_l satisfies the find_last predicate on l2
  assert(sw_l < n);
  assert(l2[sw_l] == l[sw_l]);
  assert(is_succ_set_waker_at(l2, sw_l));
  assert(get_set_waker_rid(l2[sw_l]) == rid_l);
  assert(get_set_waker_interest(l2[sw_l]).1);
  assert forall |k: int| sw_l < k < i implies !(
    is_succ_set_waker_at(l2, k) &&
    get_set_waker_rid(l2[k]) == rid_l &&
    get_set_waker_interest(l2[k]).1
  ) by {
    assert(k < n);
    assert(l2[k] == l[k]);
    if is_succ_set_waker_at(l2, k) {
      assert(is_succ_set_waker_at(l, k));
    }
  };

  find_last_set_waker_writable_properties(l2, rid_l, i);
  let sw_l2 = find_last_set_waker_for_rid_writable(l2, rid_l, i);
  assert(sw_l == sw_l2);
  assert(sw_l2 < n);
  assert(l2[sw_l2] == l[sw_l2]);
  let waker_l2 = get_set_waker_waker(l2[sw_l2]);
  assert(waker_l2 == waker_l);

  assert(get_wake_task_source_rid(l2[j0]) == rid_l);
  assert(get_wake_task_waker(l2[j0]) == waker_l2);

  assert forall |k: int| i < k < j0 implies
    !is_park_end_at(l2, k) && !is_poll_events_at(l2, k)
  by {
    assert(k < n);
    assert(l2[k] == l[k]);
    assert(!is_park_end_at(l, k));
    assert(!is_poll_events_at(l, k));
  };
}

#[verifier::rlimit(40)]
pub proof fn reactor_inv_preserved_by_non_trigger(l: Log, e: ReactorEvent)
  requires
    reactor_inv(l),
    !is_succ_register_timer_at(l.push(e), l.len() as int),
    !io_api_registered_at(l.push(e), l.len() as int),
    !is_wake_task_at(l.push(e), l.len() as int),
    !is_io_event_ready_at(l.push(e), l.len() as int),
    !is_get_current_time_at(l.push(e), l.len() as int),
  ensures
    reactor_inv(l.push(e)),
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
    flat_r5_preserved(l, e, i);
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
    flat_r6_preserved(l, e, i);
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
    flat_r14_preserved(l, e, i);
  }

  // R16
  assert forall |i: int| #![auto]
    (is_succ_register_timer_at(l2, i) && has_timeout_point(l2, i) && timer_awaiting_wake(l2, i)) implies
    exists |j: int| j > i &&
      is_wake_task_at(l2, j) &&
      get_wake_task_source_rid(l2[j]) == get_register_timer_rid(l2[i]) &&
      get_wake_task_waker(l2[j]) == get_register_timer_waker(l2[i]) && {
      let timeout_idx = first_timeout_point(l2, i);
      j > timeout_idx &&
      forall |k: int| timeout_idx < k < j ==> !is_park_end_at(l2, k)
    }
  by {
    assert(i < n);
    flat_r16_preserved(l, e, i);
  }

  // R17a
  assert forall |i: int| #![auto]
    (is_io_event_ready_at(l2, i) && has_valid_set_waker_readable_api(l2, i)) implies
    exists |j: int| #![trigger is_wake_task_at(l2, j)] j > i && {
      let event = get_io_event(l2[i]);
      let rid = event.resource_id;
      let sw_idx = find_last_set_waker_for_rid_readable(l2, rid, i);
      let waker = get_set_waker_waker(l2[sw_idx]);
      is_wake_task_at(l2, j) &&
      get_wake_task_source_rid(l2[j]) == rid &&
      get_wake_task_waker(l2[j]) == waker &&
      j > i &&
      forall |k: int| i < k < j ==> !is_park_end_at(l2, k) && !is_poll_events_at(l2, k)
    }
  by {
    assert(i < n);
    flat_r17a_preserved(l, e, i);
  }

  // R17b
  assert forall |i: int| #![auto]
    (is_io_event_ready_at(l2, i) && has_valid_set_waker_writable_api(l2, i)) implies
    exists |j: int| #![trigger is_wake_task_at(l2, j)] j > i && {
      let event = get_io_event(l2[i]);
      let rid = event.resource_id;
      let sw_idx = find_last_set_waker_for_rid_writable(l2, rid, i);
      let waker = get_set_waker_waker(l2[sw_idx]);
      is_wake_task_at(l2, j) &&
      get_wake_task_source_rid(l2[j]) == rid &&
      get_wake_task_waker(l2[j]) == waker &&
      j > i &&
      forall |k: int| i < k < j ==> !is_park_end_at(l2, k) && !is_poll_events_at(l2, k)
    }
  by {
    assert(i < n);
    flat_r17b_preserved(l, e, i);
  }
}

#[verifier::rlimit(40)]
pub proof fn reactor_inv_preserved_by_succ_register_timer(l: Log, e: ReactorEvent)
  requires
    reactor_inv(l),
    is_succ_register_timer_at(l.push(e), l.len() as int),
    !is_wake_task_at(l.push(e), l.len() as int),
    !is_io_event_ready_at(l.push(e), l.len() as int),
    !is_get_current_time_at(l.push(e), l.len() as int),
    no_prior_timer_registration(l, get_register_timer_rid(l.push(e)[l.len() as int]), l.len() as int),
    no_io_api_with_rid_before(l, get_register_timer_rid(l.push(e)[l.len() as int]), l.len() as int),
  ensures
    reactor_inv(l.push(e)),
{
  let l2 = l.push(e);
  let n = l.len() as int;
  let new_rid = get_register_timer_rid(l2[n]);

  // R7: timer_reg_uniqueness — need special case at position n
  assert forall |i: int| #![auto] is_succ_register_timer_at(l2, i) implies {
    let rid = get_register_timer_rid(l2[i]);
    no_prior_timer_registration(l2, rid, i)
  } by {
    if i == n {
      no_prior_timer_reg_preserved(l, e, new_rid, n);
    } else {
      assert(i < n);
      assert(l2[i] == l[i]);
      assert(is_succ_register_timer_at(l, i));
      let rid = get_register_timer_rid(l[i]);
      assert(no_prior_timer_registration(l, rid, i));
      no_prior_timer_reg_preserved(l, e, rid, i);
    }
  }

  // R8: io_reg_uniqueness — non-trigger (no register_io at n)
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

  // R9a: timer_io_disjoint_at_timer — need special case at position n
  assert forall |i: int| #![auto] is_succ_register_timer_at(l2, i) implies {
    let rid = get_register_timer_rid(l2[i]);
    no_io_api_with_rid_before(l2, rid, i)
  } by {
    if i == n {
      no_io_api_with_rid_preserved(l, e, new_rid, n);
    } else {
      assert(i < n);
      assert(l2[i] == l[i]);
      assert(is_succ_register_timer_at(l, i));
      let rid = get_register_timer_rid(l[i]);
      assert(no_io_api_with_rid_before(l, rid, i));
      no_io_api_with_rid_preserved(l, e, rid, i);
    }
  }

  // R9b: timer_io_disjoint_at_io — non-trigger
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

  // R5: timer_waker_validity — non-trigger (no WakeTask at n)
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
    flat_r5_preserved(l, e, i);
  }

  // R6: io_waker_validity — non-trigger
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
    flat_r6_preserved(l, e, i);
  }

  // R14: wake_has_registration — non-trigger
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
    flat_r14_preserved(l, e, i);
  }

  // R16: wake_on_expired — register_timer at n has no timeout yet
  assert forall |i: int| #![auto]
    (is_succ_register_timer_at(l2, i) && has_timeout_point(l2, i) && timer_awaiting_wake(l2, i)) implies
    exists |j: int| j > i &&
      is_wake_task_at(l2, j) &&
      get_wake_task_source_rid(l2[j]) == get_register_timer_rid(l2[i]) &&
      get_wake_task_waker(l2[j]) == get_register_timer_waker(l2[i]) && {
      let timeout_idx = first_timeout_point(l2, i);
      j > timeout_idx &&
      forall |k: int| timeout_idx < k < j ==> !is_park_end_at(l2, k)
    }
  by {
    // Position n has no timeout point (no GetCurrentTime after n)
    // So trigger only fires for i < n
    if i == n {
      // has_timeout_point(l2, n) requires exists |tp| tp > n && is_get_current_time_at(l2, tp)
      // But l2.len() = n+1, so tp must be >= n+1 which is out of bounds
      // Actually tp < l2.len() = n+1 so tp <= n, contradicting tp > n
      assert(false); // vacuously true
    } else {
      assert(i < n);
      flat_r16_preserved(l, e, i);
    }
  }

  // R17a: wake_on_io_ready_readable — non-trigger (no IoEventReady at n)
  assert forall |i: int| #![auto]
    (is_io_event_ready_at(l2, i) && has_valid_set_waker_readable_api(l2, i)) implies
    exists |j: int| #![trigger is_wake_task_at(l2, j)] j > i && {
      let event = get_io_event(l2[i]);
      let rid = event.resource_id;
      let sw_idx = find_last_set_waker_for_rid_readable(l2, rid, i);
      let waker = get_set_waker_waker(l2[sw_idx]);
      is_wake_task_at(l2, j) &&
      get_wake_task_source_rid(l2[j]) == rid &&
      get_wake_task_waker(l2[j]) == waker &&
      j > i &&
      forall |k: int| i < k < j ==> !is_park_end_at(l2, k) && !is_poll_events_at(l2, k)
    }
  by {
    assert(i < n);
    flat_r17a_preserved(l, e, i);
  }

  // R17b: wake_on_io_ready_writable — non-trigger
  assert forall |i: int| #![auto]
    (is_io_event_ready_at(l2, i) && has_valid_set_waker_writable_api(l2, i)) implies
    exists |j: int| #![trigger is_wake_task_at(l2, j)] j > i && {
      let event = get_io_event(l2[i]);
      let rid = event.resource_id;
      let sw_idx = find_last_set_waker_for_rid_writable(l2, rid, i);
      let waker = get_set_waker_waker(l2[sw_idx]);
      is_wake_task_at(l2, j) &&
      get_wake_task_source_rid(l2[j]) == rid &&
      get_wake_task_waker(l2[j]) == waker &&
      j > i &&
      forall |k: int| i < k < j ==> !is_park_end_at(l2, k) && !is_poll_events_at(l2, k)
    }
  by {
    assert(i < n);
    flat_r17b_preserved(l, e, i);
  }
}

#[verifier::rlimit(40)]
pub proof fn reactor_inv_preserved_by_succ_register_io(l: Log, e: ReactorEvent)
  requires
    reactor_inv(l),
    io_api_registered_at(l.push(e), l.len() as int),
    !is_wake_task_at(l.push(e), l.len() as int),
    !is_io_event_ready_at(l.push(e), l.len() as int),
    !is_get_current_time_at(l.push(e), l.len() as int),
    no_prior_io_api_registration(l, get_io_api_register_rid(l.push(e)[l.len() as int]), l.len() as int),
    no_timer_with_rid_before(l, get_io_api_register_rid(l.push(e)[l.len() as int]), l.len() as int),
  ensures
    reactor_inv(l.push(e)),
{
  let l2 = l.push(e);
  let n = l.len() as int;
  let new_rid = get_io_api_register_rid(l2[n]);

  // R7: non-trigger (no register_timer at n)
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

  // R8: special case at position n
  assert forall |i: int| #![auto] io_api_registered_at(l2, i) implies {
    let rid = get_io_api_register_rid(l2[i]);
    no_prior_io_api_registration(l2, rid, i)
  } by {
    if i == n {
      no_prior_io_api_reg_preserved(l, e, new_rid, n);
    } else {
      assert(i < n);
      assert(l2[i] == l[i]);
      assert(io_api_registered_at(l, i));
      let rid = get_io_api_register_rid(l[i]);
      assert(no_prior_io_api_registration(l, rid, i));
      no_prior_io_api_reg_preserved(l, e, rid, i);
    }
  }

  // R9a: non-trigger
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

  // R9b: special case at position n
  assert forall |i: int| #![auto] io_api_registered_at(l2, i) implies {
    let rid = get_io_api_register_rid(l2[i]);
    no_timer_with_rid_before(l2, rid, i)
  } by {
    if i == n {
      no_timer_with_rid_preserved(l, e, new_rid, n);
    } else {
      assert(i < n);
      assert(l2[i] == l[i]);
      assert(io_api_registered_at(l, i));
      let rid = get_io_api_register_rid(l[i]);
      assert(no_timer_with_rid_before(l, rid, i));
      no_timer_with_rid_preserved(l, e, rid, i);
    }
  }

  // R5, R6, R14, R16, R17a, R17b — same as non-trigger case
  assert forall |i: int| #![auto] is_timer_wake_at(l2, i) implies {
    let rid = get_wake_task_source_rid(l2[i]);
    let waker = get_wake_task_waker(l2[i]);
    exists |j: int| 0 <= j < i &&
      is_succ_register_timer_at(l2, j) &&
      get_register_timer_rid(l2[j]) == rid &&
      get_register_timer_waker(l2[j]) == waker &&
      timer_active_at(l2, j, i)
  } by { assert(i < n); flat_r5_preserved(l, e, i); }

  assert forall |i: int| #![auto] is_io_api_wake_at(l2, i) implies {
    let rid = get_wake_task_source_rid(l2[i]);
    let waker = get_wake_task_waker(l2[i]);
    exists |sw_idx: int| 0 <= sw_idx < i &&
      is_succ_set_waker_at(l2, sw_idx) &&
      get_set_waker_rid(l2[sw_idx]) == rid &&
      get_set_waker_waker(l2[sw_idx]) == waker &&
      io_api_active_at_set_waker(l2, rid, sw_idx)
  } by { assert(i < n); flat_r6_preserved(l, e, i); }

  assert forall |i: int| #![auto] is_wake_task_at(l2, i) implies {
    let rid = get_wake_task_source_rid(l2[i]);
    (exists |j: int| 0 <= j < i &&
      is_succ_register_timer_at(l2, j) &&
      get_register_timer_rid(l2[j]) == rid)
    ||
    (exists |j: int| 0 <= j < i &&
      io_api_registered_at(l2, j) &&
      get_io_api_register_rid(l2[j]) == rid)
  } by { assert(i < n); flat_r14_preserved(l, e, i); }

  assert forall |i: int| #![auto]
    (is_succ_register_timer_at(l2, i) && has_timeout_point(l2, i) && timer_awaiting_wake(l2, i)) implies
    exists |j: int| #![trigger is_wake_task_at(l2, j)] j > i &&
      is_wake_task_at(l2, j) &&
      get_wake_task_source_rid(l2[j]) == get_register_timer_rid(l2[i]) &&
      get_wake_task_waker(l2[j]) == get_register_timer_waker(l2[i]) && {
      let timeout_idx = first_timeout_point(l2, i);
      j > timeout_idx &&
      forall |k: int| timeout_idx < k < j ==> !is_park_end_at(l2, k)
    }
  by { assert(i < n); flat_r16_preserved(l, e, i); }

  assert forall |i: int| #![auto]
    (is_io_event_ready_at(l2, i) && has_valid_set_waker_readable_api(l2, i)) implies
    exists |j: int| #![trigger is_wake_task_at(l2, j)] j > i && {
      let event = get_io_event(l2[i]);
      let rid = event.resource_id;
      let sw_idx = find_last_set_waker_for_rid_readable(l2, rid, i);
      let waker = get_set_waker_waker(l2[sw_idx]);
      is_wake_task_at(l2, j) &&
      get_wake_task_source_rid(l2[j]) == rid &&
      get_wake_task_waker(l2[j]) == waker &&
      j > i &&
      forall |k: int| i < k < j ==> !is_park_end_at(l2, k) && !is_poll_events_at(l2, k)
    }
  by { assert(i < n); flat_r17a_preserved(l, e, i); }

  assert forall |i: int| #![auto]
    (is_io_event_ready_at(l2, i) && has_valid_set_waker_writable_api(l2, i)) implies
    exists |j: int| #![trigger is_wake_task_at(l2, j)] j > i && {
      let event = get_io_event(l2[i]);
      let rid = event.resource_id;
      let sw_idx = find_last_set_waker_for_rid_writable(l2, rid, i);
      let waker = get_set_waker_waker(l2[sw_idx]);
      is_wake_task_at(l2, j) &&
      get_wake_task_source_rid(l2[j]) == rid &&
      get_wake_task_waker(l2[j]) == waker &&
      j > i &&
      forall |k: int| i < k < j ==> !is_park_end_at(l2, k) && !is_poll_events_at(l2, k)
    }
  by { assert(i < n); flat_r17b_preserved(l, e, i); }
}

}
