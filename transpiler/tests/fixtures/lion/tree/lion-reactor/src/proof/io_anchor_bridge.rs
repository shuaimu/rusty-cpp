use vstd::prelude::*;
#[cfg(verus_keep_ghost)]
use crate::spec::log::*;
#[cfg(verus_keep_ghost)]
use crate::spec::predicates::*;
#[cfg(verus_keep_ghost)]
use crate::invariants::reactor_ext_inv;
#[cfg(verus_keep_ghost)]
use lion_reactor_spec::bridge::*;

verus! {

// F-K resolution: the io anchor bridge — every successful API (Inbound)
// registration/deregistration End carries a successful syscall (Outbound)
// registration/deregistration with the SAME rid inside the enclosing inbound
// call window. This is NOT a new trusted clause: it is a proven corollary of
// the R12/R13 inbound-result obligations of reactor_ext_inv, which this crate
// proves against its executable code (preservation_ext.rs). Wherever
// reactor_ext_inv holds of the real reactor's ghost log, the bridge holds.
#[verifier::rlimit(50)]
pub proof fn reactor_ext_inv_implies_io_anchor_bridge(l: Log)
  requires
    reactor_ext_inv(l),
  ensures
    io_anchor_bridge(l),
{
  assert forall |i: int| #![auto] io_api_registered_at(l, i) implies
    crate::invariants::inbound_register_io_result::register_io_result_valid(l, i)
  by {}
  assert forall |i: int| #![auto] is_inbound_deregister_io_end_at(l, i) implies
    crate::invariants::inbound_deregister_io_result::deregister_io_result_valid(l, i)
  by {}
  io_anchor_bridge_from_result_valid(l);
}

}
