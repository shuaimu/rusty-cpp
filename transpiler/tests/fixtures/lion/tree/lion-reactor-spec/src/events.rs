use vstd::prelude::*;
use crate::types::*;

verus! {

// ============================================================================
// Inbound Events (External -> Reactor)
// ============================================================================
//
// Inbound events have Begin/End phases to track control flow boundaries.
// End phase marks when reactor returns control, enabling verification that
// required callbacks complete before losing control.

#[derive(PartialEq, Eq)]
#[allow(inconsistent_fields)]
pub enum InboundCall {
  // I/O Resource Management
  RegisterIoResource {
    source: SourceView,
    interest: InterestView,
    result: Option<IoResultView<ResourceIdView>>,  // None=Begin, Some=End
  },
  DeregisterIoResource {
    resource_id: ResourceIdView,
    result: Option<IoResultView<()>>,
  },

  // Waker Management
  SetWaker {
    resource_id: ResourceIdView,
    interest: InterestView,
    waker: WakerView,
    result: Option<IoResultView<()>>,  // None=Begin, Some=End
  },

  // Timer Management
  RegisterTimer {
    deadline: InstantView,
    waker: WakerView,
    result: Option<IoResultView<ResourceIdView>>,
  },
  DeregisterTimer {
    resource_id: ResourceIdView,
    result: bool,
  },

  // Event Loop Control
  Park {
    timeout: Option<DurationView>,
    result: Option<IoResultView<()>>,
  },
}

// ============================================================================
// Outbound Events (Reactor -> External)
// ============================================================================
//
// Outbound events are atomic actions the reactor performs.
// No phase tracking needed since reactor controls when these occur.

#[derive(PartialEq, Eq)]
#[allow(inconsistent_fields)]
pub enum OutboundCall {
  // I/O Resource Management (OS syscalls)
  RegisterIoResource {
    source: SourceView,
    resource_id: ResourceIdView,
    interest: InterestView,
    result: IoResultView<()>,
  },
  DeregisterIoResource {
    source: SourceView,
    resource_id: ResourceIdView,
    result: IoResultView<()>,
  },

  // I/O Polling (OS syscalls)
  PollEvents {
    timeout: Option<DurationView>,
    result: IoResultView<nat>,  // Ok(n) = n events ready
  },

  // Individual IO event ready (follows PollEvents)
  IoEventReady {
    event: IoEventView,
  },

  // Time Query
  GetCurrentTime {
    timestamp: InstantView,
  },

  // Task Wake (callback to executor)
  WakeTask {
    waker: WakerView,
    source_rid: ResourceIdView,  // which timer/io resource triggered this wake
  },
}

// ============================================================================
// Log Event
// ============================================================================

#[derive(PartialEq, Eq)]
pub enum ReactorEvent {
  Inbound(InboundCall),
  Outbound(OutboundCall),
}

// ============================================================================
// Event Predicates
// ============================================================================

// Park predicates
pub open spec fn is_park_begin(e: ReactorEvent) -> bool {
  matches!(e, ReactorEvent::Inbound(InboundCall::Park { result: None, .. }))
}

pub open spec fn is_park_end(e: ReactorEvent) -> bool {
  matches!(e, ReactorEvent::Inbound(InboundCall::Park { result: Some(_), .. }))
}

// Timer predicates
pub open spec fn is_register_timer(e: ReactorEvent) -> bool {
  matches!(e, ReactorEvent::Inbound(InboundCall::RegisterTimer { .. }))
}

pub open spec fn is_succ_register_timer(e: ReactorEvent) -> bool {
  match e {
    ReactorEvent::Inbound(InboundCall::RegisterTimer { result: Some(IoResultView::Ok(_)), .. }) => true,
    _ => false,
  }
}

pub open spec fn is_deregister_timer(e: ReactorEvent) -> bool {
  matches!(e, ReactorEvent::Inbound(InboundCall::DeregisterTimer { .. }))
}

pub open spec fn is_succ_deregister_timer(e: ReactorEvent) -> bool {
  match e {
    ReactorEvent::Inbound(InboundCall::DeregisterTimer { result: true, .. }) => true,
    _ => false,
  }
}

pub proof fn succ_deregister_timer_is_deregister_timer(e: ReactorEvent)
  requires is_succ_deregister_timer(e),
  ensures is_deregister_timer(e),
{}

// ============================================================================
// IO predicates — SYSCALL anchor (Outbound events; lion-liveness convention)
// ============================================================================

pub open spec fn is_io_syscall_register(e: ReactorEvent) -> bool {
  matches!(e, ReactorEvent::Outbound(OutboundCall::RegisterIoResource { .. }))
}

pub open spec fn is_succ_io_syscall_register(e: ReactorEvent) -> bool {
  match e {
    ReactorEvent::Outbound(OutboundCall::RegisterIoResource { result: IoResultView::Ok(_), .. }) => true,
    _ => false,
  }
}

// Any Outbound DeregisterIoResource (any result).
pub open spec fn is_io_syscall_deregister(e: ReactorEvent) -> bool {
  matches!(e, ReactorEvent::Outbound(OutboundCall::DeregisterIoResource { .. }))
}

// ============================================================================
// IO predicates — API anchor (Inbound events; lion-reactor convention)
// ============================================================================

// Inbound RegisterIoResource End with Ok(rid): the API call succeeded.
pub open spec fn is_succ_io_api_register(e: ReactorEvent) -> bool {
  match e {
    ReactorEvent::Inbound(InboundCall::RegisterIoResource { result: Some(IoResultView::Ok(_)), .. }) => true,
    _ => false,
  }
}

// Any Inbound DeregisterIoResource event (Begin or End, any result): the
// impl-side retirement marker for io resources.
pub open spec fn is_io_api_deregister(e: ReactorEvent) -> bool {
  matches!(e, ReactorEvent::Inbound(InboundCall::DeregisterIoResource { .. }))
}

// SetWaker predicates
pub open spec fn is_set_waker(e: ReactorEvent) -> bool {
  matches!(e, ReactorEvent::Inbound(InboundCall::SetWaker { .. }))
}

pub open spec fn is_succ_set_waker(e: ReactorEvent) -> bool {
  match e {
    ReactorEvent::Inbound(InboundCall::SetWaker { result: Some(IoResultView::Ok(_)), .. }) => true,
    _ => false,
  }
}

// Inbound RegisterIoResource predicates (for cycle tracking)
pub open spec fn is_inbound_register_io_begin(e: ReactorEvent) -> bool {
  matches!(e, ReactorEvent::Inbound(InboundCall::RegisterIoResource { result: None, .. }))
}

pub open spec fn is_inbound_register_io_end(e: ReactorEvent) -> bool {
  matches!(e, ReactorEvent::Inbound(InboundCall::RegisterIoResource { result: Some(_), .. }))
}

// Inbound DeregisterIoResource predicates (for cycle tracking)
pub open spec fn is_inbound_deregister_io_begin(e: ReactorEvent) -> bool {
  matches!(e, ReactorEvent::Inbound(InboundCall::DeregisterIoResource { result: None, .. }))
}

pub open spec fn is_inbound_deregister_io_end(e: ReactorEvent) -> bool {
  matches!(e, ReactorEvent::Inbound(InboundCall::DeregisterIoResource { result: Some(_), .. }))
}

// Outbound predicates
pub open spec fn is_poll_events(e: ReactorEvent) -> bool {
  matches!(e, ReactorEvent::Outbound(OutboundCall::PollEvents { .. }))
}

pub open spec fn is_io_event_ready(e: ReactorEvent) -> bool {
  matches!(e, ReactorEvent::Outbound(OutboundCall::IoEventReady { .. }))
}

pub open spec fn is_get_current_time(e: ReactorEvent) -> bool {
  matches!(e, ReactorEvent::Outbound(OutboundCall::GetCurrentTime { .. }))
}

pub open spec fn is_wake_task(e: ReactorEvent) -> bool {
  matches!(e, ReactorEvent::Outbound(OutboundCall::WakeTask { .. }))
}

pub open spec fn is_inbound_non_park(e: ReactorEvent) -> bool {
  match e {
    ReactorEvent::Inbound(InboundCall::Park { .. }) => false,
    ReactorEvent::Inbound(_) => true,
    ReactorEvent::Outbound(_) => false,
  }
}

// ============================================================================
// Accessors (off-domain fallbacks are arbitrary(): an unspecified value can
// never be proven equal to anything, so a wrong-typed event cannot spuriously
// match a guard like get_*_rid(e) == rid — rid 0 is a valid nat)
// ============================================================================

pub open spec fn get_register_timer_deadline(e: ReactorEvent) -> InstantView {
  match e {
    ReactorEvent::Inbound(InboundCall::RegisterTimer { deadline, .. }) => deadline,
    _ => arbitrary(),
  }
}

pub open spec fn get_register_timer_waker(e: ReactorEvent) -> WakerView {
  match e {
    ReactorEvent::Inbound(InboundCall::RegisterTimer { waker, .. }) => waker,
    _ => arbitrary(),
  }
}

pub open spec fn get_register_timer_rid(e: ReactorEvent) -> ResourceIdView {
  match e {
    ReactorEvent::Inbound(InboundCall::RegisterTimer { result: Some(IoResultView::Ok(rid)), .. }) => rid,
    _ => arbitrary(),
  }
}

pub open spec fn get_deregister_timer_rid(e: ReactorEvent) -> ResourceIdView {
  match e {
    ReactorEvent::Inbound(InboundCall::DeregisterTimer { resource_id, .. }) => resource_id,
    _ => arbitrary(),
  }
}

pub open spec fn get_current_timestamp(e: ReactorEvent) -> InstantView {
  match e {
    ReactorEvent::Outbound(OutboundCall::GetCurrentTime { timestamp }) => timestamp,
    _ => arbitrary(),
  }
}

pub open spec fn get_wake_task_waker(e: ReactorEvent) -> WakerView {
  match e {
    ReactorEvent::Outbound(OutboundCall::WakeTask { waker, .. }) => waker,
    _ => arbitrary(),
  }
}

pub open spec fn get_wake_task_source_rid(e: ReactorEvent) -> ResourceIdView {
  match e {
    ReactorEvent::Outbound(OutboundCall::WakeTask { source_rid, .. }) => source_rid,
    _ => arbitrary(),
  }
}

pub open spec fn get_io_event(e: ReactorEvent) -> IoEventView {
  match e {
    ReactorEvent::Outbound(OutboundCall::IoEventReady { event }) => event,
    _ => arbitrary(),
  }
}

// API-anchor rid accessors (Inbound fields)
pub open spec fn get_io_api_register_rid(e: ReactorEvent) -> ResourceIdView {
  match e {
    ReactorEvent::Inbound(InboundCall::RegisterIoResource { result: Some(IoResultView::Ok(rid)), .. }) => rid,
    _ => arbitrary(),
  }
}

pub open spec fn get_io_api_deregister_rid(e: ReactorEvent) -> ResourceIdView {
  match e {
    ReactorEvent::Inbound(InboundCall::DeregisterIoResource { resource_id, .. }) => resource_id,
    _ => arbitrary(),
  }
}

// Syscall-anchor rid accessors (Outbound fields)
pub open spec fn get_io_syscall_register_rid(e: ReactorEvent) -> ResourceIdView {
  match e {
    ReactorEvent::Outbound(OutboundCall::RegisterIoResource { resource_id, .. }) => resource_id,
    _ => arbitrary(),
  }
}

pub open spec fn get_io_syscall_deregister_rid(e: ReactorEvent) -> ResourceIdView {
  match e {
    ReactorEvent::Outbound(OutboundCall::DeregisterIoResource { resource_id, .. }) => resource_id,
    _ => arbitrary(),
  }
}

pub open spec fn get_set_waker_rid(e: ReactorEvent) -> ResourceIdView {
  match e {
    ReactorEvent::Inbound(InboundCall::SetWaker { resource_id, .. }) => resource_id,
    _ => arbitrary(),
  }
}

pub open spec fn get_set_waker_waker(e: ReactorEvent) -> WakerView {
  match e {
    ReactorEvent::Inbound(InboundCall::SetWaker { waker, .. }) => waker,
    _ => arbitrary(),
  }
}

pub open spec fn get_set_waker_interest(e: ReactorEvent) -> InterestView {
  match e {
    ReactorEvent::Inbound(InboundCall::SetWaker { interest, .. }) => interest,
    _ => arbitrary(),
  }
}

}
