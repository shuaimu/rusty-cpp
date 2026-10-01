use vstd::prelude::*;
use crate::events::*;
#[allow(unused_imports)]
use crate::types::*;

verus! {

pub type Log = Seq<ReactorEvent>;

// ============================================================================
// Basic Log Predicates
// ============================================================================

pub open spec fn is_park_begin_at(l: Log, i: int) -> bool {
  0 <= i < l.len() && is_park_begin(l[i])
}

pub open spec fn is_park_end_at(l: Log, i: int) -> bool {
  0 <= i < l.len() && is_park_end(l[i])
}

pub open spec fn is_succ_register_timer_at(l: Log, i: int) -> bool {
  0 <= i < l.len() && is_succ_register_timer(l[i])
}

pub open spec fn is_deregister_timer_at(l: Log, i: int) -> bool {
  0 <= i < l.len() && is_deregister_timer(l[i])
}

pub open spec fn is_succ_deregister_timer_at(l: Log, i: int) -> bool {
  0 <= i < l.len() && is_succ_deregister_timer(l[i])
}

// SYSCALL anchor (Outbound): lion-liveness's io registration convention.
pub open spec fn io_syscall_register_at(l: Log, i: int) -> bool {
  0 <= i < l.len() && is_io_syscall_register(l[i])
}

pub open spec fn io_syscall_registered_at(l: Log, i: int) -> bool {
  0 <= i < l.len() && is_succ_io_syscall_register(l[i])
}

// Any Outbound DeregisterIoResource (any result).
pub open spec fn io_syscall_deregistered_at(l: Log, i: int) -> bool {
  0 <= i < l.len() && is_io_syscall_deregister(l[i])
}

// API anchor (Inbound): lion-reactor's io registration convention.
pub open spec fn io_api_registered_at(l: Log, i: int) -> bool {
  0 <= i < l.len() && is_succ_io_api_register(l[i])
}

// Any Inbound DeregisterIoResource (Begin or End, any result).
pub open spec fn io_api_deregistered_at(l: Log, i: int) -> bool {
  0 <= i < l.len() && is_io_api_deregister(l[i])
}

pub open spec fn is_set_waker_at(l: Log, i: int) -> bool {
  0 <= i < l.len() && is_set_waker(l[i])
}

pub open spec fn is_succ_set_waker_at(l: Log, i: int) -> bool {
  0 <= i < l.len() && is_succ_set_waker(l[i])
}

pub open spec fn is_get_current_time_at(l: Log, i: int) -> bool {
  0 <= i < l.len() && is_get_current_time(l[i])
}

pub open spec fn is_wake_task_at(l: Log, i: int) -> bool {
  0 <= i < l.len() && is_wake_task(l[i])
}

pub open spec fn is_io_event_ready_at(l: Log, i: int) -> bool {
  0 <= i < l.len() && is_io_event_ready(l[i])
}

pub open spec fn is_poll_events_at(l: Log, i: int) -> bool {
  0 <= i < l.len() && is_poll_events(l[i])
}

pub open spec fn is_inbound_register_io_begin_at(l: Log, i: int) -> bool {
  0 <= i < l.len() && is_inbound_register_io_begin(l[i])
}

pub open spec fn is_inbound_register_io_end_at(l: Log, i: int) -> bool {
  0 <= i < l.len() && is_inbound_register_io_end(l[i])
}

pub open spec fn is_inbound_deregister_io_begin_at(l: Log, i: int) -> bool {
  0 <= i < l.len() && is_inbound_deregister_io_begin(l[i])
}

pub open spec fn is_inbound_deregister_io_end_at(l: Log, i: int) -> bool {
  0 <= i < l.len() && is_inbound_deregister_io_end(l[i])
}

// ============================================================================
// Park Cycle Helpers
// ============================================================================

pub open spec fn find_last_park_begin(l: Log, i: int) -> int
  recommends 0 <= i < l.len()
  decreases i + 1 when i >= -1
{
  if i < 0 {
    -1
  } else if i < l.len() && is_park_begin(l[i]) {
    i
  } else {
    find_last_park_begin(l, i - 1)
  }
}

// Scans back from i-1 and resets to -1 at a park_end (i.e. -1 when index i is
// not inside an open park cycle).
pub open spec fn current_park_start(l: Log, i: int) -> int
  decreases i,
{
  if i <= 0 { -1int }
  else if 0 <= i - 1 < l.len() && is_park_begin_at(l, i - 1) { (i - 1) as int }
  else if 0 <= i - 1 < l.len() && is_park_end_at(l, i - 1) { -1int }
  else { current_park_start(l, i - 1) }
}

// ============================================================================
// Timer Retirement
// ============================================================================

pub closed spec fn timer_retired_at(l: Log, rid: ResourceIdView, j: int) -> bool {
  (is_succ_deregister_timer_at(l, j) && get_deregister_timer_rid(l[j]) == rid) ||
  (is_wake_task_at(l, j) && get_wake_task_source_rid(l[j]) == rid)
}

// --- lemma family in the historical lion-liveness names ---

pub proof fn reveal_timer_retired_from_deregister(l: Log, rid: ResourceIdView, j: int)
  requires
    is_succ_deregister_timer_at(l, j),
    get_deregister_timer_rid(l[j]) == rid,
  ensures
    timer_retired_at(l, rid, j),
{}

pub proof fn reveal_timer_retired_from_wake(l: Log, rid: ResourceIdView, j: int)
  requires
    is_wake_task_at(l, j),
    get_wake_task_source_rid(l[j]) == rid,
  ensures
    timer_retired_at(l, rid, j),
{}

pub proof fn reveal_timer_retired_implies(l: Log, rid: ResourceIdView, j: int)
  requires
    timer_retired_at(l, rid, j),
  ensures
    (is_succ_deregister_timer_at(l, j) && get_deregister_timer_rid(l[j]) == rid) ||
    (is_wake_task_at(l, j) && get_wake_task_source_rid(l[j]) == rid),
{}

// --- lemma family in the historical lion-reactor names ---

pub proof fn timer_retired_from_deregister(l: Log, rid: ResourceIdView, j: int)
  requires is_succ_deregister_timer_at(l, j), get_deregister_timer_rid(l[j]) == rid,
  ensures timer_retired_at(l, rid, j),
{}

pub proof fn timer_retired_from_wake(l: Log, rid: ResourceIdView, j: int)
  requires is_wake_task_at(l, j), get_wake_task_source_rid(l[j]) == rid,
  ensures timer_retired_at(l, rid, j),
{}

pub proof fn timer_retired_implies(l: Log, rid: ResourceIdView, j: int)
  requires timer_retired_at(l, rid, j),
  ensures
    (is_deregister_timer_at(l, j) && get_deregister_timer_rid(l[j]) == rid) ||
    (is_wake_task_at(l, j) && get_wake_task_source_rid(l[j]) == rid),
{}

pub proof fn timer_retired_is_deregister_when_not_wake(l: Log, rid: ResourceIdView, j: int)
  requires
    timer_retired_at(l, rid, j),
    !is_wake_task_at(l, j),
  ensures
    is_deregister_timer_at(l, j),
    get_deregister_timer_rid(l[j]) == rid,
{}

pub proof fn timer_retired_is_wake_when_not_deregister(l: Log, rid: ResourceIdView, j: int)
  requires
    timer_retired_at(l, rid, j),
    !is_deregister_timer_at(l, j),
  ensures
    is_wake_task_at(l, j),
    get_wake_task_source_rid(l[j]) == rid,
{}

pub proof fn timer_retired_preserved(l: Log, e: ReactorEvent, rid: ResourceIdView, j: int)
  requires timer_retired_at(l, rid, j), j < l.len(),
  ensures timer_retired_at(l.push(e), rid, j),
{
  assert(l.push(e)[j] == l[j]);
}

pub proof fn not_timer_retired(l: Log, rid: ResourceIdView, k: int)
  requires
    !(is_deregister_timer_at(l, k) && get_deregister_timer_rid(l[k]) == rid),
    !(is_wake_task_at(l, k) && get_wake_task_source_rid(l[k]) == rid),
  ensures
    !timer_retired_at(l, rid, k),
{}

pub proof fn not_timer_retired_implies(l: Log, rid: ResourceIdView, k: int)
  requires
    !timer_retired_at(l, rid, k),
  ensures
    !(is_succ_deregister_timer_at(l, k) && get_deregister_timer_rid(l[k]) == rid),
    !(is_wake_task_at(l, k) && get_wake_task_source_rid(l[k]) == rid),
{}

pub proof fn not_timer_retired_preserved(l: Log, e: ReactorEvent, rid: ResourceIdView, k: int)
  requires
    !timer_retired_at(l, rid, k),
    k < l.len(),
  ensures
    !timer_retired_at(l.push(e), rid, k),
{
  if timer_retired_at(l.push(e), rid, k) {
    assert(l.push(e)[k] == l[k]);
  }
}

pub proof fn not_timer_retired_shrink(l: Log, e: ReactorEvent, rid: ResourceIdView, k: int)
  requires
    !timer_retired_at(l.push(e), rid, k),
    k < l.len(),
  ensures
    !timer_retired_at(l, rid, k),
{
  if timer_retired_at(l, rid, k) {
    assert(l.push(e)[k] == l[k]);
  }
}

pub proof fn timer_retired_at_transfer(l1: Log, l2: Log, rid: ResourceIdView, k: int)
  requires
    l1[k] == l2[k],
    timer_retired_at(l1, rid, k),
    0 <= k < l1.len(),
    0 <= k < l2.len(),
  ensures
    timer_retired_at(l2, rid, k),
{}

pub proof fn not_timer_retired_transfer(l1: Log, l2: Log, rid: ResourceIdView, k: int)
  requires
    l1[k] == l2[k],
    !timer_retired_at(l1, rid, k),
    0 <= k < l1.len(),
    0 <= k < l2.len(),
  ensures
    !timer_retired_at(l2, rid, k),
{}

pub proof fn not_timer_retired_from_push_new(l: Log, e: ReactorEvent, rid: ResourceIdView)
  requires
    !(is_deregister_timer_at(l.push(e), l.len() as int) && get_deregister_timer_rid(e) == rid),
    !(is_wake_task_at(l.push(e), l.len() as int) && get_wake_task_source_rid(e) == rid),
  ensures
    !timer_retired_at(l.push(e), rid, l.len() as int),
{ assert(l.push(e)[l.len() as int] == e); }

pub proof fn timer_retired_transfer_to_extension(l: Log, l_prime: Log, rid: ResourceIdView, j: int)
  requires
    is_prefix_of(l, l_prime),
    0 <= j < l.len(),
    timer_retired_at(l, rid, j),
  ensures
    timer_retired_at(l_prime, rid, j),
{
  reveal_timer_retired_implies(l, rid, j);
  assert(l[j] == l_prime[j]);
  if is_succ_deregister_timer_at(l, j) && get_deregister_timer_rid(l[j]) == rid {
    assert(is_succ_deregister_timer_at(l_prime, j));
    reveal_timer_retired_from_deregister(l_prime, rid, j);
  } else {
    assert(is_wake_task_at(l, j) && get_wake_task_source_rid(l[j]) == rid);
    assert(is_wake_task_at(l_prime, j));
    reveal_timer_retired_from_wake(l_prime, rid, j);
  }
}

// ============================================================================
// Timer / IO Activity
// ============================================================================

// Impl-canonical form: carries the registration conjunct in the body (the
// historical lion-liveness form put it in a `recommends`).
pub open spec fn timer_active_at(l: Log, register_idx: int, i: int) -> bool {
  let rid = get_register_timer_rid(l[register_idx]);
  is_succ_register_timer_at(l, register_idx) &&
  register_idx < i &&
  forall |j: int| register_idx < j < i ==> !#[trigger] timer_retired_at(l, rid, j)
}

pub proof fn timer_active_transfer_to_prefix(l: Log, l_prime: Log, register_idx: int, i: int)
  requires
    is_prefix_of(l, l_prime),
    is_succ_register_timer_at(l, register_idx),
    0 <= register_idx < l.len(),
    register_idx < i <= l.len(),
    timer_active_at(l_prime, register_idx, i),
  ensures
    timer_active_at(l, register_idx, i),
{
  let rid = get_register_timer_rid(l[register_idx]);
  assert(rid == get_register_timer_rid(l_prime[register_idx]));
  assert forall |j: int| register_idx < j < i implies !#[trigger] timer_retired_at(l, rid, j) by {
    if timer_retired_at(l, rid, j) {
      timer_retired_transfer_to_extension(l, l_prime, rid, j);
    }
  };
}

