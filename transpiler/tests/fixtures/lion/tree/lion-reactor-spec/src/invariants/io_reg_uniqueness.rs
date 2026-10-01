use vstd::prelude::*;
use crate::log::*;
use crate::events::*;
#[allow(unused_imports)]
use crate::types::*;
use lion_framework_spec::action_safety::*;

verus! {

// R8: IO_REG_UNIQUENESS (weakened)
//
// Resource IDs for I/O resources are unique modulo deregistration.
//
// IO ANCHOR DUALITY (F-K): the liveness-side record below anchors on the
// SYSCALL registration (Outbound Ok); lion-reactor's inlined R8 clause anchors
// on the API registration (no_prior_io_api_registration, in log). They are
// different propositions, connected by the io anchor bridge.

// Path-compat re-exports (liveness historically defined these here).
#[cfg(verus_keep_ghost)]
pub use crate::log::no_prior_io_syscall_registration;
#[cfg(verus_keep_ghost)]
pub use crate::log::reveal_no_prior_io_syscall_registration;
#[cfg(verus_keep_ghost)]
pub use crate::log::intro_no_prior_io_syscall_registration;

pub open spec fn trigger_fn(l: Log, i: int) -> bool {
  io_syscall_registered_at(l, i)
}

pub open spec fn validity_fn(l: Log, i: int) -> bool {
  let rid = get_io_syscall_register_rid(l[i]);
  no_prior_io_syscall_registration(l, rid, i)
}

pub open spec fn io_reg_uniqueness() -> ActionSafety<Log> {
  ActionSafety {
    acceptance: |l: Log, i: int| trigger_fn(l, i),
    validity: |l: Log, i: int| validity_fn(l, i),
  }
}

}
