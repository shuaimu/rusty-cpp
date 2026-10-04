use vstd::prelude::*;
use crate::log::*;
use crate::events::*;
#[allow(unused_imports)]
use crate::types::*;
use lion_framework_spec::async_contract::*;

verus! {

// ============================================================================
// Bounded IO Wakeup Contract (liveness-side; moved unchanged, spelled with the
// explicit io_syscall_* anchor names)
// ============================================================================
//
// Semantics:
// - acceptance: SetWaker(rid, waker) where rid comes from an active RegisterIo
// - fulfillment: WakeTask event is generated with matching rid and waker
// - assumption: IO remains active + IoEventReady will occur within n PollEvents
//
// Bound:
//   n PollEvents (where n is from io_ready_within_n_poll_events assumption)
//   + 0 additional rounds (WAKE_ON_IO_READY guarantees same-cycle wake)
//
// Note: The relationship between n park cycles and n PollEvents is proven
// via PARK_POLL_ONCE (each park cycle has exactly one PollEvents).
//
// ============================================================================

pub open spec fn trigger_fn(l: Log, rid: ResourceIdView) -> bool {
  // The LAST successful SetWaker for rid is one with active IO. Using
  // "last" (rather than "some") aligns with response_fn's definition,
  // so any wake-task whose waker matches the LAST SetWaker satisfies
  // both the contract response and the wake_on_io_ready invariant.
  let sw_idx = find_last_set_waker_for_rid(l, rid, l.len() as int);
  sw_idx >= 0 &&
  io_syscall_active_at_set_waker(l, rid, sw_idx)
}

// Response: a successful SetWaker for `rid` is paired with a WakeTask
// that delivers its waker. This matches what the reactor's
// wake_on_io_ready invariant guarantees per IoEventReady — the waker
// active AT THAT MOMENT is the one delivered. Using "exists matching
// pair" (rather than "the LAST SetWaker's waker") avoids the spurious
// failure mode where a SetWaker added after IoEventReady has no
// matching WakeTask of its own yet.
pub open spec fn response_fn(l: Log, rid: ResourceIdView) -> bool {
  exists |sw_idx: int, wake_i: int| #![trigger l[sw_idx], l[wake_i]]
    0 <= sw_idx < l.len() &&
    is_succ_set_waker_at(l, sw_idx) &&
    get_set_waker_rid(l[sw_idx]) == rid &&
    sw_idx < wake_i < l.len() &&
    is_wake_task_at(l, wake_i) &&
    get_wake_task_source_rid(l[wake_i]) == rid &&
    get_wake_task_waker(l[wake_i]) == get_set_waker_waker(l[sw_idx])
}

// Single-state form (satisfiable). The old `io_assumption` wrapped
// io_remains_active in a forall over ALL log extensions and was constant false
// (an adversarial deregistering extension always exists); it has been removed.
// The readiness-bound content lives in the env forms (`env_reactor_io` here,
// `io_ready_forward_here` at the composed layer), not in this inert field.
pub open spec fn assumption_fn(l: Log, rid: ResourceIdView) -> bool {
  io_remains_active_assumption(l, rid)
}

pub open spec fn bounded_io_wakeup() -> AsyncContract<Log, ResourceIdView> {
  AsyncContract {
    acceptance: |l: Log, rid: ResourceIdView| trigger_fn(l, rid),
    fulfillment: |l: Log, rid: ResourceIdView| response_fn(l, rid),
    assumption: |l: Log, rid: ResourceIdView| assumption_fn(l, rid),
  }
}

// ============================================================================
// Assumption Definitions
// ============================================================================

pub open spec fn count_poll_events_in_range(l: Log, start: int, end: int) -> nat
  decreases end - start
{
  if start >= end || start < 0 || end > l.len() {
    0
  } else if is_poll_events_at(l, start) {
    1 + count_poll_events_in_range(l, start + 1, end)
  } else {
    count_poll_events_in_range(l, start + 1, end)
  }
}

pub open spec fn has_io_event_ready_matching_interest_after(
  l: Log,
  rid: ResourceIdView,
  interest: InterestView,
  start: int,
) -> bool {
  exists |i: int| #![trigger l[i]]
    start <= i < l.len() &&
    is_io_event_ready_at(l, i) &&
    get_io_event(l[i]).resource_id == rid &&
    ((interest.0 && get_io_event(l[i]).readable) ||
     (interest.1 && get_io_event(l[i]).writable))
}

// IO remains active (REUSE-TOLERANT, t5): no deregister of `rid` after the
// registration that is active at the LAST SetWaker for `rid` (the current
// waiter's registration, via find_io_syscall_register_for_rid — the
// registration most recently before that set-waker). Under rid reuse this
// protects ONLY the current waiter's registration; stale
// (deregistered-then-reused) registrations are unconstrained, and it is
// vacuous when no SetWaker for `rid` exists (no waiter).
pub open spec fn io_remains_active_assumption(l: Log, rid: ResourceIdView) -> bool {
  let sw = find_last_set_waker_for_rid(l, rid, l.len() as int);
  let reg = find_io_syscall_register_for_rid(l, rid, sw);
  (sw >= 0 && reg >= 0) ==>
    forall |j: int| reg < j < l.len() ==>
      !(#[trigger] io_syscall_deregistered_at(l, j) && get_io_syscall_deregister_rid(l[j]) == rid)
}

pub open spec fn find_last_set_waker_for_rid(l: Log, rid: ResourceIdView, before: int) -> int
  decreases before
{
  if before <= 0 {
    -1
  } else if is_succ_set_waker_at(l, before - 1) && get_set_waker_rid(l[before - 1]) == rid {
    before - 1
  } else {
    find_last_set_waker_for_rid(l, rid, before - 1)
  }
}

// If any SuccSetWaker for rid exists at index k < before, find_last returns >= 0.
pub proof fn find_last_set_waker_exists_if_some(
  l: Log, rid: ResourceIdView, before: int, k: int,
)
  requires
    0 <= k < before <= l.len(),
    is_succ_set_waker_at(l, k),
    get_set_waker_rid(l[k]) == rid,
  ensures
    find_last_set_waker_for_rid(l, rid, before) >= 0,
    find_last_set_waker_for_rid(l, rid, before) >= k,
    find_last_set_waker_for_rid(l, rid, before) < before,
    is_succ_set_waker_at(l, find_last_set_waker_for_rid(l, rid, before)),
    get_set_waker_rid(l[find_last_set_waker_for_rid(l, rid, before)]) == rid,
  decreases before
{
  if before == k + 1 {
    // Direct hit at before-1 == k
  } else {
    // before > k + 1
    if is_succ_set_waker_at(l, before - 1) && get_set_waker_rid(l[before - 1]) == rid {
      // Found at before-1
    } else {
      find_last_set_waker_exists_if_some(l, rid, before - 1, k);
    }
  }
}

// Path-compat re-exports: these historically lived in this module on the
// liveness side (io_active_at_set_waker / find_register_io_* under the old
// ambiguous names).
#[cfg(verus_keep_ghost)]
pub use crate::log::io_syscall_active_at_set_waker;
#[cfg(verus_keep_ghost)]
pub use crate::invariants::io_waker_validity::find_io_syscall_register_idx;
#[cfg(verus_keep_ghost)]
pub use crate::invariants::io_waker_validity::find_io_syscall_register_idx_from;

// ============================================================================
// Bound Computation
// ============================================================================
//
// For IO, the response must occur in the same park cycle as the IoEventReady.
// Since WAKE_ON_IO_READY guarantees WakeTask before Park::End, the bound is 0.
//
// That is: if IoEventReady has occurred (trigger), WakeTask will be in the
// same log without needing additional progress rounds.

pub open spec fn compute_io_bound() -> nat {
  0  // Response guaranteed in same round as trigger
}

}