pub open spec fn timer_remains_active_between(l: Log, register_idx: int, start: int, end: int) -> bool
  recommends is_succ_register_timer_at(l, register_idx)
{
  forall |i: int| start <= i < end ==> timer_active_at(l, register_idx, i)
}

// IO activity — API anchor (impl form: forall-not over inbound deregisters).
pub open spec fn io_api_active_at(l: Log, register_idx: int, i: int) -> bool {
  io_api_registered_at(l, register_idx) &&
  register_idx < i &&
  forall |k: int| register_idx < k < i ==> !(
    io_api_deregistered_at(l, k) &&
    get_io_api_deregister_rid(l[k]) == get_io_api_register_rid(l[register_idx])
  )
}

// IO activity — syscall anchor (liveness form: not-exists over outbound
// deregisters).
pub open spec fn io_syscall_active_at(l: Log, register_idx: int, i: int) -> bool
  recommends io_syscall_registered_at(l, register_idx)
{
  let rid = get_io_syscall_register_rid(l[register_idx]);
  register_idx < i &&
  !exists |j: int| #![trigger l[j]]
    register_idx < j < i &&
    io_syscall_deregistered_at(l, j) &&
    get_io_syscall_deregister_rid(l[j]) == rid
}

// API anchor: some active registration for rid exists before the set_waker.
pub open spec fn io_api_active_at_set_waker(l: Log, rid: ResourceIdView, set_waker_idx: int) -> bool {
  exists |reg_idx: int| 0 <= reg_idx < set_waker_idx &&
    io_api_registered_at(l, reg_idx) &&
    get_io_api_register_rid(l[reg_idx]) == rid &&
    io_api_active_at(l, reg_idx, set_waker_idx)
}

