use vstd::prelude::*;
use crate::log::*;
use crate::events::*;
#[allow(unused_imports)]
use crate::types::*;
use lion_framework_spec::action_safety::*;

verus! {

// R7: TIMER_REG_UNIQUENESS (weakened)
//
// Resource IDs for timers are unique modulo retirement.

// Path-compat re-exports: the obligation body and its companions live in log.
#[cfg(verus_keep_ghost)]
pub use crate::log::no_prior_timer_registration;
#[cfg(verus_keep_ghost)]
pub use crate::log::reveal_no_prior_timer_registration;
#[cfg(verus_keep_ghost)]
pub use crate::log::intro_no_prior_timer_registration;

pub open spec fn trigger_fn(l: Log, i: int) -> bool {
  is_succ_register_timer_at(l, i)
}

pub open spec fn validity_fn(l: Log, i: int) -> bool {
  let rid = get_register_timer_rid(l[i]);
  no_prior_timer_registration(l, rid, i)
}

pub open spec fn timer_reg_uniqueness() -> ActionSafety<Log> {
  ActionSafety {
    acceptance: |l: Log, i: int| trigger_fn(l, i),
    validity: |l: Log, i: int| validity_fn(l, i),
  }
}

}
