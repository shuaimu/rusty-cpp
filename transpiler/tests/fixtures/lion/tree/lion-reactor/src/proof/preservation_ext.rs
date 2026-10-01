use vstd::prelude::*;
use crate::spec::log::*;
use crate::spec::types::*;
use crate::spec::predicates::*;
use crate::invariants::*;
use crate::invariants::register_io_in_cycle::*;
use crate::invariants::deregister_io_in_cycle::*;
use crate::invariants::inbound_register_io_result::*;
use crate::invariants::inbound_deregister_io_result::*;
use crate::invariants::timer_deadline_future::*;
use crate::invariants::park_has_timestamp::*;
use crate::invariants::park_poll_once::*;
use crate::invariants::io_ready_in_park::*;
use crate::invariants::set_waker_active_io::*;

verus! {

pub proof fn max_timestamp_up_to_push(l: Log, e: ReactorEvent, i: int)
  requires i <= l.len() as int, i >= 0
  ensures max_timestamp_up_to(l.push(e), i) == max_timestamp_up_to(l, i)
  decreases i
{
  if i > 0 {
    max_timestamp_up_to_push(l, e, i - 1);
    assert(l.push(e)[i - 1] == l[i - 1]);
  }
}

pub proof fn current_park_start_push(l: Log, e: ReactorEvent, i: int)
  requires i <= l.len() as int, i >= 0
  ensures current_park_start(l.push(e), i) == current_park_start(l, i)
  decreases i
{
  if i > 0 {
    assert(l.push(e)[i - 1] == l[i - 1]);
    if !is_park_begin_at(l, i - 1) && !is_park_end_at(l, i - 1) {
      current_park_start_push(l, e, i - 1);
    }
  }
}

proof fn find_last_inbound_register_io_begin_push(l: Log, e: ReactorEvent, i: int)
  requires i <= l.len() as int, i >= 0
  ensures find_last_inbound_register_io_begin(l.push(e), i) == find_last_inbound_register_io_begin(l, i)
  decreases i
{
  if i > 0 {
    assert(l.push(e)[i - 1] == l[i - 1]);
    if !is_inbound_register_io_begin_at(l, i - 1) {
      find_last_inbound_register_io_begin_push(l, e, i - 1);
    }
  }
}

proof fn find_last_inbound_deregister_io_begin_push(l: Log, e: ReactorEvent, i: int)
  requires i <= l.len() as int, i >= 0
  ensures find_last_inbound_deregister_io_begin(l.push(e), i) == find_last_inbound_deregister_io_begin(l, i)
  decreases i
{
  if i > 0 {
    assert(l.push(e)[i - 1] == l[i - 1]);
    if !is_inbound_deregister_io_begin_at(l, i - 1) {
      find_last_inbound_deregister_io_begin_push(l, e, i - 1);
    }
  }
}

proof fn find_register_io_cycle_begin_push(l: Log, e: ReactorEvent, i: int)
  requires i <= l.len() as int, i >= 0
  ensures find_register_io_cycle_begin(l.push(e), i) == find_register_io_cycle_begin(l, i)
  decreases i
{
  if i > 0 {
    assert(l.push(e)[i - 1] == l[i - 1]);
    if !is_inbound_register_io_begin_at(l, i - 1) {
      find_register_io_cycle_begin_push(l, e, i - 1);
    }
  }
}

proof fn find_deregister_io_cycle_begin_push(l: Log, e: ReactorEvent, i: int)
  requires i <= l.len() as int, i >= 0
  ensures find_deregister_io_cycle_begin(l.push(e), i) == find_deregister_io_cycle_begin(l, i)
  decreases i
{
  if i > 0 {
    assert(l.push(e)[i - 1] == l[i - 1]);
    if !is_inbound_deregister_io_begin_at(l, i - 1) {
      find_deregister_io_cycle_begin_push(l, e, i - 1);
    }
  }
}

proof fn ext_r1_prefix(l: Log, e: ReactorEvent, i: int)
  requires
    reactor_ext_inv(l),
    0 <= i < l.len() as int,
    is_succ_register_timer_at(l.push(e), i),
  ensures
    timer_deadline_future_at(l.push(e), i),
{
  let l2 = l.push(e);
  assert(l2[i] == l[i]);
  assert(is_succ_register_timer_at(l, i));
  assert(timer_deadline_future_at(l, i));
  max_timestamp_up_to_push(l, e, i);
}

proof fn ext_r2_prefix(l: Log, e: ReactorEvent, i: int)
  requires
    reactor_ext_inv(l),
    0 <= i < l.len() as int,
    is_park_end_at(l.push(e), i),
  ensures
    has_get_current_time_in_park(l.push(e), i),
{
  let l2 = l.push(e);
  assert(l2[i] == l[i]);
  assert(is_park_end_at(l, i));
  assert(has_get_current_time_in_park(l, i));
  current_park_start_push(l, e, i);
  let ps = current_park_start(l, i);
  let j = choose |j: int| ps < j < i && is_get_current_time_at(l, j);
  assert(l2[j] == l[j]);
  assert(is_get_current_time_at(l2, j));
  assert(current_park_start(l2, i) < j < i);
}

pub proof fn count_poll_events_in_range_push(l: Log, e: ReactorEvent, start: int, end: int)
  requires start >= 0, end <= l.len() as int
  ensures count_poll_events_in_range(l.push(e), start, end) == count_poll_events_in_range(l, start, end)
  decreases end - start
{
  if start < end {
    assert(l.push(e)[start] == l[start]);
    count_poll_events_in_range_push(l, e, start + 1, end);
  }
}

proof fn ext_r3_prefix(l: Log, e: ReactorEvent, i: int)
  requires
    reactor_ext_inv(l),
    0 <= i < l.len() as int,
    is_park_end_at(l.push(e), i),
  ensures
    has_exactly_one_poll_events_in_park(l.push(e), i),
{
  let l2 = l.push(e);
  assert(l2[i] == l[i]);
  assert(is_park_end_at(l, i));
  assert(has_exactly_one_poll_events_in_park(l, i));
  current_park_start_push(l, e, i);
  let ps = current_park_start(l, i);
  assert(ps >= 0);
  count_poll_events_in_range_push(l, e, ps, i);
}

proof fn ext_r4_prefix(l: Log, e: ReactorEvent, i: int)
  requires
    reactor_ext_inv(l),
    0 <= i < l.len() as int,
    is_io_event_ready_at(l.push(e), i),
  ensures
    is_in_park_cycle(l.push(e), i),
{
  let l2 = l.push(e);
  assert(l2[i] == l[i]);
  assert(is_io_event_ready_at(l, i));
  current_park_start_push(l, e, i);
}

proof fn ext_r9_prefix(l: Log, e: ReactorEvent, i: int)
  requires
    reactor_ext_inv(l),
    0 <= i < l.len() as int,
    io_syscall_register_at(l.push(e), i),
  ensures
    in_register_io_cycle(l.push(e), i),
{
  let l2 = l.push(e);
  assert(l2[i] == l[i]);
  assert(io_syscall_register_at(l, i));
  assert(in_register_io_cycle(l, i));
  find_last_inbound_register_io_begin_push(l, e, i);
  let begin_idx = find_last_inbound_register_io_begin(l, i);
  assert(no_inbound_register_io_end_between(l, begin_idx, i));
  assert forall |k: int| begin_idx < k < i implies
    !#[trigger] is_inbound_register_io_end_at(l2, k)
  by {
    assert(!is_inbound_register_io_end_at(l, k));
    assert(l2[k] == l[k]);
  }
}

proof fn ext_r10_prefix(l: Log, e: ReactorEvent, i: int)
  requires
    reactor_ext_inv(l),
    0 <= i < l.len() as int,
    io_syscall_deregistered_at(l.push(e), i),
  ensures
    in_deregister_io_cycle(l.push(e), i),
{
  let l2 = l.push(e);
  assert(l2[i] == l[i]);
  assert(io_syscall_deregistered_at(l, i));
  assert(in_deregister_io_cycle(l, i));
  find_last_inbound_deregister_io_begin_push(l, e, i);
  let begin_idx = find_last_inbound_deregister_io_begin(l, i);
  assert(no_inbound_deregister_io_end_between(l, begin_idx, i));
  assert forall |k: int| begin_idx < k < i implies
    !#[trigger] is_inbound_deregister_io_end_at(l2, k)
  by {
    assert(!is_inbound_deregister_io_end_at(l, k));
    assert(l2[k] == l[k]);
  }
}

proof fn ext_r11_prefix(l: Log, e: ReactorEvent, i: int)
  requires
    reactor_ext_inv(l),
    0 <= i < l.len() as int,
    is_inbound_deregister_io_end_at(l.push(e), i),
  ensures
    deregister_io_result_valid(l.push(e), i),
{
  let l2 = l.push(e);
  assert(l2[i] == l[i]);
  assert(is_inbound_deregister_io_end_at(l, i));
  assert(deregister_io_result_valid(l, i));
  find_deregister_io_cycle_begin_push(l, e, i);
  let begin_idx = find_deregister_io_cycle_begin(l, i);
  let rid = get_io_api_deregister_rid(l[i]);
  let result = get_inbound_deregister_io_result(l[i]);
  assert(has_matching_outbound_deregister(l, begin_idx, i, rid, result));
  let k = choose |k: int| begin_idx < k < i &&
    io_syscall_deregistered_at(l, k) &&
    get_io_syscall_deregister_rid(l[k]) == rid &&
    get_outbound_deregister_io_result(l[k]) == result;
  assert(l2[k] == l[k]);
  assert(l2[begin_idx] == l[begin_idx]);
}

proof fn ext_r12_prefix(l: Log, e: ReactorEvent, i: int)
  requires
    reactor_ext_inv(l),
    0 <= i < l.len() as int,
    io_api_registered_at(l.push(e), i),
  ensures
    register_io_result_valid(l.push(e), i),
{
  let l2 = l.push(e);
  assert(l2[i] == l[i]);
  assert(io_api_registered_at(l, i));
  assert(register_io_result_valid(l, i));
  find_register_io_cycle_begin_push(l, e, i);
  let begin_idx = find_register_io_cycle_begin(l, i);
  let source = get_inbound_register_io_source(l[i]);
  let interest = get_inbound_register_io_interest(l[i]);
  let result = get_inbound_register_io_result(l[i]);
  assert(has_matching_outbound_register(l, begin_idx, i, source, interest, result));
  let k = choose |k: int| begin_idx < k < i &&
    io_syscall_register_at(l, k) &&
    get_outbound_register_io_source(l[k]) == source &&
    get_outbound_register_io_interest(l[k]) == interest &&
    inbound_result_matches_outbound_register(result, get_outbound_register_io_result(l[k]), get_io_syscall_register_rid(l[k]));
  assert(l2[k] == l[k]);
  assert(l2[begin_idx] == l[begin_idx]);
}

proof fn ext_r16b_prefix(l: Log, e: ReactorEvent, i: int)
  requires
    reactor_ext_inv(l),
    0 <= i < l.len() as int,
    is_succ_set_waker_at(l.push(e), i),
  ensures
    set_waker_on_active_io(l.push(e), i),
{
  let l2 = l.push(e);
  assert(l2[i] == l[i]);
  assert(is_succ_set_waker_at(l, i));
  assert(set_waker_on_active_io(l, i));
  let rid = get_set_waker_rid(l[i]);
  assert(io_api_active_at_set_waker(l, rid, i));
  let reg_idx = choose |reg_idx: int| 0 <= reg_idx < i &&
    io_api_registered_at(l, reg_idx) &&
    get_io_api_register_rid(l[reg_idx]) == rid &&
    io_api_active_at(l, reg_idx, i);
  assert(l2[reg_idx] == l[reg_idx]);
  assert(io_api_registered_at(l2, reg_idx));
  assert(get_io_api_register_rid(l2[reg_idx]) == rid);
  assert forall |k: int| reg_idx < k < i implies !(
    io_api_deregistered_at(l2, k) &&
    get_io_api_deregister_rid(l2[k]) == get_io_api_register_rid(l2[reg_idx])
  ) by {
    assert(!(io_api_deregistered_at(l, k) && get_io_api_deregister_rid(l[k]) == get_io_api_register_rid(l[reg_idx])));
    assert(l2[k] == l[k]);
  }
  assert(io_api_active_at(l2, reg_idx, i));
  assert(io_api_active_at_set_waker(l2, rid, i));
}

pub proof fn reactor_ext_inv_preserved_by_non_trigger(l: Log, e: ReactorEvent)
  requires
    reactor_ext_inv(l),
    !is_succ_register_timer_at(l.push(e), l.len() as int),
    !is_park_end_at(l.push(e), l.len() as int),
    !is_io_event_ready_at(l.push(e), l.len() as int),
    !io_syscall_register_at(l.push(e), l.len() as int),
    !io_syscall_deregistered_at(l.push(e), l.len() as int),
    !io_api_registered_at(l.push(e), l.len() as int),
    !is_inbound_deregister_io_end_at(l.push(e), l.len() as int),
    !is_succ_set_waker_at(l.push(e), l.len() as int),
  ensures
    reactor_ext_inv(l.push(e)),
{
  let l2 = l.push(e);
  let n = l.len() as int;

  assert forall |i: int| #![auto] is_succ_register_timer_at(l2, i) implies
    timer_deadline_future_at(l2, i) by { assert(i < n); ext_r1_prefix(l, e, i); }

  assert forall |i: int| #![auto] is_park_end_at(l2, i) implies
    has_get_current_time_in_park(l2, i) by { assert(i < n); ext_r2_prefix(l, e, i); }

  assert forall |i: int| #![auto] is_park_end_at(l2, i) implies
    has_exactly_one_poll_events_in_park(l2, i) by { assert(i < n); ext_r3_prefix(l, e, i); }

  assert forall |i: int| #![auto] is_io_event_ready_at(l2, i) implies
    is_in_park_cycle(l2, i) by { assert(i < n); ext_r4_prefix(l, e, i); }

  assert forall |i: int| #![auto] io_syscall_register_at(l2, i) implies
    in_register_io_cycle(l2, i) by { assert(i < n); ext_r9_prefix(l, e, i); }

  assert forall |i: int| #![auto] io_syscall_deregistered_at(l2, i) implies
    in_deregister_io_cycle(l2, i) by { assert(i < n); ext_r10_prefix(l, e, i); }

  assert forall |i: int| #![auto] is_inbound_deregister_io_end_at(l2, i) implies
    deregister_io_result_valid(l2, i) by { assert(i < n); ext_r11_prefix(l, e, i); }

  assert forall |i: int| #![auto] io_api_registered_at(l2, i) implies
    register_io_result_valid(l2, i) by { assert(i < n); ext_r12_prefix(l, e, i); }

  assert forall |i: int| #![auto] is_succ_set_waker_at(l2, i) implies
    set_waker_on_active_io(l2, i) by { assert(i < n); ext_r16b_prefix(l, e, i); }
}

pub proof fn reactor_ext_inv_preserved_by_succ_register_timer(l: Log, e: ReactorEvent)
  requires
    reactor_ext_inv(l),
    is_succ_register_timer_at(l.push(e), l.len() as int),
    !is_park_end_at(l.push(e), l.len() as int),
    !is_io_event_ready_at(l.push(e), l.len() as int),
    !io_syscall_register_at(l.push(e), l.len() as int),
    !io_syscall_deregistered_at(l.push(e), l.len() as int),
    !is_inbound_deregister_io_end_at(l.push(e), l.len() as int),
    !is_succ_set_waker_at(l.push(e), l.len() as int),
    get_register_timer_deadline(l.push(e)[l.len() as int]) > max_timestamp_up_to(l, l.len() as int),
  ensures
    reactor_ext_inv(l.push(e)),
{
  let l2 = l.push(e);
  let n = l.len() as int;

  assert forall |i: int| #![auto] is_succ_register_timer_at(l2, i) implies
    timer_deadline_future_at(l2, i)
  by {
    if i < n { ext_r1_prefix(l, e, i); }
    else { max_timestamp_up_to_push(l, e, n); }
  }

  assert forall |i: int| #![auto] is_park_end_at(l2, i) implies
    has_get_current_time_in_park(l2, i) by { assert(i < n); ext_r2_prefix(l, e, i); }

  assert forall |i: int| #![auto] is_park_end_at(l2, i) implies
    has_exactly_one_poll_events_in_park(l2, i) by { assert(i < n); ext_r3_prefix(l, e, i); }

  assert forall |i: int| #![auto] is_io_event_ready_at(l2, i) implies
    is_in_park_cycle(l2, i) by { assert(i < n); ext_r4_prefix(l, e, i); }

  assert forall |i: int| #![auto] io_syscall_register_at(l2, i) implies
    in_register_io_cycle(l2, i) by { assert(i < n); ext_r9_prefix(l, e, i); }

  assert forall |i: int| #![auto] io_syscall_deregistered_at(l2, i) implies
    in_deregister_io_cycle(l2, i) by { assert(i < n); ext_r10_prefix(l, e, i); }

  assert forall |i: int| #![auto] is_inbound_deregister_io_end_at(l2, i) implies
    deregister_io_result_valid(l2, i) by { assert(i < n); ext_r11_prefix(l, e, i); }

  assert forall |i: int| #![auto] io_api_registered_at(l2, i) implies
    register_io_result_valid(l2, i) by { assert(i < n); ext_r12_prefix(l, e, i); }

  assert forall |i: int| #![auto] is_succ_set_waker_at(l2, i) implies
    set_waker_on_active_io(l2, i) by { assert(i < n); ext_r16b_prefix(l, e, i); }
}

pub proof fn reactor_ext_inv_preserved_by_succ_register_io(l: Log, e: ReactorEvent)
  requires
    reactor_ext_inv(l),
    io_api_registered_at(l.push(e), l.len() as int),
    !is_park_end_at(l.push(e), l.len() as int),
    !is_io_event_ready_at(l.push(e), l.len() as int),
    !is_succ_set_waker_at(l.push(e), l.len() as int),
    register_io_result_valid(l.push(e), l.len() as int),
  ensures
    reactor_ext_inv(l.push(e)),
{
  let l2 = l.push(e);
  let n = l.len() as int;

  assert forall |i: int| #![auto] is_succ_register_timer_at(l2, i) implies
    timer_deadline_future_at(l2, i) by { assert(i < n); ext_r1_prefix(l, e, i); }

  assert forall |i: int| #![auto] is_park_end_at(l2, i) implies
    has_get_current_time_in_park(l2, i) by { assert(i < n); ext_r2_prefix(l, e, i); }

  assert forall |i: int| #![auto] is_park_end_at(l2, i) implies
    has_exactly_one_poll_events_in_park(l2, i) by { assert(i < n); ext_r3_prefix(l, e, i); }

  assert forall |i: int| #![auto] is_io_event_ready_at(l2, i) implies
    is_in_park_cycle(l2, i) by { assert(i < n); ext_r4_prefix(l, e, i); }

  assert forall |i: int| #![auto] io_syscall_register_at(l2, i) implies
    in_register_io_cycle(l2, i) by { assert(i < n); ext_r9_prefix(l, e, i); }

  assert forall |i: int| #![auto] io_syscall_deregistered_at(l2, i) implies
    in_deregister_io_cycle(l2, i) by { assert(i < n); ext_r10_prefix(l, e, i); }

  assert forall |i: int| #![auto] is_inbound_deregister_io_end_at(l2, i) implies
    deregister_io_result_valid(l2, i) by { assert(i < n); ext_r11_prefix(l, e, i); }

  assert forall |i: int| #![auto] io_api_registered_at(l2, i) implies
    register_io_result_valid(l2, i)
  by {
    if i < n { ext_r12_prefix(l, e, i); }
  }

  assert forall |i: int| #![auto] is_succ_set_waker_at(l2, i) implies
    set_waker_on_active_io(l2, i) by { assert(i < n); ext_r16b_prefix(l, e, i); }
}

pub proof fn reactor_ext_inv_preserved_by_outbound_register_io(l: Log, e: ReactorEvent)
  requires
    reactor_ext_inv(l),
    io_syscall_register_at(l.push(e), l.len() as int),
    !is_succ_register_timer_at(l.push(e), l.len() as int),
    !is_park_end_at(l.push(e), l.len() as int),
    !is_io_event_ready_at(l.push(e), l.len() as int),
    !io_api_registered_at(l.push(e), l.len() as int),
    !is_inbound_deregister_io_end_at(l.push(e), l.len() as int),
    !is_succ_set_waker_at(l.push(e), l.len() as int),
    in_register_io_cycle(l.push(e), l.len() as int),
  ensures
    reactor_ext_inv(l.push(e)),
{
  let l2 = l.push(e);
  let n = l.len() as int;

  assert forall |i: int| #![auto] is_succ_register_timer_at(l2, i) implies
    timer_deadline_future_at(l2, i) by { assert(i < n); ext_r1_prefix(l, e, i); }

  assert forall |i: int| #![auto] is_park_end_at(l2, i) implies
    has_get_current_time_in_park(l2, i) by { assert(i < n); ext_r2_prefix(l, e, i); }

  assert forall |i: int| #![auto] is_park_end_at(l2, i) implies
    has_exactly_one_poll_events_in_park(l2, i) by { assert(i < n); ext_r3_prefix(l, e, i); }

  assert forall |i: int| #![auto] is_io_event_ready_at(l2, i) implies
    is_in_park_cycle(l2, i) by { assert(i < n); ext_r4_prefix(l, e, i); }

  assert forall |i: int| #![auto] io_syscall_register_at(l2, i) implies
    in_register_io_cycle(l2, i)
  by {
    if i < n { ext_r9_prefix(l, e, i); }
  }

  assert forall |i: int| #![auto] io_syscall_deregistered_at(l2, i) implies
    in_deregister_io_cycle(l2, i) by { assert(i < n); ext_r10_prefix(l, e, i); }

  assert forall |i: int| #![auto] is_inbound_deregister_io_end_at(l2, i) implies
    deregister_io_result_valid(l2, i) by { assert(i < n); ext_r11_prefix(l, e, i); }

  assert forall |i: int| #![auto] io_api_registered_at(l2, i) implies
    register_io_result_valid(l2, i) by { assert(i < n); ext_r12_prefix(l, e, i); }

  assert forall |i: int| #![auto] is_succ_set_waker_at(l2, i) implies
    set_waker_on_active_io(l2, i) by { assert(i < n); ext_r16b_prefix(l, e, i); }
}

pub proof fn reactor_ext_inv_preserved_by_outbound_deregister_io(l: Log, e: ReactorEvent)
  requires
    reactor_ext_inv(l),
    io_syscall_deregistered_at(l.push(e), l.len() as int),
    !is_succ_register_timer_at(l.push(e), l.len() as int),
    !is_park_end_at(l.push(e), l.len() as int),
    !is_io_event_ready_at(l.push(e), l.len() as int),
    !io_api_registered_at(l.push(e), l.len() as int),
    !is_inbound_deregister_io_end_at(l.push(e), l.len() as int),
    !is_succ_set_waker_at(l.push(e), l.len() as int),
    in_deregister_io_cycle(l.push(e), l.len() as int),
  ensures
    reactor_ext_inv(l.push(e)),
{
  let l2 = l.push(e);
  let n = l.len() as int;

  assert forall |i: int| #![auto] is_succ_register_timer_at(l2, i) implies
    timer_deadline_future_at(l2, i) by { assert(i < n); ext_r1_prefix(l, e, i); }

  assert forall |i: int| #![auto] is_park_end_at(l2, i) implies
    has_get_current_time_in_park(l2, i) by { assert(i < n); ext_r2_prefix(l, e, i); }

  assert forall |i: int| #![auto] is_park_end_at(l2, i) implies
    has_exactly_one_poll_events_in_park(l2, i) by { assert(i < n); ext_r3_prefix(l, e, i); }

  assert forall |i: int| #![auto] is_io_event_ready_at(l2, i) implies
    is_in_park_cycle(l2, i) by { assert(i < n); ext_r4_prefix(l, e, i); }

  assert forall |i: int| #![auto] io_syscall_register_at(l2, i) implies
    in_register_io_cycle(l2, i) by { assert(i < n); ext_r9_prefix(l, e, i); }

  assert forall |i: int| #![auto] io_syscall_deregistered_at(l2, i) implies
    in_deregister_io_cycle(l2, i)
  by {
    if i < n { ext_r10_prefix(l, e, i); }
  }

  assert forall |i: int| #![auto] is_inbound_deregister_io_end_at(l2, i) implies
    deregister_io_result_valid(l2, i) by { assert(i < n); ext_r11_prefix(l, e, i); }

  assert forall |i: int| #![auto] io_api_registered_at(l2, i) implies
    register_io_result_valid(l2, i) by { assert(i < n); ext_r12_prefix(l, e, i); }

  assert forall |i: int| #![auto] is_succ_set_waker_at(l2, i) implies
    set_waker_on_active_io(l2, i) by { assert(i < n); ext_r16b_prefix(l, e, i); }
}

pub proof fn reactor_ext_inv_preserved_by_inbound_deregister_io_end(l: Log, e: ReactorEvent)
  requires
    reactor_ext_inv(l),
    is_inbound_deregister_io_end_at(l.push(e), l.len() as int),
    !is_succ_register_timer_at(l.push(e), l.len() as int),
    !is_park_end_at(l.push(e), l.len() as int),
    !is_io_event_ready_at(l.push(e), l.len() as int),
    !is_succ_set_waker_at(l.push(e), l.len() as int),
    deregister_io_result_valid(l.push(e), l.len() as int),
  ensures
    reactor_ext_inv(l.push(e)),
{
  let l2 = l.push(e);
  let n = l.len() as int;

  assert forall |i: int| #![auto] is_succ_register_timer_at(l2, i) implies
    timer_deadline_future_at(l2, i) by { assert(i < n); ext_r1_prefix(l, e, i); }

  assert forall |i: int| #![auto] is_park_end_at(l2, i) implies
    has_get_current_time_in_park(l2, i) by { assert(i < n); ext_r2_prefix(l, e, i); }

  assert forall |i: int| #![auto] is_park_end_at(l2, i) implies
    has_exactly_one_poll_events_in_park(l2, i) by { assert(i < n); ext_r3_prefix(l, e, i); }

  assert forall |i: int| #![auto] is_io_event_ready_at(l2, i) implies
    is_in_park_cycle(l2, i) by { assert(i < n); ext_r4_prefix(l, e, i); }

  assert forall |i: int| #![auto] io_syscall_register_at(l2, i) implies
    in_register_io_cycle(l2, i) by { assert(i < n); ext_r9_prefix(l, e, i); }

  assert forall |i: int| #![auto] io_syscall_deregistered_at(l2, i) implies
    in_deregister_io_cycle(l2, i) by { assert(i < n); ext_r10_prefix(l, e, i); }

  assert forall |i: int| #![auto] is_inbound_deregister_io_end_at(l2, i) implies
    deregister_io_result_valid(l2, i)
  by {
    if i < n { ext_r11_prefix(l, e, i); }
  }

  assert forall |i: int| #![auto] io_api_registered_at(l2, i) implies
    register_io_result_valid(l2, i) by { assert(i < n); ext_r12_prefix(l, e, i); }

  assert forall |i: int| #![auto] is_succ_set_waker_at(l2, i) implies
    set_waker_on_active_io(l2, i) by { assert(i < n); ext_r16b_prefix(l, e, i); }
}

pub proof fn reactor_ext_inv_preserved_by_succ_set_waker(l: Log, e: ReactorEvent)
  requires
    reactor_ext_inv(l),
    is_succ_set_waker_at(l.push(e), l.len() as int),
    !is_succ_register_timer_at(l.push(e), l.len() as int),
    !is_park_end_at(l.push(e), l.len() as int),
    !is_io_event_ready_at(l.push(e), l.len() as int),
    !io_syscall_register_at(l.push(e), l.len() as int),
    !io_syscall_deregistered_at(l.push(e), l.len() as int),
    !io_api_registered_at(l.push(e), l.len() as int),
    !is_inbound_deregister_io_end_at(l.push(e), l.len() as int),
    set_waker_on_active_io(l.push(e), l.len() as int),
  ensures
    reactor_ext_inv(l.push(e)),
{
  let l2 = l.push(e);
  let n = l.len() as int;

  assert forall |i: int| #![auto] is_succ_register_timer_at(l2, i) implies
    timer_deadline_future_at(l2, i) by { assert(i < n); ext_r1_prefix(l, e, i); }

  assert forall |i: int| #![auto] is_park_end_at(l2, i) implies
    has_get_current_time_in_park(l2, i) by { assert(i < n); ext_r2_prefix(l, e, i); }

  assert forall |i: int| #![auto] is_park_end_at(l2, i) implies
    has_exactly_one_poll_events_in_park(l2, i) by { assert(i < n); ext_r3_prefix(l, e, i); }

  assert forall |i: int| #![auto] is_io_event_ready_at(l2, i) implies
    is_in_park_cycle(l2, i) by { assert(i < n); ext_r4_prefix(l, e, i); }

  assert forall |i: int| #![auto] io_syscall_register_at(l2, i) implies
    in_register_io_cycle(l2, i) by { assert(i < n); ext_r9_prefix(l, e, i); }

  assert forall |i: int| #![auto] io_syscall_deregistered_at(l2, i) implies
    in_deregister_io_cycle(l2, i) by { assert(i < n); ext_r10_prefix(l, e, i); }

  assert forall |i: int| #![auto] is_inbound_deregister_io_end_at(l2, i) implies
    deregister_io_result_valid(l2, i) by { assert(i < n); ext_r11_prefix(l, e, i); }

  assert forall |i: int| #![auto] io_api_registered_at(l2, i) implies
    register_io_result_valid(l2, i) by { assert(i < n); ext_r12_prefix(l, e, i); }

  assert forall |i: int| #![auto] is_succ_set_waker_at(l2, i) implies
    set_waker_on_active_io(l2, i)
  by {
    if i < n { ext_r16b_prefix(l, e, i); }
  }
}

pub proof fn reactor_ext_inv_preserved_by_io_event_ready(l: Log, e: ReactorEvent)
  requires
    reactor_ext_inv(l),
    is_io_event_ready_at(l.push(e), l.len() as int),
    !is_succ_register_timer_at(l.push(e), l.len() as int),
    !is_park_end_at(l.push(e), l.len() as int),
    !io_syscall_register_at(l.push(e), l.len() as int),
    !io_syscall_deregistered_at(l.push(e), l.len() as int),
    !io_api_registered_at(l.push(e), l.len() as int),
    !is_inbound_deregister_io_end_at(l.push(e), l.len() as int),
    !is_succ_set_waker_at(l.push(e), l.len() as int),
    is_in_park_cycle(l.push(e), l.len() as int),
  ensures
    reactor_ext_inv(l.push(e)),
{
  let l2 = l.push(e);
  let n = l.len() as int;

  assert forall |i: int| #![auto] is_succ_register_timer_at(l2, i) implies
    timer_deadline_future_at(l2, i) by { assert(i < n); ext_r1_prefix(l, e, i); }

  assert forall |i: int| #![auto] is_park_end_at(l2, i) implies
    has_get_current_time_in_park(l2, i) by { assert(i < n); ext_r2_prefix(l, e, i); }

  assert forall |i: int| #![auto] is_park_end_at(l2, i) implies
    has_exactly_one_poll_events_in_park(l2, i) by { assert(i < n); ext_r3_prefix(l, e, i); }

  assert forall |i: int| #![auto] is_io_event_ready_at(l2, i) implies
    is_in_park_cycle(l2, i)
  by {
    if i < n { ext_r4_prefix(l, e, i); }
  }

  assert forall |i: int| #![auto] io_syscall_register_at(l2, i) implies
    in_register_io_cycle(l2, i) by { assert(i < n); ext_r9_prefix(l, e, i); }

  assert forall |i: int| #![auto] io_syscall_deregistered_at(l2, i) implies
    in_deregister_io_cycle(l2, i) by { assert(i < n); ext_r10_prefix(l, e, i); }

  assert forall |i: int| #![auto] is_inbound_deregister_io_end_at(l2, i) implies
    deregister_io_result_valid(l2, i) by { assert(i < n); ext_r11_prefix(l, e, i); }

  assert forall |i: int| #![auto] io_api_registered_at(l2, i) implies
    register_io_result_valid(l2, i) by { assert(i < n); ext_r12_prefix(l, e, i); }

  assert forall |i: int| #![auto] is_succ_set_waker_at(l2, i) implies
    set_waker_on_active_io(l2, i) by { assert(i < n); ext_r16b_prefix(l, e, i); }
}

pub proof fn reactor_ext_inv_preserved_by_park_end(l: Log, e: ReactorEvent)
  requires
    reactor_ext_inv(l),
    is_park_end_at(l.push(e), l.len() as int),
    !is_succ_register_timer_at(l.push(e), l.len() as int),
    !is_io_event_ready_at(l.push(e), l.len() as int),
    !io_syscall_register_at(l.push(e), l.len() as int),
    !io_syscall_deregistered_at(l.push(e), l.len() as int),
    !io_api_registered_at(l.push(e), l.len() as int),
    !is_inbound_deregister_io_end_at(l.push(e), l.len() as int),
    !is_succ_set_waker_at(l.push(e), l.len() as int),
    has_get_current_time_in_park(l.push(e), l.len() as int),
    has_exactly_one_poll_events_in_park(l.push(e), l.len() as int),
  ensures
    reactor_ext_inv(l.push(e)),
{
  let l2 = l.push(e);
  let n = l.len() as int;

  assert forall |i: int| #![auto] is_succ_register_timer_at(l2, i) implies
    timer_deadline_future_at(l2, i) by { assert(i < n); ext_r1_prefix(l, e, i); }

  assert forall |i: int| #![auto] is_park_end_at(l2, i) implies
    has_get_current_time_in_park(l2, i)
  by {
    if i < n { ext_r2_prefix(l, e, i); }
  }

  assert forall |i: int| #![auto] is_park_end_at(l2, i) implies
    has_exactly_one_poll_events_in_park(l2, i)
  by {
    if i < n { ext_r3_prefix(l, e, i); }
  }

  assert forall |i: int| #![auto] is_io_event_ready_at(l2, i) implies
    is_in_park_cycle(l2, i) by { assert(i < n); ext_r4_prefix(l, e, i); }

  assert forall |i: int| #![auto] io_syscall_register_at(l2, i) implies
    in_register_io_cycle(l2, i) by { assert(i < n); ext_r9_prefix(l, e, i); }

  assert forall |i: int| #![auto] io_syscall_deregistered_at(l2, i) implies
    in_deregister_io_cycle(l2, i) by { assert(i < n); ext_r10_prefix(l, e, i); }

  assert forall |i: int| #![auto] is_inbound_deregister_io_end_at(l2, i) implies
    deregister_io_result_valid(l2, i) by { assert(i < n); ext_r11_prefix(l, e, i); }

  assert forall |i: int| #![auto] io_api_registered_at(l2, i) implies
    register_io_result_valid(l2, i) by { assert(i < n); ext_r12_prefix(l, e, i); }

  assert forall |i: int| #![auto] is_succ_set_waker_at(l2, i) implies
    set_waker_on_active_io(l2, i) by { assert(i < n); ext_r16b_prefix(l, e, i); }
}

pub proof fn count_poll_events_no_poll_range(l: Log, start: int, end: int)
  requires
    start >= 0,
    end <= l.len(),
    forall |k: int| start <= k < end ==> !#[trigger] is_poll_events_at(l, k),
  ensures
    count_poll_events_in_range(l, start, end) == 0,
  decreases end - start,
{
  if start < end {
    count_poll_events_no_poll_range(l, start + 1, end);
  }
}

pub proof fn count_poll_events_split(l: Log, start: int, mid: int, end: int)
  requires start <= mid, mid <= end
  ensures
    count_poll_events_in_range(l, start, end) ==
    count_poll_events_in_range(l, start, mid) + count_poll_events_in_range(l, mid, end),
  decreases mid - start,
{
  if start < mid {
    count_poll_events_split(l, start + 1, mid, end);
  }
}

pub proof fn park_start_preserved_by_non_park(l: Log, e: ReactorEvent)
  requires
    !is_park_begin_at(l.push(e), l.len() as int),
    !is_park_end_at(l.push(e), l.len() as int),
    current_park_start(l, l.len() as int) >= 0,
  ensures
    current_park_start(l.push(e), (l.len() + 1) as int) == current_park_start(l, l.len() as int),
{
  current_park_start_push(l, e, l.len() as int);
}

}
