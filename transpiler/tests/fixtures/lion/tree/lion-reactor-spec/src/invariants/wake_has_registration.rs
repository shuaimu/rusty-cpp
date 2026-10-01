use vstd::prelude::*;
use crate::log::*;
use crate::events::*;
#[allow(unused_imports)]
use crate::types::*;
use lion_framework_spec::action_safety::*;

verus! {

// R15 (liveness) / R14 (impl): WAKE_HAS_REGISTRATION
//
// Every WakeTask event must be associated with a prior resource registration
// (timer or IO).
//
// IO ANCHOR DUALITY (F-K): the io branch of the liveness-side record uses the
// SYSCALL anchor; lion-reactor's inlined R14 uses the API anchor
// (validity_fn_api below).

pub open spec fn trigger_fn(l: Log, i: int) -> bool {
  is_wake_task_at(l, i)
}

pub open spec fn validity_fn(l: Log, i: int) -> bool {
  let rid = get_wake_task_source_rid(l[i]);
  (exists |j: int| #![trigger l[j]]
    0 <= j < i &&
    is_succ_register_timer_at(l, j) &&
    get_register_timer_rid(l[j]) == rid)
  ||
  (exists |j: int| #![trigger l[j]]
    0 <= j < i &&
    io_syscall_registered_at(l, j) &&
    get_io_syscall_register_rid(l[j]) == rid)
}

// API-anchored variant (the io branch matches lion-reactor's inlined R14).
pub open spec fn validity_fn_api(l: Log, i: int) -> bool {
  let rid = get_wake_task_source_rid(l[i]);
  (exists |j: int| 0 <= j < i &&
    is_succ_register_timer_at(l, j) &&
    get_register_timer_rid(l[j]) == rid)
  ||
  (exists |j: int| 0 <= j < i &&
    io_api_registered_at(l, j) &&
    get_io_api_register_rid(l[j]) == rid)
}

pub open spec fn wake_has_registration() -> ActionSafety<Log> {
  ActionSafety {
    acceptance: |l: Log, i: int| trigger_fn(l, i),
    validity: |l: Log, i: int| validity_fn(l, i),
  }
}

}
