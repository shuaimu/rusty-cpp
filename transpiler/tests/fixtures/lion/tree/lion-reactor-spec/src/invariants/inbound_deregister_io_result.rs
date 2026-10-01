use vstd::prelude::*;
use crate::log::*;
use crate::events::*;
use crate::types::*;
use lion_framework_spec::action_safety::*;

verus! {

// R11 (liveness) / R13 (impl): INBOUND_DEREGISTER_IO_RESULT
//
// Inbound DeregisterIoResource::End result must match an Outbound
// DeregisterIoResource in the same cycle (same rid, same result). This is the
// deregister half of the io anchor bridge.

pub open spec fn get_inbound_deregister_io_result(e: ReactorEvent) -> IoResultView<()> {
  match e {
    ReactorEvent::Inbound(InboundCall::DeregisterIoResource { result: Some(r), .. }) => r,
    _ => IoResultView::Err(0),
  }
}

pub open spec fn get_outbound_deregister_io_result(e: ReactorEvent) -> IoResultView<()> {
  match e {
    ReactorEvent::Outbound(OutboundCall::DeregisterIoResource { result, .. }) => result,
    _ => IoResultView::Err(0),
  }
}

pub open spec fn find_deregister_io_cycle_begin(l: Log, end_idx: int) -> int
  decreases end_idx
{
  if end_idx <= 0 {
    -1
  } else if is_inbound_deregister_io_begin_at(l, end_idx - 1) {
    end_idx - 1
  } else {
    find_deregister_io_cycle_begin(l, end_idx - 1)
  }
}

pub open spec fn has_matching_outbound_deregister(
  l: Log,
  begin_idx: int,
  end_idx: int,
  rid: ResourceIdView,
  result: IoResultView<()>,
) -> bool {
  exists |k: int| #![trigger l[k]]
    begin_idx < k < end_idx &&
    io_syscall_deregistered_at(l, k) &&
    get_io_syscall_deregister_rid(l[k]) == rid &&
    get_outbound_deregister_io_result(l[k]) == result
}

pub open spec fn deregister_io_result_valid(l: Log, i: int) -> bool {
  let begin_idx = find_deregister_io_cycle_begin(l, i);
  let rid = get_io_api_deregister_rid(l[i]);
  let result = get_inbound_deregister_io_result(l[i]);
  begin_idx >= 0 &&
  get_io_api_deregister_rid(l[begin_idx]) == rid &&
  has_matching_outbound_deregister(l, begin_idx, i, rid, result)
}

// Path-compat re-export (liveness historically defined this here).
#[cfg(verus_keep_ghost)]
pub use crate::log::is_inbound_deregister_io_end_at;

pub open spec fn trigger_fn(l: Log, i: int) -> bool {
  is_inbound_deregister_io_end_at(l, i)
}

pub open spec fn validity_fn(l: Log, i: int) -> bool {
  deregister_io_result_valid(l, i)
}

pub open spec fn inbound_deregister_io_result() -> ActionSafety<Log> {
  ActionSafety {
    acceptance: |l: Log, i: int| trigger_fn(l, i),
    validity: |l: Log, i: int| validity_fn(l, i),
  }
}

}
