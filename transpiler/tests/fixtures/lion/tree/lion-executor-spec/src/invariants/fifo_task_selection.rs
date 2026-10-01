use vstd::prelude::*;
use crate::log::*;
use crate::events::*;
use lion_framework_spec::action_safety::*;

// The FIFO queue model itself lives in crate::fifo_queue; re-exported here so
// consumers can keep addressing it through this invariant's module path.
// (verus_keep_ghost-gated: these are ghost items, absent from plain cargo builds.)
#[cfg(verus_keep_ghost)]
#[allow(unused_imports)]
pub use crate::fifo_queue::{remove_first_occurrence, fifo_queue_at, is_fifo_head_at,
  fifo_queue_matches, remove_other_preserves_member};

verus! {

// FIFO_TASK_SELECTION: Every PollTask(tid) must select the next task in FIFO order
// from all queues (Injection, ReactorWake, TaskWake, Deferred).
//
// The FIFO order is determined by tracking:
// 1. Tasks entering via PopInjection(Some(task))
// 2. Tasks entering via Drain(source, tids)
//
// When PollTask(tid) occurs, tid must be the head of the accumulated FIFO queue.

pub open spec fn action_fn(l: Log, i: int) -> bool {
  is_poll_task_at(l, i)
}

pub open spec fn validity_fn(l: Log, i: int) -> bool {
  let tid = get_poll_task_id(l[i]);
  is_fifo_head_at(l, i, tid)
}

pub open spec fn fifo_task_selection() -> ActionSafety<Log> {
  ActionSafety {
    acceptance: |l: Log, i: int| action_fn(l, i),
    validity: |l: Log, i: int| validity_fn(l, i),
  }
}

}
