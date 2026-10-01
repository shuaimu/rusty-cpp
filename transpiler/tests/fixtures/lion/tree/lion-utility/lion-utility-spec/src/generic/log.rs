use vstd::prelude::*;
use crate::generic::events::*;
use crate::generic::types::{ResourceIdView, WakerView};

verus! {

pub type Log<M, R> = Seq<UtilityEvent<M, R>>;

// ============================================================================
// Atomic event predicates (lift utility-event predicates to log indices)
// ============================================================================

pub open spec fn is_tick_at<M, R>(l: Log<M, R>, i: int) -> bool {
  0 <= i < l.len() && is_tick(l[i])
}

pub open spec fn is_tick_begin_at<M, R>(l: Log<M, R>, i: int) -> bool {
  0 <= i < l.len() && is_tick_begin(l[i])
}

pub open spec fn is_tick_end_at<M, R>(l: Log<M, R>, i: int) -> bool {
  0 <= i < l.len() && is_tick_end(l[i])
}

pub open spec fn is_tick_end_pending_at<M, R>(l: Log<M, R>, i: int) -> bool {
  0 <= i < l.len() && is_tick_end_pending(l[i])
}

pub open spec fn is_tick_end_finished_at<M, R>(l: Log<M, R>, i: int) -> bool {
  0 <= i < l.len() && is_tick_end_finished(l[i])
}

pub open spec fn is_pass_waker_at<M, R>(l: Log<M, R>, i: int) -> bool {
  0 <= i < l.len() && is_pass_waker(l[i])
}

pub open spec fn is_wake_waker_at<M, R>(l: Log<M, R>, i: int) -> bool {
  0 <= i < l.len() && is_wake_waker(l[i])
}

pub open spec fn is_cancel_waker_at<M, R>(l: Log<M, R>, i: int) -> bool {
  0 <= i < l.len() && is_cancel_waker(l[i])
}

pub open spec fn is_register_timer_succ_at<M, R>(l: Log<M, R>, i: int) -> bool {
  0 <= i < l.len() && is_register_timer_succ(l[i])
}

pub open spec fn is_register_io_succ_at<M, R>(l: Log<M, R>, i: int) -> bool {
  0 <= i < l.len() && is_register_io_succ(l[i])
}

pub open spec fn is_deregister_timer_at<M, R>(l: Log<M, R>, i: int) -> bool {
  0 <= i < l.len() && is_deregister_timer(l[i])
}

pub open spec fn is_deregister_io_at<M, R>(l: Log<M, R>, i: int) -> bool {
  0 <= i < l.len() && is_deregister_io(l[i])
}

pub open spec fn is_set_io_waker_at<M, R>(l: Log<M, R>, i: int) -> bool {
  0 <= i < l.len() && is_set_io_waker(l[i])
}

pub open spec fn is_defer_at<M, R>(l: Log<M, R>, i: int) -> bool {
  0 <= i < l.len() && is_defer(l[i])
}

// A successful Register* (timer or io) of token `rt` at index i.
pub open spec fn is_register_succ_of_token_at<M, R>(l: Log<M, R>, i: int, rt: ResourceIdView) -> bool {
  0 <= i < l.len() && (
    (is_register_timer_succ(l[i]) && get_register_timer_token(l[i]) == rt) ||
    (is_register_io_succ(l[i]) && get_register_io_token(l[i]) == rt)
  )
}

// ============================================================================
// Tick cycle helpers
// ============================================================================

// A complete tick cycle bracketed by a Begin at `b` and an End at `e`, with no
// other Begin / End events in between (no nesting / overlap).
pub open spec fn complete_tick_cycle<M, R>(l: Log<M, R>, b: int, e: int) -> bool {
  is_tick_begin_at(l, b) &&
  is_tick_end_at(l, e) &&
  b < e &&
  (forall |k: int| b < k < e ==>
     !#[trigger] is_tick_begin_at(l, k) && !is_tick_end_at(l, k))
}

// ============================================================================
// Active wakeup-source helpers (waker-typed) — used by wakeup_guarantee<M,R>.
//
// A Pending tick must, within its tick cycle (b, i), arm a wake source bound to
// THAT tick's waker. Four legal source kinds — the four-way of the monotype
// `has_active_wakeup_source`, but strengthened here with explicit waker
// matching now that events carry `WakerView` (monotype events were waker-blind):
//   PassWaker(w) | Defer | RegisterTimer(w) (re-armed, not later deregistered)
//   | SetIoWaker(w) on a still-active io resource.
// ============================================================================

// A DeregisterTimer for token `rt` strictly between `reg_idx` and `end`.
pub open spec fn timer_deregistered_after_in_cycle<M, R>(l: Log<M, R>, rt: ResourceIdView, reg_idx: int, end: int) -> bool {
  exists |k: int| #![trigger is_deregister_timer_at(l, k)]
    reg_idx < k < end &&
    is_deregister_timer_at(l, k) &&
    get_resource_token(l[k]) == rt
}

// io resource `rid` was successfully registered before `i` and not deregistered
// before `i` (registration persists across polls; only the waker is re-armed).
pub open spec fn io_active_at<M, R>(l: Log<M, R>, rid: ResourceIdView, i: int) -> bool {
  (exists |j: int| #![trigger is_register_io_succ_at(l, j)]
     0 <= j < i && is_register_io_succ_at(l, j) && get_register_io_token(l[j]) == rid) &&
  !(exists |k: int| #![trigger is_deregister_io_at(l, k)]
     0 <= k < i && is_deregister_io_at(l, k) && get_resource_token(l[k]) == rid)
}

// (a) cross-task signal source: a PassWaker for `w` in the cycle.
pub open spec fn passwaker_armed_in_cycle<M, R>(l: Log<M, R>, w: WakerView, b: int, i: int) -> bool {
  exists |j: int| #![trigger is_pass_waker_at(l, j)]
    b < j < i && is_pass_waker_at(l, j) && get_pass_waker_waker(l[j]) == w
}

// (b) self-reschedule source: a Defer in the cycle.
pub open spec fn defer_in_cycle<M, R>(l: Log<M, R>, b: int, i: int) -> bool {
  exists |j: int| #![trigger is_defer_at(l, j)]
    b < j < i && is_defer_at(l, j)
}

// (c) timer source: a successful RegisterTimer for `w` in the cycle, not
// deregistered again before the tick end (i.e. still armed at i).
pub open spec fn timer_armed_for_in_cycle<M, R>(l: Log<M, R>, w: WakerView, b: int, i: int) -> bool {
  exists |j: int| #![trigger is_register_timer_succ_at(l, j)]
    b < j < i &&
    is_register_timer_succ_at(l, j) &&
    get_register_timer_waker(l[j]) == w &&
    !timer_deregistered_after_in_cycle(l, get_register_timer_token(l[j]), j, i)
}

// (d) io source: a still-active io resource whose waker was set to `w` in the
// cycle (registration may predate the cycle; the waker re-arm is in-cycle).
pub open spec fn io_armed_for_in_cycle<M, R>(l: Log<M, R>, w: WakerView, b: int, i: int) -> bool {
  exists |rid: ResourceIdView| #![trigger io_active_at(l, rid, i)]
    io_active_at(l, rid, i) &&
    (exists |j: int| #![trigger is_set_io_waker_at(l, j)]
       b < j < i &&
       is_set_io_waker_at(l, j) &&
       get_resource_token(l[j]) == rid &&
       get_set_io_waker_waker(l[j]) == w)
}

// The four-way wakeup-source disjunction, all bound to waker `w`.
pub open spec fn active_wakeup_source_for<M, R>(l: Log<M, R>, w: WakerView, b: int, i: int) -> bool {
  passwaker_armed_in_cycle(l, w, b, i) ||
  defer_in_cycle(l, b, i) ||
  timer_armed_for_in_cycle(l, w, b, i) ||
  io_armed_for_in_cycle(l, w, b, i)
}

// ============================================================================
// Resource liveness/ownership helpers — used by resource_ownership<M,R>.
// ============================================================================

// A DeregisterTimer or DeregisterIo for token `rt` at index `k`.
pub open spec fn is_dereg_of_token_at<M, R>(l: Log<M, R>, k: int, rt: ResourceIdView) -> bool {
  0 <= k < l.len() &&
  (is_deregister_timer(l[k]) || is_deregister_io(l[k])) &&
  get_resource_token(l[k]) == rt
}

// Token `rt` is active just before index `i`: some successful Register* of `rt`
// occurs at j < i with NO Deregister of `rt` in (j, i). Correct under token
// re-registration — only the latest register-without-a-later-deregister counts,
// so a re-registered token reads active again (unlike a naive "ever registered
// and never deregistered" check).
pub open spec fn token_active_before<M, R>(l: Log<M, R>, rt: ResourceIdView, i: int) -> bool {
  exists |j: int| #![trigger is_register_succ_of_token_at(l, j, rt)]
    0 <= j < i &&
    is_register_succ_of_token_at(l, j, rt) &&
    !(exists |k: int| #![trigger is_dereg_of_token_at(l, k, rt)]
        j < k < i && is_dereg_of_token_at(l, k, rt))
}

}
