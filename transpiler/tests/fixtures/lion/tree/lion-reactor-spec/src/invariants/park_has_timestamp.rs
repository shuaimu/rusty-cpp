use vstd::prelude::*;
use crate::log::*;
use crate::events::*;
#[allow(unused_imports)]
use crate::types::*;
use lion_framework_spec::action_safety::*;

verus! {

// R7 (liveness) / R2 (impl): PARK_HAS_TIMESTAMP
//
// Every park cycle must contain at least one GetCurrentTime call.

pub open spec fn action_fn(l: Log, i: int) -> bool {
  is_park_end_at(l, i)
}

pub open spec fn has_get_current_time_in_park(l: Log, park_end_idx: int) -> bool {
  let park_start = current_park_start(l, park_end_idx);
  park_start >= 0 &&
  exists |j: int| #![trigger l[j]]
    park_start < j < park_end_idx &&
    is_get_current_time_at(l, j)
}

pub open spec fn validity_fn(l: Log, i: int) -> bool {
  has_get_current_time_in_park(l, i)
}

pub open spec fn park_has_timestamp() -> ActionSafety<Log> {
  ActionSafety {
    acceptance: |l: Log, i: int| action_fn(l, i),
    validity: |l: Log, i: int| validity_fn(l, i),
  }
}

}
