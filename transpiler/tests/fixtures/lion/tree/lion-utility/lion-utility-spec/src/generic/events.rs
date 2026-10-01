use vstd::prelude::*;
use crate::generic::types::*;

verus! {

// ============================================================================
// Generic per-utility event vocabulary (aligned with the paper's §5.1 and the
// LION-RE design draft). Parameterized by:
//   M = utility-specific method tag (e.g. NotifyMethod = Notified | NotifyOne)
//   R = utility-specific return value type (sum over all methods' returns)
//
// This is the verification-template vocabulary each concrete utility (Sleep,
// Notify, Mutex, TcpStream, …) instantiates with its own (M, R).
// ============================================================================

// Inbound: a method call on the utility, bracketed Begin/End. `waker` is the
// calling task's `cx.waker()`; `method` is the utility-specific method tag.
#[allow(inconsistent_fields)]
pub enum UtilityInbound<M, R> {
  Tick {
    waker: WakerView,
    method: M,
    result: Option<TickResult<R>>,   // None = Begin, Some = End
  },
}

// Outbound: the utility's actions on its environment.
#[allow(inconsistent_fields)]
pub enum UtilityOutbound {
  // Abstract (generic contract): register / fire a task waker.
  PassWaker { waker: WakerView },
  WakeWaker { waker: WakerView },

  // A parked waiter withdraws: its future was dropped/cancelled before its wake.
  // Removes the waiter from the utility's queue; does not touch permits.
  CancelWaker { waker: WakerView },

  // Concrete reactor outbounds.
  RegisterTimer { deadline: InstantView, waker: WakerView, result: Option<ResourceIdView> },
  DeregisterTimer { resource_id: ResourceIdView, result: bool },
  RegisterIoResource { source: SourceView, interest: InterestView, result: Option<IoResultView<ResourceIdView>> },
  DeregisterIoResource { resource_id: ResourceIdView, result: IoResultView<()> },
  SetIoWaker { resource_id: ResourceIdView, interest: InterestView, waker: WakerView, result: IoResultView<()> },

  // Concrete executor outbound.
  Defer,
}

pub enum UtilityEvent<M, R> {
  Inbound(UtilityInbound<M, R>),
  Outbound(UtilityOutbound),
}

// ============================================================================
// Inbound predicates / accessors
// ============================================================================

pub open spec fn is_tick<M, R>(e: UtilityEvent<M, R>) -> bool {
  matches!(e, UtilityEvent::Inbound(UtilityInbound::Tick { .. }))
}

pub open spec fn is_tick_begin<M, R>(e: UtilityEvent<M, R>) -> bool {
  matches!(e, UtilityEvent::Inbound(UtilityInbound::Tick { result: None, .. }))
}

pub open spec fn is_tick_end<M, R>(e: UtilityEvent<M, R>) -> bool {
  matches!(e, UtilityEvent::Inbound(UtilityInbound::Tick { result: Some(_), .. }))
}

pub open spec fn is_tick_end_pending<M, R>(e: UtilityEvent<M, R>) -> bool {
  matches!(e, UtilityEvent::Inbound(UtilityInbound::Tick { result: Some(TickResult::Pending), .. }))
}

pub open spec fn is_tick_end_finished<M, R>(e: UtilityEvent<M, R>) -> bool {
  matches!(e, UtilityEvent::Inbound(UtilityInbound::Tick { result: Some(TickResult::Finished(_)), .. }))
}

pub open spec fn is_tick_end_ongoing<M, R>(e: UtilityEvent<M, R>) -> bool {
  matches!(e, UtilityEvent::Inbound(UtilityInbound::Tick { result: Some(TickResult::Ongoing(_)), .. }))
}

pub open spec fn get_tick_waker<M, R>(e: UtilityEvent<M, R>) -> WakerView {
  match e {
    UtilityEvent::Inbound(UtilityInbound::Tick { waker, .. }) => waker,
    _ => arbitrary(),
  }
}

pub open spec fn get_tick_method<M, R>(e: UtilityEvent<M, R>) -> M {
  match e {
    UtilityEvent::Inbound(UtilityInbound::Tick { method, .. }) => method,
    _ => arbitrary(),
  }
}

pub open spec fn get_tick_return<M, R>(e: UtilityEvent<M, R>) -> R {
  match e {
    UtilityEvent::Inbound(UtilityInbound::Tick { result: Some(TickResult::Finished(r)), .. }) => r,
    UtilityEvent::Inbound(UtilityInbound::Tick { result: Some(TickResult::Ongoing(r)), .. }) => r,
    _ => arbitrary(),
  }
}

// ============================================================================
// Outbound predicates — abstract (PassWaker / WakeWaker)
// ============================================================================

pub open spec fn is_pass_waker<M, R>(e: UtilityEvent<M, R>) -> bool {
  matches!(e, UtilityEvent::Outbound(UtilityOutbound::PassWaker { .. }))
}

pub open spec fn get_pass_waker_waker<M, R>(e: UtilityEvent<M, R>) -> WakerView {
  match e {
    UtilityEvent::Outbound(UtilityOutbound::PassWaker { waker }) => waker,
    _ => arbitrary(),
  }
}

pub open spec fn is_wake_waker<M, R>(e: UtilityEvent<M, R>) -> bool {
  matches!(e, UtilityEvent::Outbound(UtilityOutbound::WakeWaker { .. }))
}

pub open spec fn get_wake_waker_waker<M, R>(e: UtilityEvent<M, R>) -> WakerView {
  match e {
    UtilityEvent::Outbound(UtilityOutbound::WakeWaker { waker }) => waker,
    _ => arbitrary(),
  }
}

pub open spec fn is_cancel_waker<M, R>(e: UtilityEvent<M, R>) -> bool {
  matches!(e, UtilityEvent::Outbound(UtilityOutbound::CancelWaker { .. }))
}

pub open spec fn get_cancel_waker_waker<M, R>(e: UtilityEvent<M, R>) -> WakerView {
  match e {
    UtilityEvent::Outbound(UtilityOutbound::CancelWaker { waker }) => waker,
    _ => arbitrary(),
  }
}

// ============================================================================
// Outbound predicates — concrete reactor / executor calls
// ============================================================================

pub open spec fn is_register_timer<M, R>(e: UtilityEvent<M, R>) -> bool {
  matches!(e, UtilityEvent::Outbound(UtilityOutbound::RegisterTimer { .. }))
}

pub open spec fn is_register_timer_succ<M, R>(e: UtilityEvent<M, R>) -> bool {
  matches!(e, UtilityEvent::Outbound(UtilityOutbound::RegisterTimer { result: Some(_), .. }))
}

pub open spec fn is_deregister_timer<M, R>(e: UtilityEvent<M, R>) -> bool {
  matches!(e, UtilityEvent::Outbound(UtilityOutbound::DeregisterTimer { .. }))
}

pub open spec fn is_register_io<M, R>(e: UtilityEvent<M, R>) -> bool {
  matches!(e, UtilityEvent::Outbound(UtilityOutbound::RegisterIoResource { .. }))
}

pub open spec fn is_register_io_succ<M, R>(e: UtilityEvent<M, R>) -> bool {
  match e {
    UtilityEvent::Outbound(UtilityOutbound::RegisterIoResource { result: Some(IoResultView::Ok(_)), .. }) => true,
    _ => false,
  }
}

pub open spec fn is_deregister_io<M, R>(e: UtilityEvent<M, R>) -> bool {
  matches!(e, UtilityEvent::Outbound(UtilityOutbound::DeregisterIoResource { .. }))
}

pub open spec fn is_set_io_waker<M, R>(e: UtilityEvent<M, R>) -> bool {
  matches!(e, UtilityEvent::Outbound(UtilityOutbound::SetIoWaker { .. }))
}

pub open spec fn is_defer<M, R>(e: UtilityEvent<M, R>) -> bool {
  matches!(e, UtilityEvent::Outbound(UtilityOutbound::Defer))
}

// Resource token of any resource operation (Deregister* / SetIoWaker) — used
// by the resource_ownership invariant template.
pub open spec fn get_resource_token<M, R>(e: UtilityEvent<M, R>) -> ResourceIdView {
  match e {
    UtilityEvent::Outbound(UtilityOutbound::DeregisterTimer { resource_id, .. }) => resource_id,
    UtilityEvent::Outbound(UtilityOutbound::DeregisterIoResource { resource_id, .. }) => resource_id,
    UtilityEvent::Outbound(UtilityOutbound::SetIoWaker { resource_id, .. }) => resource_id,
    _ => arbitrary(),
  }
}

// Resource token produced by a successful Register* — paired with the above.
pub open spec fn get_register_timer_token<M, R>(e: UtilityEvent<M, R>) -> ResourceIdView {
  match e {
    UtilityEvent::Outbound(UtilityOutbound::RegisterTimer { result: Some(rt), .. }) => rt,
    _ => arbitrary(),
  }
}

pub open spec fn get_register_io_token<M, R>(e: UtilityEvent<M, R>) -> ResourceIdView {
  match e {
    UtilityEvent::Outbound(UtilityOutbound::RegisterIoResource { result: Some(IoResultView::Ok(rt)), .. }) => rt,
    _ => arbitrary(),
  }
}

// Waker carried by a RegisterTimer — the timer fires this waker on deadline.
pub open spec fn get_register_timer_waker<M, R>(e: UtilityEvent<M, R>) -> WakerView {
  match e {
    UtilityEvent::Outbound(UtilityOutbound::RegisterTimer { waker, .. }) => waker,
    _ => arbitrary(),
  }
}

// Waker armed on an io resource by a SetIoWaker — the reactor fires it on readiness.
pub open spec fn get_set_io_waker_waker<M, R>(e: UtilityEvent<M, R>) -> WakerView {
  match e {
    UtilityEvent::Outbound(UtilityOutbound::SetIoWaker { waker, .. }) => waker,
    _ => arbitrary(),
  }
}

}