// Syscall anchor: the registration most recently BEFORE the set_waker is still
// active at the set_waker.
pub open spec fn find_io_syscall_register_for_rid(l: Log, rid: ResourceIdView, before: int) -> int
  decreases before
{
  if before <= 0 {
    -1
  } else if io_syscall_registered_at(l, before - 1) && get_io_syscall_register_rid(l[before - 1]) == rid {
    before - 1
  } else {
    find_io_syscall_register_for_rid(l, rid, before - 1)
  }
}

// find_io_syscall_register_for_rid returns a valid io registration for rid when >= 0.
pub proof fn find_io_syscall_register_for_rid_valid(l: Log, rid: ResourceIdView, before: int)
  requires
    find_io_syscall_register_for_rid(l, rid, before) >= 0,
  ensures
    0 <= find_io_syscall_register_for_rid(l, rid, before) < before,
    io_syscall_registered_at(l, find_io_syscall_register_for_rid(l, rid, before)),
    get_io_syscall_register_rid(l[find_io_syscall_register_for_rid(l, rid, before)]) == rid,
  decreases before
{
  if before <= 0 {
  } else if io_syscall_registered_at(l, before - 1) && get_io_syscall_register_rid(l[before - 1]) == rid {
  } else {
    find_io_syscall_register_for_rid_valid(l, rid, before - 1);
  }
}

