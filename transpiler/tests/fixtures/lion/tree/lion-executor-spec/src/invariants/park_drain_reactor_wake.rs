use vstd::prelude::*;
use crate::log::*;
use lion_framework_spec::local_liveness::*;

verus! {

// PARK_DRAIN_REACTOR_WAKE: After Park, reactor-wake drain happens in same tick
// (before Tick::End)

pub open spec fn trigger_fn(l: Log, i: int) -> bool {
  is_park_at(l, i)
}

pub open spec fn response_fn(l: Log, i: int, j: int) -> bool {
  is_drain_reactor_wake_at(l, j)
}

pub open spec fn timely_fn(l: Log, i: int, j: int) -> bool {
  no_tick_end_between(l, i, j)
}

pub open spec fn park_drain_reactor_wake() -> LocalLiveness<Log> {
  LocalLiveness {
    acceptance: |l: Log, i: int| trigger_fn(l, i),
    fulfillment: |l: Log, i: int, j: int| response_fn(l, i, j),
    timely: |l: Log, i: int, j: int| timely_fn(l, i, j),
  }
}

}
