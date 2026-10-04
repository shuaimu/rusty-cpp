use vstd::prelude::*;
use crate::log::*;
use crate::events::*;
#[allow(unused_imports)]
use crate::types::*;
use lion_framework_spec::action_safety::*;

verus! {

// R16b (liveness) / R15 (impl): SET_WAKER_ACTIVE_IO
//
// When a SetWaker operation succeeds, the referenced I/O resource must be
// active.
//
// IO ANCHOR DUALITY (F-K): the liveness-side record uses the syscall-anchored
// io_syscall_active_at_set_waker; lion-reactor's inlined R15 uses the
// API-anchored set_waker_on_active_io.

// Impl-side obligation body (API anchor).
pub open spec fn set_waker_on_active_io(l: Log, i: int) -> bool {
  let rid = get_set_waker_rid(l[i]);
  io_api_active_at_set_waker(l, rid, i)
}

pub open spec fn trigger_fn(l: Log, i: int) -> bool {
  is_succ_set_waker_at(l, i)
}

pub open spec fn validity_fn(l: Log, i: int) -> bool {
  let rid = get_set_waker_rid(l[i]);
  io_syscall_active_at_set_waker(l, rid, i)
}

pub open spec fn set_waker_active_io() -> ActionSafety<Log> {
  ActionSafety {
    acceptance: |l: Log, i: int| trigger_fn(l, i),
    validity: |l: Log, i: int| validity_fn(l, i),
  }
}

}
