use vstd::prelude::*;
use crate::log::*;
use crate::events::*;
#[allow(unused_imports)]
use crate::types::*;
use lion_framework_spec::action_safety::*;

verus! {

// R8 (liveness) / R3 (impl): PARK_POLL_ONCE
//
// Every park cycle must contain exactly one PollEvents call.

pub open spec fn action_fn(l: Log, i: int) -> bool {
  is_park_end_at(l, i)
}

pub open spec fn count_poll_events_in_range(l: Log, start: int, end: int) -> nat
  decreases end - start
{
  if start >= end {
    0
  } else if is_poll_events_at(l, start) {
    1 + count_poll_events_in_range(l, start + 1, end)
  } else {
    count_poll_events_in_range(l, start + 1, end)
  }
}

pub open spec fn has_exactly_one_poll_events_in_park(l: Log, park_end_idx: int) -> bool {
  let park_start = current_park_start(l, park_end_idx);
  park_start >= 0 &&
  count_poll_events_in_range(l, park_start, park_end_idx) == 1
}

pub open spec fn validity_fn(l: Log, i: int) -> bool {
  has_exactly_one_poll_events_in_park(l, i)
}

pub open spec fn park_poll_once() -> ActionSafety<Log> {
  ActionSafety {
    acceptance: |l: Log, i: int| action_fn(l, i),
    validity: |l: Log, i: int| validity_fn(l, i),
  }
}

}
