use vstd::prelude::*;
use crate::log::*;
use crate::events::*;
#[allow(unused_imports)]
use crate::types::*;
use lion_framework_spec::action_safety::*;

verus! {

// R14 (liveness) / R4 (impl): IO_READY_IN_PARK
//
// IoEventReady events can only occur within a park cycle.

pub open spec fn action_fn(l: Log, i: int) -> bool {
  is_io_event_ready_at(l, i)
}

pub open spec fn is_in_park_cycle(l: Log, i: int) -> bool {
  let park_start = current_park_start(l, i);
  park_start >= 0
}

pub open spec fn validity_fn(l: Log, i: int) -> bool {
  is_in_park_cycle(l, i)
}

pub open spec fn io_ready_in_park() -> ActionSafety<Log> {
  ActionSafety {
    acceptance: |l: Log, i: int| action_fn(l, i),
    validity: |l: Log, i: int| validity_fn(l, i),
  }
}

}
