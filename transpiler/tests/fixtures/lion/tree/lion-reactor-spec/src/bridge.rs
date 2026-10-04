use vstd::prelude::*;
use crate::log::*;
use crate::events::*;
#[allow(unused_imports)]
use crate::types::*;
use crate::invariants::inbound_register_io_result::*;
use crate::invariants::inbound_deregister_io_result::*;

verus! {

// ============================================================================
// IO Anchor Bridge (F-K resolution)
// ============================================================================
//
// lion-liveness anchors io registration on the SYSCALL success (Outbound
// RegisterIoResource with Ok(())); lion-reactor anchors it on the API success
// (Inbound RegisterIoResource End with Ok(rid)). The bridge states that the two
// anchors coincide per registration cycle: every successful API registration
// (resp. deregistration) End carries a successful syscall registration (resp.
// deregistration) with the SAME rid inside the enclosing inbound call window
// (cycle begin .. end).
//
// The bridge is DERIVED (not assumed): it follows from the R12/R13
// inbound-result obligations (register_io_result_valid /
// deregister_io_result_valid), which lion-reactor proves against its
// executable code as part of reactor_ext_inv and which lion-liveness carries
// in its reactor_action_safety_inv.

// A successful API deregistration End: Inbound DeregisterIoResource End
// carrying Ok(()).
pub open spec fn io_api_deregistered_ok_at(l: Log, i: int) -> bool {
  is_inbound_deregister_io_end_at(l, i) &&
  get_inbound_deregister_io_result(l[i]) == IoResultView::<()>::Ok(())
}

// Register half: the syscall registration with the same rid sits inside the
// enclosing inbound register cycle.
pub open spec fn io_register_bridge_at(l: Log, i: int) -> bool {
  exists |k: int| #![trigger l[k]]
    find_register_io_cycle_begin(l, i) < k < i &&
    io_syscall_registered_at(l, k) &&
    get_io_syscall_register_rid(l[k]) == get_io_api_register_rid(l[i])
}

// Deregister half: the successful syscall deregistration with the same rid
// sits inside the enclosing inbound deregister cycle.
pub open spec fn io_deregister_bridge_at(l: Log, i: int) -> bool {
  exists |k: int| #![trigger l[k]]
    find_deregister_io_cycle_begin(l, i) < k < i &&
    io_syscall_deregistered_at(l, k) &&
    get_outbound_deregister_io_result(l[k]) == IoResultView::<()>::Ok(()) &&
    get_io_syscall_deregister_rid(l[k]) == get_io_api_deregister_rid(l[i])
}

pub open spec fn io_anchor_bridge(l: Log) -> bool {
  (forall |i: int| #![auto] io_api_registered_at(l, i) ==> io_register_bridge_at(l, i)) &&
  (forall |i: int| #![auto] io_api_deregistered_ok_at(l, i) ==> io_deregister_bridge_at(l, i))
}

// ============================================================================
// Derivation from the R12/R13 obligations
// ============================================================================

#[verifier::rlimit(50)]
pub proof fn register_bridge_from_result_valid(l: Log, i: int)
  requires
    io_api_registered_at(l, i),
    register_io_result_valid(l, i),
  ensures
    io_register_bridge_at(l, i),
{
  let begin_idx = find_register_io_cycle_begin(l, i);
  let source = get_inbound_register_io_source(l[i]);
  let interest = get_inbound_register_io_interest(l[i]);
  let result = get_inbound_register_io_result(l[i]);
  let rid = get_io_api_register_rid(l[i]);
  assert(result == IoResultView::<ResourceIdView>::Ok(rid));
  assert(has_matching_outbound_register(l, begin_idx, i, source, interest, result));
  let k = choose |k: int| #![trigger l[k]]
    begin_idx < k < i &&
    io_syscall_register_at(l, k) &&
    get_outbound_register_io_source(l[k]) == source &&
    get_outbound_register_io_interest(l[k]) == interest &&
    inbound_result_matches_outbound_register(
      result,
      get_outbound_register_io_result(l[k]),
      get_io_syscall_register_rid(l[k]),
    );
  assert(get_outbound_register_io_result(l[k]) is Ok);
  assert(get_io_syscall_register_rid(l[k]) == rid);
  assert(io_syscall_registered_at(l, k));
}

#[verifier::rlimit(50)]
pub proof fn deregister_bridge_from_result_valid(l: Log, i: int)
  requires
    io_api_deregistered_ok_at(l, i),
    deregister_io_result_valid(l, i),
  ensures
    io_deregister_bridge_at(l, i),
{
  let begin_idx = find_deregister_io_cycle_begin(l, i);
  let rid = get_io_api_deregister_rid(l[i]);
  let result = get_inbound_deregister_io_result(l[i]);
  assert(result == IoResultView::<()>::Ok(()));
  assert(has_matching_outbound_deregister(l, begin_idx, i, rid, result));
  let k = choose |k: int| #![trigger l[k]]
    begin_idx < k < i &&
    io_syscall_deregistered_at(l, k) &&
    get_io_syscall_deregister_rid(l[k]) == rid &&
    get_outbound_deregister_io_result(l[k]) == result;
  assert(io_deregister_bridge_at(l, i));
}

// Aggregate form, with hypotheses shaped exactly like lion-reactor's
// reactor_ext_inv R12/R13 clauses (and derivable from lion-liveness's
// reactor_action_safety_inv records).
#[verifier::rlimit(50)]
pub proof fn io_anchor_bridge_from_result_valid(l: Log)
  requires
    forall |i: int| #![auto] io_api_registered_at(l, i) ==> register_io_result_valid(l, i),
    forall |i: int| #![auto] is_inbound_deregister_io_end_at(l, i) ==> deregister_io_result_valid(l, i),
  ensures
    io_anchor_bridge(l),
{
  assert forall |i: int| #![auto] io_api_registered_at(l, i) implies io_register_bridge_at(l, i) by {
    register_bridge_from_result_valid(l, i);
  }
  assert forall |i: int| #![auto] io_api_deregistered_ok_at(l, i) implies io_deregister_bridge_at(l, i) by {
    deregister_bridge_from_result_valid(l, i);
  }
}

}
