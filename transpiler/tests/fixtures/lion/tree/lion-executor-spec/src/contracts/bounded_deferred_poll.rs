use vstd::prelude::*;
use crate::types::*;
use crate::log::*;
use crate::events::*;
use lion_framework_spec::async_contract::*;

verus! {

// Bounded Deferred Poll Contract
//
// Semantics:
// - acceptance: Task T appears in a Drain{Deferred, ...} result
// - fulfillment: Task T is polled by the executor
// - assumption: Task IDs are unique
//
// When a task calls yield_now(), it defers itself to allow other tasks
// to run. The Deferred queue holds these yielded tasks.
// The queue is drained entirely each tick.

// NOT DISCHARGED, NOT CONSUMED — kept as the per-source statement only.
//
// No theorem proves the executor satisfies this AsyncContract value, and nothing
// reads it. That is deliberate, not an omission: the per-source liveness content
// it states is proved and consumed in a finer-grained form, so packaging it a
// second time as a contract value would duplicate the obligation, not add to it.
// The two legs live elsewhere:
//
//   arrival  lion-liveness/src/executor/proof/bounded_drain_poll.rs
//            `single_progress_has_drain_deferred` proves every progress step
//            contains at least one Drain{Deferred}; composed consumes it from
//            composed/proof/assumption_satisfiable.rs.
//   queue -> poll
//            lion-liveness/src/executor/proof/bounded_liveness.rs
//            `executor_drain_satisfies_liveness_env` discharges `drain_contract`,
//            which is keyed on FIFO-queue membership and so is shared by every
//            drain source rather than restated per source.
//
// This file therefore documents the contract the paper attributes to the
// Deferred queue; the proof of that contract is the pair above. Before deleting
// it, check that no reader wants the per-source statement in AsyncContract form.

// Option B: trigger/response are anchored at `l_start`, so they only
// observe events in the new segment [l_start.len(), l.len()). This removes the
// position-blindness that previously required contract_form_gap_bridges.
pub open spec fn trigger_fn(l_start: Log, l: Log, tid: TID) -> bool {
  has_drain_with_task_id_after(l, DrainSource::Deferred, tid, l_start.len() as int)
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

pub open spec fn bounded_deferred_poll(l_start: Log) -> AsyncContract<Log, TID> {
  AsyncContract {
    acceptance: |l: Log, tid: TID| trigger_fn(l_start, l, tid),
    fulfillment: |l: Log, tid: TID| response_fn(l_start, l, tid),
    assumption: |l: Log, tid: TID| assumption_fn(l, tid),
  }
}

}
