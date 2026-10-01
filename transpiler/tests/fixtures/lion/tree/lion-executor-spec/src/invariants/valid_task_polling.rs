use vstd::prelude::*;
use crate::log::*;
use crate::events::*;
use crate::types::*;
use lion_framework_spec::action_safety::*;

verus! {

// VALID_TASK_POLLING: PollTask(tid, result) must satisfy:
//   1. Every polled task was previously injected
//   2. result == Invalid ⟺ tid_is_invalid (returned Ready before OR never injected)
//   Combined with (1), Invalid only fires for already-completed tasks
//
// Runtime justification: task IDs enter the poll queue only via
// PopInjection (= injection) or Drain (= wakeup of previously-injected task)

pub open spec fn tid_was_injected_before(l: Log, i: int, tid: TID) -> bool {
  exists |j: int| #![trigger l[j]]
    0 <= j < i &&
    is_pop_injection_at(l, j) &&
    get_pop_injection_task(l[j]).is_some() &&
    get_pop_injection_task(l[j]).unwrap().id == tid
}

pub open spec fn tid_returned_ready_before(l: Log, i: int, tid: TID) -> bool {
  exists |j: int| #![trigger l[j]]
    0 <= j < i &&
    is_poll_task_at(l, j) &&
    get_poll_task_id(l[j]) == tid &&
    get_poll_result(l[j]) == PollResult::<()>::Ready(())
}

pub open spec fn tid_is_invalid(l: Log, i: int, tid: TID) -> bool {
  tid_returned_ready_before(l, i, tid) || !tid_was_injected_before(l, i, tid)
}

pub open spec fn action_fn(l: Log, i: int) -> bool {
  is_poll_task_at(l, i)
}

pub open spec fn validity_fn(l: Log, i: int) -> bool {
  let tid = get_poll_task_id(l[i]);
  let result = get_poll_result(l[i]);
  tid_was_injected_before(l, i, tid) &&
  (result == PollResult::<()>::Invalid <==> tid_is_invalid(l, i, tid))
}

pub open spec fn valid_task_polling() -> ActionSafety<Log> {
  ActionSafety {
    acceptance: |l: Log, i: int| action_fn(l, i),
    validity: |l: Log, i: int| validity_fn(l, i),
  }
}

}
