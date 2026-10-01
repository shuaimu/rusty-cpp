use vstd::prelude::*;
use crate::log::*;
use crate::events::*;
#[allow(unused_imports)]
use crate::types::*;
use lion_framework_spec::action_safety::*;

verus! {

// R6: IO_WAKER_VALIDITY
//
// When a WakeTask is generated for an IO resource, the waker must match the
// waker from a successful SetWaker on an IO resource active at that SetWaker.
//
// IO ANCHOR DUALITY (F-K): the impl anchors "IO wake" on the API registration
// (io_api_*), the liveness proof on the syscall registration (io_syscall_*).
// Both variants are kept side by side.

/// A WakeTask whose source RID belongs to an active IO resource — API anchor
/// (lion-reactor's form).
pub open spec fn is_io_api_wake_at(l: Log, i: int) -> bool {
  is_wake_task_at(l, i) &&
  exists |j: int| 0 <= j < i &&
    io_api_registered_at(l, j) &&
    get_io_api_register_rid(l[j]) == get_wake_task_source_rid(l[i]) &&
    io_api_active_at(l, j, i)
}

/// A WakeTask whose source RID belongs to an active IO resource — syscall
/// anchor (lion-liveness's form).
pub open spec fn is_io_syscall_wake_at(l: Log, i: int) -> bool {
  is_wake_task_at(l, i) && {
    let rid = get_wake_task_source_rid(l[i]);
    exists |j: int| #![trigger l[j]]
      0 <= j < i &&
      io_syscall_registered_at(l, j) &&
      get_io_syscall_register_rid(l[j]) == rid &&
      io_syscall_active_at(l, j, i)
  }
}

pub open spec fn trigger_fn(l: Log, i: int) -> bool {
  is_io_syscall_wake_at(l, i)
}

pub open spec fn validity_fn(l: Log, i: int) -> bool {
  let rid = get_wake_task_source_rid(l[i]);
  let waker = get_wake_task_waker(l[i]);
  exists |sw_idx: int| #![trigger l[sw_idx]]
    0 <= sw_idx < i &&
    is_succ_set_waker_at(l, sw_idx) &&
    get_set_waker_rid(l[sw_idx]) == rid &&
    get_set_waker_waker(l[sw_idx]) == waker &&
    io_syscall_active_at_set_waker(l, rid, sw_idx)
}

// Path-compat re-export: io_syscall_active_at_set_waker historically lived in
// this module on the liveness side.
#[cfg(verus_keep_ghost)]
pub use crate::log::io_syscall_active_at_set_waker;

pub open spec fn find_io_syscall_register_idx(l: Log, rid: ResourceIdView) -> int {
  find_io_syscall_register_idx_from(l, rid, 0)
}

pub open spec fn find_io_syscall_register_idx_from(l: Log, rid: ResourceIdView, start: int) -> int
  decreases l.len() - start
{
  if start >= l.len() {
    -1
  } else if io_syscall_registered_at(l, start) && get_io_syscall_register_rid(l[start]) == rid {
    start
  } else {
    find_io_syscall_register_idx_from(l, rid, start + 1)
  }
}

pub open spec fn io_waker_validity() -> ActionSafety<Log> {
  ActionSafety {
    acceptance: |l: Log, i: int| trigger_fn(l, i),
    validity: |l: Log, i: int| validity_fn(l, i),
  }
}

}
