use vstd::prelude::*;
use crate::types::*;
use crate::log::*;
use crate::events::*;
use lion_framework_spec::async_contract::*;

verus! {

// Generic Bounded Drain Poll Contract
//
// This is a parameterized contract that covers all three Drain-based contracts:
// - Bounded ReactorWake Poll (source = ReactorWake)
// - Bounded TaskWake Poll (source = TaskWake)
// - Bounded Deferred Poll (source = Deferred)
//
// Semantics:
// - acceptance: Task T appears in a Drain{source, ...} result
// - fulfillment: Task T is polled by the executor
// - assumption: Task IDs are unique
//
// All three Drain queues are drained entirely each tick.

// Option B: trigger/response anchored at `l_start` (new segment only).
pub open spec fn trigger_fn(l_start: Log, source: DrainSource, l: Log, tid: TID) -> bool {
  has_drain_with_task_id_after(l, source, tid, l_start.len() as int)
}

pub open spec fn response_fn(l_start: Log, l: Log, tid: TID) -> bool {
  has_poll_task_for_id_after(l, tid, l_start.len() as int)
}

// Single-state form (satisfiable). The old conjuncts `tid_unique_persistent` /
// `queue_length_bounded_persistent` quantified over ALL log extensions and were
// constant false; the single-state queue bound lives in
// lion-liveness's executor/proof/queue_bound_single_state.rs.
pub open spec fn assumption_fn(l: Log, tid: TID) -> bool {
  tid_unique(l, tid)
}

pub open spec fn bounded_drain_poll(l_start: Log, source: DrainSource) -> AsyncContract<Log, TID> {
  AsyncContract {
    acceptance: |l: Log, tid: TID| trigger_fn(l_start, source, l, tid),
    fulfillment: |l: Log, tid: TID| response_fn(l_start, l, tid),
    assumption: |l: Log, tid: TID| assumption_fn(l, tid),
  }
}

}