// find_io_syscall_register_for_rid is the LARGEST io registration index <
// before: any io registration for rid at k < before is <= the result.
pub proof fn find_io_syscall_register_for_rid_ge(l: Log, rid: ResourceIdView, before: int, k: int)
  requires
    0 <= k < before,
    io_syscall_registered_at(l, k),
    get_io_syscall_register_rid(l[k]) == rid,
  ensures
    find_io_syscall_register_for_rid(l, rid, before) >= k,
  decreases before
{
  if before <= 0 {
  } else if io_syscall_registered_at(l, before - 1) && get_io_syscall_register_rid(l[before - 1]) == rid {
  } else {
    if before - 1 == k {
      assert(false);
    }
    find_io_syscall_register_for_rid_ge(l, rid, before - 1, k);
  }
}

pub open spec fn io_syscall_active_at_set_waker(l: Log, rid: ResourceIdView, set_waker_idx: int) -> bool {
  let register_idx = find_io_syscall_register_for_rid(l, rid, set_waker_idx);
  register_idx >= 0 &&
  register_idx < set_waker_idx &&
  io_syscall_active_at(l, register_idx, set_waker_idx)
}

// ============================================================================
// Wake Task Existence
// ============================================================================

pub open spec fn has_wake_task_for_timer_after(l: Log, register_idx: int, start: int) -> bool
  recommends is_succ_register_timer_at(l, register_idx)
{
  let rid = get_register_timer_rid(l[register_idx]);
  let waker = get_register_timer_waker(l[register_idx]);
  exists |i: int| #![trigger l[i]]
    start <= i < l.len() &&
    is_wake_task_at(l, i) &&
    get_wake_task_source_rid(l[i]) == rid &&
    get_wake_task_waker(l[i]) == waker
}

pub open spec fn has_io_event_ready_between(l: Log, rid: ResourceIdView, start: int, end: int) -> bool {
  exists |k: int| start < k < end &&
    is_io_event_ready_at(l, k) &&
    get_io_event(l[k]).resource_id == rid
}

// ============================================================================
// Timestamps
// ============================================================================

pub open spec fn max_timestamp_up_to(l: Log, i: int) -> InstantView
  decreases i
{
  if i <= 0 {
    0  // initial timestamp
  } else if is_get_current_time_at(l, i - 1) {
    let ts = get_current_timestamp(l[i - 1]);
    let prev_max = max_timestamp_up_to(l, i - 1);
    if ts > prev_max { ts } else { prev_max }
  } else {
    max_timestamp_up_to(l, i - 1)
  }
}

// CLOCK-GRANULARITY IDEALIZATION: strict monotonicity is an environment
// idealization — the implementation clock guarantees only non-strict
// monotonicity (millisecond granularity, clamped to wheel.elapsed). Runs where
// two GetCurrentTime reads observe the same millisecond are outside this
// clause. Single definition: lion-liveness re-exports this spec fn
// (composed/spec/assumptions.rs). See TCB_and_limitations.md §2 for why
// strictness is load-bearing.
pub open spec fn timestamps_strictly_increasing(l: Log) -> bool {
  forall |i: int, j: int|
    #![trigger get_current_timestamp(l[i]), get_current_timestamp(l[j])]
    0 <= i < j < l.len() &&
    is_get_current_time_at(l, i) &&
    is_get_current_time_at(l, j) ==>
    get_current_timestamp(l[i]) < get_current_timestamp(l[j])
}

pub open spec fn timestamps_positive(l: Log) -> bool {
  forall |i: int| #![trigger l[i]]
    0 <= i < l.len() &&
    is_get_current_time_at(l, i) ==>
    get_current_timestamp(l[i]) >= 1
}

