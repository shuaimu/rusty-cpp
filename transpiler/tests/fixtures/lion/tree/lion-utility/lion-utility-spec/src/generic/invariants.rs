use vstd::prelude::*;
use crate::generic::events::*;
use crate::generic::log::*;
use crate::framework::action_safety::*;

verus! {

// ============================================================================
// wakeup_guarantee<M, R>() — Pending ticks must arm a wake source
//
// Whenever a tick ends Pending, within that same tick cycle the task must have
// armed at least one wake source BOUND TO THAT TICK'S WAKER. There are four
// legal source kinds (`active_wakeup_source_for`): PassWaker (cross-task
// signalling utilities), Defer (self-reschedule), RegisterTimer (reactor-timer
// utilities — Sleep), or SetIoWaker on a still-active io resource (reactor-io
// utilities — Tcp). This is the universal utility-side guarantee that a
// suspended task always has a pending wake source — the foundation of the
// composing layer's B1, and it applies to reactor-mediated and self-contained
// utilities alike.
//
// (Generalized from the former PassWaker-only form to the waker-typed four-way,
// matching — and strengthening with explicit waker matching — the monotype
// `has_active_wakeup_source`.)
// ============================================================================
pub open spec fn wakeup_acceptance<M, R>(l: Log<M, R>, i: int) -> bool {
  is_tick_end_pending_at(l, i)
}

pub open spec fn wakeup_validity<M, R>(l: Log<M, R>, i: int) -> bool {
  let waker = get_tick_waker(l[i]);
  exists |b: int| #![trigger complete_tick_cycle(l, b, i)]
    complete_tick_cycle(l, b, i) &&
    active_wakeup_source_for(l, waker, b, i)
}

pub open spec fn wakeup_guarantee<M, R>() -> ActionSafety<Log<M, R>> {
  ActionSafety {
    acceptance: |l: Log<M, R>, i: int| wakeup_acceptance(l, i),
    validity: |l: Log<M, R>, i: int| wakeup_validity(l, i),
  }
}

// ============================================================================
// resource_ownership<M, R>() — resource ops must target a still-ACTIVE resource
//
// Any operation on a resource token (DeregisterTimer / DeregisterIo /
// SetIoWaker) must target a token that is active at that point: successfully
// Register*ed earlier with no intervening Deregister of the same token. This is
// uniform across all three ops (no double-deregister, no SetIoWaker on a closed
// resource) and is correct under token re-registration. Pure log-internal
// ordering; cross-tick allowed.
//
// (Strengthened from the former "ever registered" (owned-only) form to the
// "still active" form, matching — and made consistent across ops, unlike — the
// monotype resource_ownership which only required `active` for timer-dereg and
// set-waker.)
// ============================================================================
pub open spec fn resource_acceptance<M, R>(l: Log<M, R>, i: int) -> bool {
  is_deregister_timer_at(l, i) ||
  is_deregister_io_at(l, i) ||
  is_set_io_waker_at(l, i)
}

pub open spec fn resource_validity<M, R>(l: Log<M, R>, i: int) -> bool {
  let rt = get_resource_token(l[i]);
  token_active_before(l, rt, i)
}

pub open spec fn resource_ownership<M, R>() -> ActionSafety<Log<M, R>> {
  ActionSafety {
    acceptance: |l: Log<M, R>, i: int| resource_acceptance(l, i),
    validity: |l: Log<M, R>, i: int| resource_validity(l, i),
  }
}

// ============================================================================
// Aggregator — default well_formed predicate of utility_module_spec<M, R>.
// A concrete utility may strengthen by conjoining its own invariants.
// ============================================================================
pub open spec fn utility_inv<M, R>(l: Log<M, R>) -> bool {
  action_safety_satisfied(wakeup_guarantee::<M, R>(), l) &&
  action_safety_satisfied(resource_ownership::<M, R>(), l)
}

}
