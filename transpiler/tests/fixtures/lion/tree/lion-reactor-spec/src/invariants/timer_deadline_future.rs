use vstd::prelude::*;
use crate::log::*;
use crate::events::*;
#[allow(unused_imports)]
use crate::types::*;
use lion_framework_spec::action_safety::*;

verus! {

// R6 (liveness) / R1 (impl): TIMER_DEADLINE_FUTURE
//
// When a timer is successfully registered, its deadline must be strictly
// greater than all previously observed timestamps.

pub open spec fn action_fn(l: Log, i: int) -> bool {
  is_succ_register_timer_at(l, i)
}

// Impl-side name for the obligation body.
pub open spec fn timer_deadline_future_at(l: Log, i: int) -> bool {
  let deadline = get_register_timer_deadline(l[i]);
  let max_ts = max_timestamp_up_to(l, i);
  deadline > max_ts
}

pub open spec fn validity_fn(l: Log, i: int) -> bool {
  let deadline = get_register_timer_deadline(l[i]);
  let max_ts = max_timestamp_up_to(l, i);
  deadline > max_ts
}

pub open spec fn timer_deadline_future() -> ActionSafety<Log> {
  ActionSafety {
    acceptance: |l: Log, i: int| action_fn(l, i),
    validity: |l: Log, i: int| validity_fn(l, i),
  }
}

}