// ============================================================================
// Prefix and Extension
// ============================================================================

pub open spec fn is_prefix_of(l: Log, l_prime: Log) -> bool {
  l.len() <= l_prime.len() &&
  l =~= l_prime.subrange(0, l.len() as int)
}

// ============================================================================
// No-prior-registration families
// ============================================================================
//
// Timer side is anchor-neutral (both crates agree); io side is split into the
// API-anchor forms (impl) and the syscall-anchor forms (liveness).

pub open spec fn no_prior_timer_registration(l: Log, rid: ResourceIdView, end: int) -> bool {
  forall |k: int| 0 <= k < end && is_succ_register_timer_at(l, k) && get_register_timer_rid(l[k]) == rid ==>
    exists |j: int| k < j < end && #[trigger] timer_retired_at(l, rid, j)
}

pub proof fn reveal_no_prior_timer_registration(l: Log, rid: ResourceIdView, end: int)
  requires
    no_prior_timer_registration(l, rid, end),
  ensures
    forall |k: int| 0 <= k < end && is_succ_register_timer_at(l, k) && get_register_timer_rid(l[k]) == rid ==>
      exists |j: int| k < j < end && #[trigger] timer_retired_at(l, rid, j),
{}

pub proof fn intro_no_prior_timer_registration(l: Log, rid: ResourceIdView, end: int)
  requires
    forall |k: int| 0 <= k < end && is_succ_register_timer_at(l, k) && get_register_timer_rid(l[k]) == rid ==>
      exists |j: int| k < j < end && #[trigger] timer_retired_at(l, rid, j),
  ensures
    no_prior_timer_registration(l, rid, end),
{}

// Historical impl-side duplicate of no_prior_timer_registration (kept: the two
// names are used by different invariant clauses on both sides).
pub open spec fn no_timer_with_rid_before(l: Log, rid: ResourceIdView, end: int) -> bool {
  forall |k: int| 0 <= k < end && is_succ_register_timer_at(l, k) && get_register_timer_rid(l[k]) == rid ==>
    exists |j: int| k < j < end && #[trigger] timer_retired_at(l, rid, j)
}

pub proof fn reveal_no_timer_with_rid_before(l: Log, rid: ResourceIdView, end: int)
  requires
    no_timer_with_rid_before(l, rid, end),
  ensures
    forall |k: int| 0 <= k < end && is_succ_register_timer_at(l, k) && get_register_timer_rid(l[k]) == rid ==>
      exists |j: int| k < j < end && #[trigger] timer_retired_at(l, rid, j),
{}

pub proof fn intro_no_timer_with_rid_before(l: Log, rid: ResourceIdView, end: int)
  requires
    forall |k: int| 0 <= k < end && is_succ_register_timer_at(l, k) && get_register_timer_rid(l[k]) == rid ==>
      exists |j: int| k < j < end && #[trigger] timer_retired_at(l, rid, j),
  ensures
    no_timer_with_rid_before(l, rid, end),
{}

// API anchor (impl)
pub open spec fn no_prior_io_api_registration(l: Log, rid: ResourceIdView, end: int) -> bool {
  forall |k: int| 0 <= k < end && io_api_registered_at(l, k) && get_io_api_register_rid(l[k]) == rid ==>
    exists |j: int| k < j < end && io_api_deregistered_at(l, j) && get_io_api_deregister_rid(l[j]) == rid
}

pub open spec fn no_io_api_with_rid_before(l: Log, rid: ResourceIdView, end: int) -> bool {
  forall |k: int| 0 <= k < end && io_api_registered_at(l, k) && get_io_api_register_rid(l[k]) == rid ==>
    exists |j: int| k < j < end && io_api_deregistered_at(l, j) && get_io_api_deregister_rid(l[j]) == rid
}

// Syscall anchor (liveness; closed with reveal/intro companions, as before)
pub closed spec fn no_prior_io_syscall_registration(l: Log, rid: ResourceIdView, end: int) -> bool {
  forall |k: int| 0 <= k < end && io_syscall_registered_at(l, k) && get_io_syscall_register_rid(l[k]) == rid ==>
    exists |j: int| k < j < end && #[trigger] io_syscall_deregistered_at(l, j) && get_io_syscall_deregister_rid(l[j]) == rid
}

pub proof fn reveal_no_prior_io_syscall_registration(l: Log, rid: ResourceIdView, end: int)
  requires
    no_prior_io_syscall_registration(l, rid, end),
  ensures
    forall |k: int| 0 <= k < end && io_syscall_registered_at(l, k) && get_io_syscall_register_rid(l[k]) == rid ==>
      exists |j: int| k < j < end && #[trigger] io_syscall_deregistered_at(l, j) && get_io_syscall_deregister_rid(l[j]) == rid,
{}

