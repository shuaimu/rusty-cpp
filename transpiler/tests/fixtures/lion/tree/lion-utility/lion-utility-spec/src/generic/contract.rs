use vstd::prelude::*;
use crate::generic::types::*;
use crate::generic::events::*;
use crate::generic::log::*;
use crate::framework::async_contract::*;

verus! {

// ============================================================================
// PassWaker -> WakeWaker contract template (the paper's §5.1 utility contract).
//
// Used by cross-task signaling utilities (Notify, Mutex, Channel, Semaphore, …)
// whose wake mechanism lives inside the utility's own state (waker queue, lock
// waiters, channel buffer) and is fired by another task's method call
// (notify_one, guard drop, send, …).
//
// Reactor-mediated utilities (Sleep, TcpStream, …) do NOT instantiate this —
// their wake chain runs through the reactor's bounded_{io,timer}_wakeup
// contract directly.
//
// Uses the `bounded_liveness_without_arrival` shape (our framework's
// "from acceptance" form): once `acceptance` (a PassWaker is registered) and
// the utility-specific `assumption` hold, `response` (a WakeWaker fires)
// follows within bounded progress.
//
// Field-name mapping to our `AsyncContract`:
//   paper "acceptance"  -> our `trigger`
//   paper "fulfillment" -> our `response`
//
// `l_start`-anchored (Option B): `response` requires a WakeWaker NEW since
// `l_start`, so the contract cannot be discharged trivially with n = 0 by a
// stale WakeWaker already in the prefix.
//
// Type parameters: M = method tag, R = return value type, W = waker type
// (instantiated as `WakerView`).
// ============================================================================

// Acceptance: a PassWaker for `w` exists anywhere in `l` — a state predicate
// marking that the utility is waiting for `w`. Past PassWakers are valid
// acceptance witnesses for re-firing the contract.
pub open spec fn acceptance_fn<M, R>(l: Log<M, R>, w: WakerView) -> bool {
  exists |i: int| #![trigger is_pass_waker_at(l, i)]
    is_pass_waker_at(l, i) && get_pass_waker_waker(l[i]) == w
}

// Fulfillment (position-relative): a WakeWaker for `w` at index >= l_start.len()
// — i.e. a NEW WakeWaker since the starting state.
pub open spec fn new_wake_waker_since<M, R>(l_start: Log<M, R>, l: Log<M, R>, w: WakerView) -> bool {
  exists |j: int| #![trigger is_wake_waker_at(l, j)]
    l_start.len() as int <= j < l.len() as int &&
    is_wake_waker_at(l, j) &&
    get_wake_waker_waker(l[j]) == w
}

pub open spec fn passwaker_to_wakewaker_contract<M, R>(
  l_start: Log<M, R>,
  utility_assumption: spec_fn(Log<M, R>, WakerView) -> bool,
) -> AsyncContract<Log<M, R>, WakerView> {
  AsyncContract {
    acceptance:    |l: Log<M, R>, w: WakerView| acceptance_fn(l, w),
    fulfillment:   |l: Log<M, R>, w: WakerView| new_wake_waker_since(l_start, l, w),
    assumption: utility_assumption,
  }
}

// The bounded-liveness obligation a concrete utility instance discharges:
//   bounded_liveness_without_arrival(utility_module_spec(inv),
//                                    passwaker_to_wakewaker_contract(l_start, assumption))
// stated via the framework's `bounded_liveness_without_arrival` in
// utilities/generic/contract usage sites and per-utility crates.

}
