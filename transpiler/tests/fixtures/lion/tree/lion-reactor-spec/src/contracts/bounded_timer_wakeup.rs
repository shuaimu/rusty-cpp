use vstd::prelude::*;
use crate::log::*;
use crate::events::*;
#[allow(unused_imports)]
use crate::types::*;
use lion_framework_spec::async_contract::*;

verus! {

// ============================================================================
// Bounded Timer Wakeup Contract (liveness-side; moved unchanged)
// ============================================================================
//
// Semantics:
// - acceptance: A timer is successfully registered with resource_id `rid`
// - fulfillment: WakeTask event is generated for this timer
// - assumption: Timer remains active, timestamps strictly increase
//
// Bound computation:
//   Given deadline and current max timestamp, the bound is:
//   k = (deadline - current_max_ts) + 1
//   Since timestamps strictly increase each GetCurrentTime, and each park
//   cycle has at least one GetCurrentTime (PARK_HAS_TIMESTAMP), after
//   (deadline - current) rounds we reach the deadline.
//
// ============================================================================

pub open spec fn trigger_fn(l: Log, rid: ResourceIdView) -> bool {
  exists |i: int| #![trigger l[i]]
    0 <= i < l.len() &&
    is_succ_register_timer_at(l, i) &&
    get_register_timer_rid(l[i]) == rid
}

pub open spec fn response_fn(l: Log, rid: ResourceIdView) -> bool {
  let register_idx = find_register_timer_idx(l, rid);
  register_idx >= 0 && {
    let waker = get_register_timer_waker(l[register_idx]);
    exists |i: int| #![trigger l[i]]
      0 <= i < l.len() &&
      is_wake_task_at(l, i) &&
      get_wake_task_source_rid(l[i]) == rid &&
      get_wake_task_waker(l[i]) == waker
  }
}

// Single-state form (satisfiable). The old `timer_assumption` quantified over
// ALL log extensions and was constant false (an adversarial extension that
// retires the timer always exists); it and its sole consumer
// `timer_bounded_liveness_proof` have been removed. The per-trace-state form of
// these facts is what the live env path (`env_reactor_timer`) consumes.
pub open spec fn assumption_fn(l: Log, rid: ResourceIdView) -> bool {
  timestamps_strictly_increasing_assumption(l) &&
  timestamps_positive_assumption(l) &&
  timer_remains_active_assumption(l, rid)
}

pub open spec fn bounded_timer_wakeup() -> AsyncContract<Log, ResourceIdView> {
  AsyncContract {
    acceptance: |l: Log, rid: ResourceIdView| trigger_fn(l, rid),
    fulfillment: |l: Log, rid: ResourceIdView| response_fn(l, rid),
    assumption: |l: Log, rid: ResourceIdView| assumption_fn(l, rid),
  }
}

// ============================================================================
// PER-REGISTRATION timer contract (Phase 0 / Option A) — T = reactor-log INDEX
// of the register event, NOT the rid. Reuse-tolerant (each registration is its
// own contract instance) and MONOTONE (i is fixed, so response_at is about i's
// fixed waker firing — no leftmost/most-recent anchor to move). Framework-safe:
// AsyncContract<L,T> is generic in T (ResourceIdView / TID / UID already coexist),
// so this is just another instantiation — the generic template is untouched.
// ============================================================================

// acceptance: i is a successful RegisterTimer event.
pub open spec fn trigger_at(l: Log, i: int) -> bool {
  0 <= i < l.len() && is_succ_register_timer_at(l, i)
}

// fulfillment: a WakeTask fires carrying i's own (rid, waker) — i's timer woke its
// own waker. MONOTONE in l for fixed i (an existing WakeTask persists).
pub open spec fn response_at(l: Log, i: int) -> bool {
  0 <= i < l.len() &&
  exists |w: int|
    #![trigger l[w]]
    0 <= w < l.len() &&
    is_wake_task_at(l, w) &&
    get_wake_task_source_rid(l[w]) == get_register_timer_rid(l[i]) &&
    get_wake_task_waker(l[w]) == get_register_timer_waker(l[i])
}

// assumption: the timer registered at i is not deregistered through the log
// (anchored at the SPECIFIC registration i — reuse of the rid via other
// registrations is unconstrained).
pub open spec fn assumption_at(l: Log, i: int) -> bool {
  timestamps_strictly_increasing_assumption(l) &&
  timestamps_positive_assumption(l) &&
  crate::invariants::wake_on_expired::timer_not_deregistered_through(l, i, l.len() as int)
}

pub open spec fn bounded_timer_wakeup_at() -> AsyncContract<Log, int> {
  AsyncContract {
    acceptance: |l: Log, i: int| trigger_at(l, i),
    fulfillment: |l: Log, i: int| response_at(l, i),
    assumption: |l: Log, i: int| assumption_at(l, i),
  }
}

// ============================================================================
// Assumption Definitions
// ============================================================================

pub open spec fn timestamps_strictly_increasing_assumption(l: Log) -> bool {
  crate::log::timestamps_strictly_increasing(l)
}

pub open spec fn timestamps_positive_assumption(l: Log) -> bool {
  crate::log::timestamps_positive(l)
}

// WEAK form: the "resource hold" assumption only forbids the
// TASK deregistering/dropping the timer — NOT the timer firing. (The old
// timer_remains_active_between counted a WakeTask as retirement, wrongly
// excluding real firing states from env.)
pub open spec fn timer_remains_active_assumption(l: Log, rid: ResourceIdView) -> bool {
  let register_idx = find_register_timer_idx(l, rid);
  register_idx >= 0 ==>
    crate::invariants::wake_on_expired::timer_not_deregistered_through(
      l, register_idx, l.len() as int)
}

// ============================================================================
// Helper Functions
// ============================================================================

pub open spec fn find_register_timer_idx(l: Log, rid: ResourceIdView) -> int
  decreases l.len()
{
  find_register_timer_idx_from(l, rid, 0)
}

pub open spec fn find_register_timer_idx_from(l: Log, rid: ResourceIdView, start: int) -> int
  decreases l.len() - start
{
  if start >= l.len() {
    -1
  } else if is_succ_register_timer_at(l, start) && get_register_timer_rid(l[start]) == rid {
    start
  } else {
    find_register_timer_idx_from(l, rid, start + 1)
  }
}

// ============================================================================
// Bound Computation
// ============================================================================
//
// The bound is: (deadline - current_max_ts) + 1 rounds
//
// Intuition:
// - Each round has at least one GetCurrentTime (PARK_HAS_TIMESTAMP)
// - Timestamps strictly increase each GetCurrentTime
// - So after (deadline - current) rounds, we reach the deadline
// - Plus 1 round for the WakeTask to be generated (WAKE_ON_EXPIRED)

pub open spec fn compute_timer_bound(deadline: InstantView, current_max_ts: InstantView) -> nat {
  if deadline <= current_max_ts {
    1  // Already expired, just need 1 round for wake
  } else {
    let diff = (deadline - current_max_ts) as nat;
    diff + 1
  }
}

}