pub proof fn intro_no_prior_io_syscall_registration(l: Log, rid: ResourceIdView, end: int)
  requires
    forall |k: int| 0 <= k < end && io_syscall_registered_at(l, k) && get_io_syscall_register_rid(l[k]) == rid ==>
      exists |j: int| k < j < end && #[trigger] io_syscall_deregistered_at(l, j) && get_io_syscall_deregister_rid(l[j]) == rid,
  ensures
    no_prior_io_syscall_registration(l, rid, end),
{}

pub closed spec fn no_io_syscall_registration_with_rid(l: Log, rid: ResourceIdView, end: int) -> bool {
  forall |k: int| 0 <= k < end && io_syscall_registered_at(l, k) && get_io_syscall_register_rid(l[k]) == rid ==>
    exists |j: int| k < j < end && #[trigger] io_syscall_deregistered_at(l, j) && get_io_syscall_deregister_rid(l[j]) == rid
}

pub proof fn reveal_no_io_syscall_registration_with_rid(l: Log, rid: ResourceIdView, end: int)
  requires
    no_io_syscall_registration_with_rid(l, rid, end),
  ensures
    forall |k: int| 0 <= k < end && io_syscall_registered_at(l, k) && get_io_syscall_register_rid(l[k]) == rid ==>
      exists |j: int| k < j < end && #[trigger] io_syscall_deregistered_at(l, j) && get_io_syscall_deregister_rid(l[j]) == rid,
{}

pub proof fn intro_no_io_syscall_registration_with_rid(l: Log, rid: ResourceIdView, end: int)
  requires
    forall |k: int| 0 <= k < end && io_syscall_registered_at(l, k) && get_io_syscall_register_rid(l[k]) == rid ==>
      exists |j: int| k < j < end && #[trigger] io_syscall_deregistered_at(l, j) && get_io_syscall_deregister_rid(l[j]) == rid,
  ensures
    no_io_syscall_registration_with_rid(l, rid, end),
{}

// --- append-preservation lemmas (pure-log, from lion-reactor) ---

pub proof fn no_prior_timer_reg_preserved(l: Log, e: ReactorEvent, rid: ResourceIdView, i: int)
  requires
    no_prior_timer_registration(l, rid, i),
    i <= l.len(),
  ensures
    no_prior_timer_registration(l.push(e), rid, i),
{
  let l2 = l.push(e);
  assert forall |k: int| 0 <= k < i && is_succ_register_timer_at(l2, k) && get_register_timer_rid(l2[k]) == rid implies
    exists |j: int| k < j < i && #[trigger] timer_retired_at(l2, rid, j)
  by {
    assert(l2[k] == l[k]);
    assert(is_succ_register_timer_at(l, k));
    let j = choose |j: int| k < j < i && #[trigger] timer_retired_at(l, rid, j);
    timer_retired_preserved(l, e, rid, j);
  }
}

pub proof fn no_prior_io_api_reg_preserved(l: Log, e: ReactorEvent, rid: ResourceIdView, i: int)
  requires
    no_prior_io_api_registration(l, rid, i),
    i <= l.len(),
  ensures
    no_prior_io_api_registration(l.push(e), rid, i),
{
  let l2 = l.push(e);
  assert forall |k: int| 0 <= k < i && io_api_registered_at(l2, k) && get_io_api_register_rid(l2[k]) == rid implies
    exists |j: int| k < j < i && io_api_deregistered_at(l2, j) && get_io_api_deregister_rid(l2[j]) == rid
  by {
    assert(l2[k] == l[k]);
    assert(io_api_registered_at(l, k));
    let j = choose |j: int| k < j < i && io_api_deregistered_at(l, j) && get_io_api_deregister_rid(l[j]) == rid;
    assert(l2[j] == l[j]);
    assert(io_api_deregistered_at(l2, j));
    assert(get_io_api_deregister_rid(l2[j]) == rid);
  }
}

pub proof fn no_timer_with_rid_preserved(l: Log, e: ReactorEvent, rid: ResourceIdView, i: int)
  requires
    no_timer_with_rid_before(l, rid, i),
    i <= l.len(),
  ensures
    no_timer_with_rid_before(l.push(e), rid, i),
{
  let l2 = l.push(e);
  assert forall |k: int| 0 <= k < i && is_succ_register_timer_at(l2, k) && get_register_timer_rid(l2[k]) == rid implies
    exists |j: int| k < j < i && #[trigger] timer_retired_at(l2, rid, j)
  by {
    assert(l2[k] == l[k]);
    assert(is_succ_register_timer_at(l, k));
    let j = choose |j: int| k < j < i && #[trigger] timer_retired_at(l, rid, j);
    timer_retired_preserved(l, e, rid, j);
  }
}

