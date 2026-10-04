use vstd::prelude::*;
use crate::log::*;
use lion_framework_spec::action_safety::*;

verus! {

// POLL_WITHIN_TICK: Every PollTask has a preceding tick_begin (polls only
// happen within ticks). Impl-side invariant; lion-liveness does not consume it.

pub open spec fn action_fn(l: Log, i: int) -> bool {
  is_poll_task_at(l, i)
}

pub open spec fn validity_fn(l: Log, i: int) -> bool {
  exists |tb: int| #![auto]
    0 <= tb < i && is_tick_begin_at(l, tb) && no_tick_begin_between(l, tb, i)
}

pub open spec fn poll_within_tick() -> ActionSafety<Log> {
  ActionSafety {
    acceptance: |l: Log, i: int| action_fn(l, i),
    validity: |l: Log, i: int| validity_fn(l, i),
  }
}

}
