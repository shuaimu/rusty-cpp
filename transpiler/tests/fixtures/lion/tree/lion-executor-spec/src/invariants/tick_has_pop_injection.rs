use vstd::prelude::*;
use crate::log::*;
use lion_framework_spec::action_safety::*;

verus! {

// TICK_HAS_POP_INJECTION: Every Tick::End has a PopInjection in same tick

pub open spec fn action_fn(l: Log, i: int) -> bool {
  is_tick_end_at(l, i)
}

pub open spec fn validity_fn(l: Log, i: int) -> bool {
  exists |p: int| #![auto]
    0 <= p < i && is_pop_injection_at(l, p) && no_tick_begin_between(l, p, i)
}

pub open spec fn tick_has_pop_injection() -> ActionSafety<Log> {
  ActionSafety {
    acceptance: |l: Log, i: int| action_fn(l, i),
    validity: |l: Log, i: int| validity_fn(l, i),
  }
}

}
