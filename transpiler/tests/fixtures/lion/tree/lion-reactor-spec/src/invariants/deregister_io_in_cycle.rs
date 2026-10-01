use vstd::prelude::*;
use crate::log::*;
use crate::events::*;
#[allow(unused_imports)]
use crate::types::*;
use lion_framework_spec::action_safety::*;

verus! {

// R10 (liveness) / R11 (impl): DEREGISTER_IO_IN_CYCLE
//
// Outbound DeregisterIoResource only occurs within an Inbound
// DeregisterIoResource cycle.

pub open spec fn find_last_inbound_deregister_io_begin(l: Log, before: int) -> int
  decreases before
{
  if before <= 0 {
    -1
  } else if is_inbound_deregister_io_begin_at(l, before - 1) {
    before - 1
  } else {
    find_last_inbound_deregister_io_begin(l, before - 1)
  }
}

pub open spec fn no_inbound_deregister_io_end_between(l: Log, start: int, end: int) -> bool {
  forall |k: int| start < k < end ==>
    !#[trigger] is_inbound_deregister_io_end_at(l, k)
}

pub open spec fn in_deregister_io_cycle(l: Log, i: int) -> bool {
  let begin_idx = find_last_inbound_deregister_io_begin(l, i);
  begin_idx >= 0 && no_inbound_deregister_io_end_between(l, begin_idx, i)
}

pub open spec fn trigger_fn(l: Log, i: int) -> bool {
  io_syscall_deregistered_at(l, i)  // Outbound DeregisterIoResource
}

pub open spec fn validity_fn(l: Log, i: int) -> bool {
  in_deregister_io_cycle(l, i)
}

pub open spec fn deregister_io_in_cycle() -> ActionSafety<Log> {
  ActionSafety {
    acceptance: |l: Log, i: int| trigger_fn(l, i),
    validity: |l: Log, i: int| validity_fn(l, i),
  }
}

}
