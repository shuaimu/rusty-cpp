use vstd::prelude::*;
use crate::log::*;
use crate::events::*;
#[allow(unused_imports)]
use crate::types::*;
use lion_framework_spec::action_safety::*;

verus! {

// R5: TIMER_WAKER_VALIDITY
//
// When a WakeTask is generated for a timer, the waker must match
// the waker from the corresponding RegisterTimer.

/// A WakeTask whose source RID belongs to an active timer (impl-canonical form:
/// includes the timer_active_at conjunct).
pub open spec fn is_timer_wake_at(l: Log, i: int) -> bool {
  is_wake_task_at(l, i) &&
  exists |j: int| 0 <= j < i &&
    is_succ_register_timer_at(l, j) &&
    get_register_timer_rid(l[j]) == get_wake_task_source_rid(l[i]) &&
    timer_active_at(l, j, i)
}

pub open spec fn trigger_fn(l: Log, i: int) -> bool {
  is_timer_wake_at(l, i)
}

pub open spec fn validity_fn(l: Log, i: int) -> bool {
  let rid = get_wake_task_source_rid(l[i]);
  let waker = get_wake_task_waker(l[i]);
  exists |j: int| #![trigger l[j]]
    0 <= j < i &&
    is_succ_register_timer_at(l, j) &&
    get_register_timer_rid(l[j]) == rid &&
    get_register_timer_waker(l[j]) == waker &&
    timer_active_at(l, j, i)
}

pub open spec fn timer_waker_validity() -> ActionSafety<Log> {
  ActionSafety {
    acceptance: |l: Log, i: int| trigger_fn(l, i),
    validity: |l: Log, i: int| validity_fn(l, i),
  }
}

}