pub proof fn no_io_api_with_rid_preserved(l: Log, e: ReactorEvent, rid: ResourceIdView, i: int)
  requires
    no_io_api_with_rid_before(l, rid, i),
    i <= l.len(),
  ensures
    no_io_api_with_rid_before(l.push(e), rid, i),
{
  let l2 = l.push(e);
  assert forall |k: int| 0 <= k < i && io_api_registered_at(l2, k) && get_io_api_register_rid(l2[k]) == rid implies
    exists |j: int| k < j < i && io_api_deregistered_at(l2, j) && get_io_api_deregister_rid(l2[j]) == rid
  by {
    assert(l2[k] == l[k]);
    assert(io_api_registered_at(l, k));
    let j = choose |j: int| k < j < i && io_api_deregistered_at(l, j) && get_io_api_deregister_rid(l[j]) == rid;
    assert(l2[j] == l[j]);
    assert(io_api_deregistered_at(l2, j));
    assert(get_io_api_deregister_rid(l2[j]) == rid);
  }
}

pub proof fn freshness_preserved_by_append(l: Log, e: ReactorEvent, rid: ResourceIdView)
  requires
    no_prior_io_api_registration(l, rid, l.len() as int),
    no_timer_with_rid_before(l, rid, l.len() as int),
    no_prior_timer_registration(l, rid, l.len() as int),
    no_io_api_with_rid_before(l, rid, l.len() as int),
    !io_api_registered_at(l.push(e), l.len() as int),
    !is_succ_register_timer_at(l.push(e), l.len() as int),
    !timer_retired_at(l.push(e), rid, l.len() as int),
    !io_api_deregistered_at(l.push(e), l.len() as int),
  ensures
    no_prior_io_api_registration(l.push(e), rid, (l.len() + 1) as int),
    no_timer_with_rid_before(l.push(e), rid, (l.len() + 1) as int),
    no_prior_timer_registration(l.push(e), rid, (l.len() + 1) as int),
    no_io_api_with_rid_before(l.push(e), rid, (l.len() + 1) as int),
{
  let l2 = l.push(e);
  let n = l.len() as int;
  no_prior_timer_reg_preserved(l, e, rid, n);
  no_prior_io_api_reg_preserved(l, e, rid, n);
  no_timer_with_rid_preserved(l, e, rid, n);
  no_io_api_with_rid_preserved(l, e, rid, n);
  assert forall |k: int| 0 <= k < n + 1 && is_succ_register_timer_at(l2, k) && get_register_timer_rid(l2[k]) == rid implies
    exists |j: int| k < j < n + 1 && #[trigger] timer_retired_at(l2, rid, j)
  by {
    assert(k < n);
    assert(l2[k] == l[k]);
    assert(is_succ_register_timer_at(l, k));
    let j = choose |j: int| k < j < n && #[trigger] timer_retired_at(l, rid, j);
    timer_retired_preserved(l, e, rid, j);
  }
  assert forall |k: int| 0 <= k < n + 1 && io_api_registered_at(l2, k) && get_io_api_register_rid(l2[k]) == rid implies
    exists |j: int| k < j < n + 1 && io_api_deregistered_at(l2, j) && get_io_api_deregister_rid(l2[j]) == rid
  by {
    assert(k < n);
    assert(l2[k] == l[k]);
    assert(io_api_registered_at(l, k));
    let j = choose |j: int| k < j < n && io_api_deregistered_at(l, j) && get_io_api_deregister_rid(l[j]) == rid;
    assert(l2[j] == l[j]);
  }
}

#[verifier::rlimit(50)]
pub proof fn no_prior_timer_reg_after_deregister_timer(l: Log, e: ReactorEvent, rid: ResourceIdView)
  requires
    is_succ_deregister_timer_at(l.push(e), l.len() as int),
    get_deregister_timer_rid(l.push(e)[l.len() as int]) == rid,
  ensures
    no_prior_timer_registration(l.push(e), rid, l.push(e).len() as int),
    no_timer_with_rid_before(l.push(e), rid, l.push(e).len() as int),
{
  let l2 = l.push(e);
  let n = l.len() as int;
  assert forall |k: int| 0 <= k < n + 1 && is_succ_register_timer_at(l2, k) && get_register_timer_rid(l2[k]) == rid implies
    exists |j: int| k < j < n + 1 && #[trigger] timer_retired_at(l2, rid, j)
  by {
    timer_retired_from_deregister(l2, rid, n);
  }
}

#[verifier::rlimit(50)]
pub proof fn no_prior_timer_reg_after_wake(l: Log, e: ReactorEvent, rid: ResourceIdView)
  requires
    is_wake_task_at(l.push(e), l.len() as int),
    get_wake_task_source_rid(l.push(e)[l.len() as int]) == rid,
  ensures
    no_prior_timer_registration(l.push(e), rid, l.push(e).len() as int),
    no_timer_with_rid_before(l.push(e), rid, l.push(e).len() as int),
{
  let l2 = l.push(e);
  let n = l.len() as int;
  assert forall |k: int| 0 <= k < n + 1 && is_succ_register_timer_at(l2, k) && get_register_timer_rid(l2[k]) == rid implies
    exists |j: int| k < j < n + 1 && #[trigger] timer_retired_at(l2, rid, j)
  by {
    timer_retired_from_wake(l2, rid, n);
  }
}

