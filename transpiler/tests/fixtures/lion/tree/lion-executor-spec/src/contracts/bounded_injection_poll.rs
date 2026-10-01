use vstd::prelude::*;
use crate::types::*;
use crate::log::*;
use crate::events::*;
use lion_framework_spec::async_contract::*;

verus! {

// Bounded Injection Poll Contract
//
// Paper reference: §4 names this contract `input_queue_poll` and its queue the
// "input queue". That is Example 1's deliberately simplified executor, whose one
// input queue is this injection queue (the Injection Queue of §3). Lion's other
// arrival paths carry their own contracts: bounded_reactor_wake_poll,
// bounded_task_wake_poll, bounded_deferred_poll, bounded_drain_poll.
//
// Semantics:
// - acceptance: Task T appears in a PopInjection result (i.e., popped from queue)
// - fulfillment: Task T is polled by the executor
// - assumption: Task IDs are unique (no duplicate spawns)

// Option B: trigger/response anchored at `l_start` (new segment only).
pub open spec fn trigger_fn(l_start: Log, l: Log, tid: TID) -> bool {
  exists |i: int| #![trigger l[i]]
    l_start.len() as int <= i < l.len() &&
    is_pop_injection_at(l, i) &&
    get_pop_injection_task(l[i]).is_some() &&
    get_pop_injection_task(l[i]).unwrap().id == tid
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

pub open spec fn bounded_injection_poll(l_start: Log) -> AsyncContract<Log, TID> {
  AsyncContract {
    acceptance: |l: Log, tid: TID| trigger_fn(l_start, l, tid),
    fulfillment: |l: Log, tid: TID| response_fn(l_start, l, tid),
    assumption: |l: Log, tid: TID| assumption_fn(l, tid),
  }
}

}
