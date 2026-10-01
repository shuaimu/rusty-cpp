use vstd::prelude::*;
use crate::log::*;
use lion_framework_spec::action_safety::*;

verus! {

// TICK_HAS_DRAIN_TASK_WAKE: Every Tick::End has a Drain(TaskWake) before it (in same tick)

pub open spec fn action_fn(l: Log, i: int) -> bool {
  is_tick_end_at(l, i)
}

pub open spec fn validity_fn(l: Log, i: int) -> bool {
  exists |d: int| #![auto]
    0 <= d < i && is_drain_task_wake_at(l, d) && no_tick_begin_between(l, d, i)
}

pub open spec fn tick_has_drain_task_wake() -> ActionSafety<Log> {
  ActionSafety {
    acceptance: |l: Log, i: int| action_fn(l, i),
    validity: |l: Log, i: int| validity_fn(l, i),
  }
}

}