#[verifier::rlimit(50)]
pub proof fn no_prior_io_api_reg_after_deregister_io(l: Log, e: ReactorEvent, rid: ResourceIdView)
  requires
    io_api_deregistered_at(l.push(e), l.len() as int),
    get_io_api_deregister_rid(l.push(e)[l.len() as int]) == rid,
  ensures
    no_prior_io_api_registration(l.push(e), rid, l.push(e).len() as int),
    no_io_api_with_rid_before(l.push(e), rid, l.push(e).len() as int),
{
  let l2 = l.push(e);
  let n = l.len() as int;
  assert forall |k: int| 0 <= k < n + 1 && io_api_registered_at(l2, k) && get_io_api_register_rid(l2[k]) == rid implies
    exists |j: int| k < j < n + 1 && io_api_deregistered_at(l2, j) && get_io_api_deregister_rid(l2[j]) == rid
  by {
  }
}

// ============================================================================
// Timeout points and last-set-waker searches (impl-side choose forms)
// ============================================================================

pub open spec fn has_timeout_point(l: Log, register_idx: int) -> bool {
  exists |timeout_idx: int| timeout_idx > register_idx &&
    is_get_current_time_at(l, timeout_idx) &&
    get_current_timestamp(l[timeout_idx]) >= get_register_timer_deadline(l[register_idx]) &&
    timer_active_at(l, register_idx, timeout_idx)
}

pub open spec fn first_timeout_point(l: Log, register_idx: int) -> int {
  choose |timeout_idx: int| timeout_idx > register_idx &&
    is_get_current_time_at(l, timeout_idx) &&
    get_current_timestamp(l[timeout_idx]) >= get_register_timer_deadline(l[register_idx]) &&
    timer_active_at(l, register_idx, timeout_idx) &&
    forall |k: int| register_idx < k < timeout_idx ==> !(
      is_get_current_time_at(l, k) &&
      get_current_timestamp(l[k]) >= get_register_timer_deadline(l[register_idx]) &&
      timer_active_at(l, register_idx, k)
    )
}

pub open spec fn find_last_set_waker_for_rid_readable(l: Log, rid: ResourceIdView, before: int) -> int {
  choose |j: int| 0 <= j < before &&
    is_succ_set_waker_at(l, j) &&
    get_set_waker_rid(l[j]) == rid &&
    get_set_waker_interest(l[j]).0 &&
    forall |k: int| j < k < before ==> !(
      is_succ_set_waker_at(l, k) &&
      get_set_waker_rid(l[k]) == rid &&
      get_set_waker_interest(l[k]).0
    )
}

pub open spec fn find_last_set_waker_for_rid_writable(l: Log, rid: ResourceIdView, before: int) -> int {
  choose |j: int| 0 <= j < before &&
    is_succ_set_waker_at(l, j) &&
    get_set_waker_rid(l[j]) == rid &&
    get_set_waker_interest(l[j]).1 &&
    forall |k: int| j < k < before ==> !(
      is_succ_set_waker_at(l, k) &&
      get_set_waker_rid(l[k]) == rid &&
      get_set_waker_interest(l[k]).1
    )
}

// Valid-set-waker witnesses — API anchor (impl semantics).
pub open spec fn has_valid_set_waker_readable_api(l: Log, io_ready_idx: int) -> bool {
  let event = get_io_event(l[io_ready_idx]);
  let rid = event.resource_id;
  event.readable &&
  exists |sw_idx: int| 0 <= sw_idx < io_ready_idx &&
    is_succ_set_waker_at(l, sw_idx) &&
    get_set_waker_rid(l[sw_idx]) == rid &&
    get_set_waker_interest(l[sw_idx]).0 &&
    io_api_active_at_set_waker(l, rid, sw_idx) &&
    forall |k: int| sw_idx < k < io_ready_idx ==> !(
      io_api_deregistered_at(l, k) && get_io_api_deregister_rid(l[k]) == rid
    )
}

pub open spec fn has_valid_set_waker_writable_api(l: Log, io_ready_idx: int) -> bool {
  let event = get_io_event(l[io_ready_idx]);
  let rid = event.resource_id;
  event.writable &&
  exists |sw_idx: int| 0 <= sw_idx < io_ready_idx &&
    is_succ_set_waker_at(l, sw_idx) &&
    get_set_waker_rid(l[sw_idx]) == rid &&
    get_set_waker_interest(l[sw_idx]).1 &&
    io_api_active_at_set_waker(l, rid, sw_idx) &&
    forall |k: int| sw_idx < k < io_ready_idx ==> !(
      io_api_deregistered_at(l, k) && get_io_api_deregister_rid(l[k]) == rid
    )
}

}
