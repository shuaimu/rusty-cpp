use vstd::prelude::*;
use crate::log::*;
use crate::events::*;
#[allow(unused_imports)]
use crate::types::*;
use lion_framework_spec::local_liveness::*;

verus! {

// L1 (liveness) / R16 (impl): WAKE_ON_EXPIRED
//
// DEGENERATE / SAFETY-ONLY (ported warning — mirrors lion-reactor R16):
// the acceptance's `timer_awaiting_wake` requires that NO wake for this rid
// occurs after i, which directly contradicts the `exists j > i.
// is_wake_task_at(...)` fulfillment (is_wake_task_at forces j < l.len()). So
// the fulfillment is UNSATISFIABLE whenever the acceptance holds, and this
// invariant collapses to the SAFETY property "no expired timer is ever left
// awaiting a wake". Satisfiable, meaningful, and sound — but the `exists j
// (wake) ... timely` machinery is effectively DEAD: it does NOT assert that
// any wake actually fires. The real liveness content lives in the env-form
// timer path (env_timer_wake_general_at).
// NOTE: timeout_triggers_wake_lemma closes one branch ex falso through this
// contradiction — intentional, but fragile if the definition changes.
//
// BRIDGE — how "an expired timer is woken within the same park cycle" is
// nevertheless a real, verified fact (read this before concluding that prose
// descriptions of wake_on_expired as timely delivery contradict the warning
// above):
//
// 1. Where the timing really comes from. The invariant is only checked at
//    park-cycle boundaries: lion-reactor's park() proves reactor_inv on the
//    final log as a whole-cycle postcondition, and lion-liveness's
//    reactor_progress steps one full park cycle at a time. Mid-cycle logs are
//    never required to satisfy it. A GetCurrentTime >= deadline observed while
//    the timer is active constitutes a timeout point permanently (history is
//    append-only), so at the first boundary after that clock read the collapsed
//    safety form — "no expired timer is still awaiting" — forces a WakeTask (or
//    a DeregisterTimer) to have landed between the clock read and the cycle's
//    end. That is same-cycle delivery, obtained from the checkpoint granularity,
//    NOT from the dead `timely` clause. Corollary: if preservation were ever
//    reproved at per-event granularity, this timing content would silently
//    evaporate while the definition stays verbatim.
//
// 2. The disjunction. The effective content is "expired ==> already woken OR
//    already deregistered". The not-deregistered half is NOT part of this
//    invariant; the composing context supplies it as a side condition
//    (timer_not_deregistered_through below, discharged by the user-task
//    assumption that a task does not abandon a resource it waits on).
//
// 3. How it is consumed. timeout_triggers_wake_lemma (lion-liveness,
//    reactor/proof/timeout_triggers_wake.rs) case-splits on whether a wake for
//    the rid exists at all; in the no-wake branch the antecedent holds, the
//    invariant's unsatisfiable consequent yields the contradiction, and the
//    branch closes — so "a wake exists" is extracted, with no timing. The timer
//    contract's round bound (K) is derived on the env-form clock path
//    (env_timer_wake_general_at / compute_bound), never from this invariant.
//
// RECURSION VS CHOOSE: lion-liveness's timely clause uses the constructive
// first_timeout_point_rec below; lion-reactor's inlined R16 uses the
// choose-based has_timeout_point/first_timeout_point (in log). They coexist
// under distinct names.

pub open spec fn has_timeout_point_at(l: Log, register_idx: int, timeout_idx: int) -> bool
  recommends is_succ_register_timer_at(l, register_idx)
{
  let deadline = get_register_timer_deadline(l[register_idx]);
  is_get_current_time_at(l, timeout_idx) &&
  get_current_timestamp(l[timeout_idx]) >= deadline &&
  timer_active_at(l, register_idx, timeout_idx)
}

pub open spec fn first_timeout_point_rec(l: Log, register_idx: int) -> int
  recommends is_succ_register_timer_at(l, register_idx)
  decreases l.len()
{
  find_first_timeout_point_from(l, register_idx, register_idx + 1)
}

pub open spec fn find_first_timeout_point_from(l: Log, register_idx: int, start: int) -> int
  decreases l.len() - start
{
  if start >= l.len() {
    -1
  } else if has_timeout_point_at(l, register_idx, start) {
    start
  } else {
    find_first_timeout_point_from(l, register_idx, start + 1)
  }
}

pub open spec fn has_first_timeout_point(l: Log, register_idx: int) -> bool
  recommends is_succ_register_timer_at(l, register_idx)
{
  first_timeout_point_rec(l, register_idx) >= 0
}

// Timer still active to end-of-log and no WakeTask for its rid since
// registration (same body as lion-reactor's data_inv::timer_awaiting_wake).
pub open spec fn timer_awaiting_wake(l: Log, register_idx: int) -> bool {
  0 <= register_idx < l.len() &&
  is_succ_register_timer_at(l, register_idx) &&
  timer_active_at(l, register_idx, l.len() as int) &&
  forall |k: int| register_idx < k < l.len() ==> !(
    is_wake_task_at(l, k) &&
    get_wake_task_source_rid(l[k]) == get_register_timer_rid(l[register_idx])
  )
}

// Decoupled "not deregistered through `end`" (does NOT assert not-woken, unlike
// timer_active_at). The composing context supplies this: a task pending on its
// own timer has not deregistered it.
pub open spec fn timer_not_deregistered_through(l: Log, register_idx: int, end: int) -> bool {
  forall |j: int| register_idx < j < end ==> !(
    is_succ_deregister_timer_at(l, j) &&
    get_deregister_timer_rid(l[j]) == get_register_timer_rid(l[register_idx])
  )
}

pub open spec fn trigger_fn(l: Log, i: int) -> bool {
  is_succ_register_timer_at(l, i) &&
  has_first_timeout_point(l, i) &&
  timer_awaiting_wake(l, i)
}

pub open spec fn response_fn(l: Log, trigger_idx: int, j: int) -> bool {
  let rid = get_register_timer_rid(l[trigger_idx]);
  let waker = get_register_timer_waker(l[trigger_idx]);
  is_wake_task_at(l, j) &&
  get_wake_task_source_rid(l[j]) == rid &&
  get_wake_task_waker(l[j]) == waker
}

pub open spec fn timely_fn(l: Log, trigger_idx: int, response_idx: int) -> bool {
  let timeout_idx = first_timeout_point_rec(l, trigger_idx);
  response_idx > timeout_idx &&
  !exists |k: int| #![trigger l[k]]
    timeout_idx < k < response_idx &&
    is_park_end_at(l, k)
}

pub open spec fn wake_on_expired() -> LocalLiveness<Log> {
  LocalLiveness {
    acceptance: |l: Log, i: int| trigger_fn(l, i),
    fulfillment: |l: Log, i: int, j: int| response_fn(l, i, j),
    timely: |l: Log, i: int, j: int| timely_fn(l, i, j),
  }
}

}
