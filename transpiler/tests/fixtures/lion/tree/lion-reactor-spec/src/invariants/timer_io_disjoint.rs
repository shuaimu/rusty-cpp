use vstd::prelude::*;
use crate::log::*;
use crate::events::*;
#[allow(unused_imports)]
use crate::types::*;
use lion_framework_spec::action_safety::*;

verus! {

// R13 (liveness) / R9a+R9b (impl): TIMER_IO_DISJOINT (weakened)
//
// Timer RIDs and IO RIDs are allocated from disjoint namespaces, modulo
// retirement/deregistration.
//
// IO ANCHOR DUALITY (F-K): the io half of the liveness-side records below uses
// the SYSCALL anchor (no_io_syscall_registration_with_rid,
// io_syscall_registered_at); lion-reactor's inlined R9a/R9b use the API anchor
// (no_io_api_with_rid_before, io_api_registered_at).

// Path-compat re-exports.
#[cfg(verus_keep_ghost)]
pub use crate::log::no_io_syscall_registration_with_rid;
#[cfg(verus_keep_ghost)]
pub use crate::log::reveal_no_io_syscall_registration_with_rid;
#[cfg(verus_keep_ghost)]
pub use crate::log::intro_no_io_syscall_registration_with_rid;
#[cfg(verus_keep_ghost)]
pub use crate::log::no_timer_with_rid_before;
#[cfg(verus_keep_ghost)]
pub use crate::log::reveal_no_timer_with_rid_before;
#[cfg(verus_keep_ghost)]
pub use crate::log::intro_no_timer_with_rid_before;

pub open spec fn timer_trigger_fn(l: Log, i: int) -> bool {
  is_succ_register_timer_at(l, i)
}

pub open spec fn timer_validity_fn(l: Log, i: int) -> bool {
  let rid = get_register_timer_rid(l[i]);
  no_io_syscall_registration_with_rid(l, rid, i)
}

pub open spec fn timer_io_disjoint_at_timer() -> ActionSafety<Log> {
  ActionSafety {
    acceptance: |l: Log, i: int| timer_trigger_fn(l, i),
    validity: |l: Log, i: int| timer_validity_fn(l, i),
  }
}

pub open spec fn io_trigger_fn(l: Log, i: int) -> bool {
  io_syscall_registered_at(l, i)
}

pub open spec fn io_validity_fn(l: Log, i: int) -> bool {
  let rid = get_io_syscall_register_rid(l[i]);
  no_timer_with_rid_before(l, rid, i)
}

pub open spec fn timer_io_disjoint_at_io() -> ActionSafety<Log> {
  ActionSafety {
    acceptance: |l: Log, i: int| io_trigger_fn(l, i),
    validity: |l: Log, i: int| io_validity_fn(l, i),
  }
}

}
