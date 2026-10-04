use vstd::prelude::*;
use crate::types::{TID, TaskView, PollResult};

verus! {

// Drain source: where wakeup tasks come from
#[derive(PartialEq, Eq)]
pub enum DrainSource {
  ReactorWake,  // from reactor (timer/io ready)
  TaskWake,     // from other tasks (sync utilities)
  Deferred,     // from yield_now / defer
}

// Inbound: external calls into Executor (has Begin/End phases)
// Following the paper's design: result=None means Begin, Some means End
pub enum InboundCall {
  Tick { result: Option<()> },  // None=Begin, Some=End
}

// Outbound: operations initiated by Executor (no phases, atomic)
#[allow(inconsistent_fields)]
pub enum OutboundCall {
  PollTask {
    task_id: TID,
    task: Option<TaskView>,
    result: PollResult<()>,
  },
  Drain {
    source: DrainSource,
    task_ids: Seq<TID>,
  },
  Park,
  PopInjection {
    task: Option<TaskView>,
  },
}

pub enum ExecutorEvent {
  Inbound(InboundCall),
  Outbound(OutboundCall),
}

// === Inbound predicates ===

pub open spec fn is_tick_begin(e: ExecutorEvent) -> bool {
  matches!(e, ExecutorEvent::Inbound(InboundCall::Tick { result: None }))
}

pub open spec fn is_tick_end(e: ExecutorEvent) -> bool {
  matches!(e, ExecutorEvent::Inbound(InboundCall::Tick { result: Some(_) }))
}

// === Outbound predicates ===

pub open spec fn is_poll_task(e: ExecutorEvent) -> bool {
  matches!(e, ExecutorEvent::Outbound(OutboundCall::PollTask { .. }))
}

pub open spec fn is_drain(e: ExecutorEvent) -> bool {
  matches!(e, ExecutorEvent::Outbound(OutboundCall::Drain { .. }))
}

pub open spec fn is_park(e: ExecutorEvent) -> bool {
  matches!(e, ExecutorEvent::Outbound(OutboundCall::Park))
}

pub open spec fn is_pop_injection(e: ExecutorEvent) -> bool {
  matches!(e, ExecutorEvent::Outbound(OutboundCall::PopInjection { .. }))
}

// === Accessors ===

pub open spec fn get_poll_task_id(e: ExecutorEvent) -> TID {
  match e {
    ExecutorEvent::Outbound(OutboundCall::PollTask { task_id, .. }) => task_id,
    _ => arbitrary(),
  }
}

pub open spec fn get_poll_result(e: ExecutorEvent) -> PollResult<()> {
  match e {
    ExecutorEvent::Outbound(OutboundCall::PollTask { result, .. }) => result,
    _ => arbitrary(),
  }
}

pub open spec fn get_poll_task(e: ExecutorEvent) -> Option<TaskView> {
  match e {
    ExecutorEvent::Outbound(OutboundCall::PollTask { task, .. }) => task,
    _ => arbitrary(),
  }
}

pub open spec fn get_drain_source(e: ExecutorEvent) -> DrainSource {
  match e {
    ExecutorEvent::Outbound(OutboundCall::Drain { source, .. }) => source,
    _ => arbitrary(),
  }
}

pub open spec fn get_drain_task_ids(e: ExecutorEvent) -> Seq<TID> {
  match e {
    ExecutorEvent::Outbound(OutboundCall::Drain { task_ids, .. }) => task_ids,
    _ => arbitrary(),
  }
}

pub open spec fn get_pop_injection_task(e: ExecutorEvent) -> Option<TaskView> {
  match e {
    ExecutorEvent::Outbound(OutboundCall::PopInjection { task }) => task,
    _ => arbitrary(),
  }
}

}
