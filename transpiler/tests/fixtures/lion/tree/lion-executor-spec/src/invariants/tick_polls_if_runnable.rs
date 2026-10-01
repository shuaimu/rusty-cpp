use vstd::prelude::*;
use crate::log::*;
#[cfg(verus_keep_ghost)]
use crate::fifo_queue::fifo_queue_at;
use lion_framework_spec::local_liveness::*;

verus! {

// TICK_POLLS_IF_RUNNABLE: If the FIFO queue is non-empty at Tick::Begin,
// then a PollTask occurs before Tick::End.
//
// Trigger is non-prophetic: only checks queue state at tick begin.

pub open spec fn trigger_fn(l: Log, i: int) -> bool {
  is_tick_begin_at(l, i) && fifo_queue_at(l, i).len() > 0
}

pub open spec fn response_fn(l: Log, i: int, j: int) -> bool {
  is_poll_task_at(l, j)
}

pub open spec fn timely_fn(l: Log, i: int, j: int) -> bool {
  forall |k: int| i < k < j ==> !#[trigger] is_tick_end_at(l, k)
}

pub open spec fn tick_polls_if_runnable() -> LocalLiveness<Log> {
  LocalLiveness {
    acceptance: |l: Log, i: int| trigger_fn(l, i),
    fulfillment: |l: Log, i: int, j: int| response_fn(l, i, j),
    timely: |l: Log, i: int, j: int| timely_fn(l, i, j),
  }
}

}
