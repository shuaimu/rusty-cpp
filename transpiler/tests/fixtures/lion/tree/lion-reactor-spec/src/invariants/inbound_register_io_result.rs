use vstd::prelude::*;
use crate::log::*;
use crate::events::*;
use crate::types::*;
use lion_framework_spec::action_safety::*;

verus! {

// R12: INBOUND_REGISTER_IO_RESULT
//
// Inbound RegisterIoResource::End result must match an Outbound
// RegisterIoResource in the same cycle (same source, interest, and — for Ok —
// the same rid). This is the obligation the io anchor bridge is derived from.

pub open spec fn get_inbound_register_io_source(e: ReactorEvent) -> SourceView {
  match e {
    ReactorEvent::Inbound(InboundCall::RegisterIoResource { source, .. }) => source,
    _ => 0,
  }
}

pub open spec fn get_inbound_register_io_interest(e: ReactorEvent) -> InterestView {
  match e {
    ReactorEvent::Inbound(InboundCall::RegisterIoResource { interest, .. }) => interest,
    _ => (false, false),
  }
}

pub open spec fn get_inbound_register_io_result(e: ReactorEvent) -> IoResultView<ResourceIdView> {
  match e {
    ReactorEvent::Inbound(InboundCall::RegisterIoResource { result: Some(r), .. }) => r,
    _ => IoResultView::Err(0),
  }
}

pub open spec fn get_outbound_register_io_source(e: ReactorEvent) -> SourceView {
  match e {
    ReactorEvent::Outbound(OutboundCall::RegisterIoResource { source, .. }) => source,
    _ => 0,
  }
}

pub open spec fn get_outbound_register_io_interest(e: ReactorEvent) -> InterestView {
  match e {
    ReactorEvent::Outbound(OutboundCall::RegisterIoResource { interest, .. }) => interest,
    _ => (false, false),
  }
}

pub open spec fn get_outbound_register_io_result(e: ReactorEvent) -> IoResultView<()> {
  match e {
    ReactorEvent::Outbound(OutboundCall::RegisterIoResource { result, .. }) => result,
    _ => IoResultView::Err(0),
  }
}

pub open spec fn find_register_io_cycle_begin(l: Log, end_idx: int) -> int
  decreases end_idx
{
  if end_idx <= 0 {
    -1
  } else if is_inbound_register_io_begin_at(l, end_idx - 1) {
    end_idx - 1
  } else {
    find_register_io_cycle_begin(l, end_idx - 1)
  }
}

pub open spec fn inbound_result_matches_outbound_register(
  inbound_result: IoResultView<ResourceIdView>,
  outbound_result: IoResultView<()>,
  outbound_rid: ResourceIdView,
) -> bool {
  match (inbound_result, outbound_result) {
    (IoResultView::Ok(rid), IoResultView::Ok(_)) => rid == outbound_rid,
    (IoResultView::Err(e1), IoResultView::Err(e2)) => e1 == e2,
    _ => false,
  }
}

pub open spec fn has_matching_outbound_register(
  l: Log,
  begin_idx: int,
  end_idx: int,
  source: SourceView,
  interest: InterestView,
  inbound_result: IoResultView<ResourceIdView>,
) -> bool {
  exists |k: int| #![trigger l[k]]
    begin_idx < k < end_idx &&
    io_syscall_register_at(l, k) &&
    get_outbound_register_io_source(l[k]) == source &&
    get_outbound_register_io_interest(l[k]) == interest &&
    inbound_result_matches_outbound_register(
      inbound_result,
      get_outbound_register_io_result(l[k]),
      get_io_syscall_register_rid(l[k]),
    )
}

pub open spec fn register_io_result_valid(l: Log, i: int) -> bool {
  let begin_idx = find_register_io_cycle_begin(l, i);
  let source = get_inbound_register_io_source(l[i]);
  let interest = get_inbound_register_io_interest(l[i]);
  let result = get_inbound_register_io_result(l[i]);
  begin_idx >= 0 &&
  get_inbound_register_io_source(l[begin_idx]) == source &&
  get_inbound_register_io_interest(l[begin_idx]) == interest &&
  has_matching_outbound_register(l, begin_idx, i, source, interest, result)
}

// Path-compat re-export (liveness historically defined this here).
#[cfg(verus_keep_ghost)]
pub use crate::log::is_inbound_register_io_end_at;

pub open spec fn trigger_fn(l: Log, i: int) -> bool {
  is_inbound_register_io_end_at(l, i)
}

pub open spec fn validity_fn(l: Log, i: int) -> bool {
  register_io_result_valid(l, i)
}

pub open spec fn inbound_register_io_result() -> ActionSafety<Log> {
  ActionSafety {
    acceptance: |l: Log, i: int| trigger_fn(l, i),
    validity: |l: Log, i: int| validity_fn(l, i),
  }
}

}
