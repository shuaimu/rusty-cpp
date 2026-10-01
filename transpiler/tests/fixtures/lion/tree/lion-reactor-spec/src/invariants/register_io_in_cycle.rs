use vstd::prelude::*;
use crate::log::*;
use crate::events::*;
#[allow(unused_imports)]
use crate::types::*;
use lion_framework_spec::action_safety::*;

verus! {

// R9 (liveness) / R10 (impl): REGISTER_IO_IN_CYCLE
//
// Outbound RegisterIoResource only occurs within an Inbound RegisterIoResource
// cycle.

pub open spec fn find_last_inbound_register_io_begin(l: Log, before: int) -> int
  decreases before
{
  if before <= 0 {
    -1
  } else if is_inbound_register_io_begin_at(l, before - 1) {
    before - 1
  } else {
    find_last_inbound_register_io_begin(l, before - 1)
  }
}

pub open spec fn no_inbound_register_io_end_between(l: Log, start: int, end: int) -> bool {
  forall |k: int| start < k < end ==>
    !#[trigger] is_inbound_register_io_end_at(l, k)
}

pub open spec fn in_register_io_cycle(l: Log, i: int) -> bool {
  let begin_idx = find_last_inbound_register_io_begin(l, i);
  begin_idx >= 0 && no_inbound_register_io_end_between(l, begin_idx, i)
}

pub open spec fn trigger_fn(l: Log, i: int) -> bool {
  io_syscall_register_at(l, i)  // Outbound RegisterIoResource
}

pub open spec fn validity_fn(l: Log, i: int) -> bool {
  in_register_io_cycle(l, i)
}

pub open spec fn register_io_in_cycle() -> ActionSafety<Log> {
  ActionSafety {
    acceptance: |l: Log, i: int| trigger_fn(l, i),
    validity: |l: Log, i: int| validity_fn(l, i),
  }
}

}
