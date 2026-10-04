use vstd::prelude::*;
use crate::types::*;
use crate::events::*;

verus! {

pub type Log = Seq<ExecutorEvent>;

// === Basic log predicates ===

pub open spec fn is_tick_begin_at(l: Log, i: int) -> bool {
  0 <= i < l.len() && is_tick_begin(l[i])
}

pub open spec fn is_tick_end_at(l: Log, i: int) -> bool {
  0 <= i < l.len() && is_tick_end(l[i])
}

pub open spec fn is_poll_task_at(l: Log, i: int) -> bool {
  0 <= i < l.len() && is_poll_task(l[i])
}

pub open spec fn is_drain_at(l: Log, i: int) -> bool {
  0 <= i < l.len() && is_drain(l[i])
}

pub open spec fn is_park_at(l: Log, i: int) -> bool {
  0 <= i < l.len() && is_park(l[i])
}

pub open spec fn is_pop_injection_at(l: Log, i: int) -> bool {
  0 <= i < l.len() && is_pop_injection(l[i])
}

// === Drain source predicates ===

pub open spec fn is_drain_deferred_at(l: Log, i: int) -> bool {
  is_drain_at(l, i) && get_drain_source(l[i]) == DrainSource::Deferred
}

pub open spec fn is_drain_reactor_wake_at(l: Log, i: int) -> bool {
  is_drain_at(l, i) && get_drain_source(l[i]) == DrainSource::ReactorWake
}

pub open spec fn is_drain_task_wake_at(l: Log, i: int) -> bool {
  is_drain_at(l, i) && get_drain_source(l[i]) == DrainSource::TaskWake
}

// === Tick window helpers ===

pub open spec fn no_tick_begin_between(l: Log, start: int, end: int) -> bool {
  forall |k: int| start < k <= end ==> !is_tick_begin_at(l, k)
}

pub open spec fn no_tick_end_between(l: Log, start: int, end: int) -> bool {
  forall |k: int| start < k < end ==> !is_tick_end_at(l, k)
}

// === Poll result predicates ===

pub open spec fn is_poll_task_for_id_at(l: Log, i: int, tid: TID) -> bool {
  is_poll_task_at(l, i) && get_poll_task_id(l[i]) == tid
}

pub open spec fn is_poll_pending_for_id_at(l: Log, i: int, tid: TID) -> bool {
  is_poll_task_for_id_at(l, i, tid) && get_poll_result(l[i]) == PollResult::<()>::Pending
}

pub open spec fn is_poll_ready_for_id_at(l: Log, i: int, tid: TID) -> bool {
  is_poll_task_for_id_at(l, i, tid) && get_poll_result(l[i]) == PollResult::<()>::Ready(())
}

// === Drain task membership ===

pub open spec fn task_id_in_drain_at(l: Log, i: int, tid: TID) -> bool
  recommends is_drain_at(l, i)
{
  let ids = get_drain_task_ids(l[i]);
  exists |j: int| #![trigger ids[j]] 0 <= j < ids.len() && ids[j] == tid
}

// === Tick cycle helpers ===

pub open spec fn find_last_tick_begin(l: Log, i: int) -> int
  recommends 0 <= i < l.len()
  decreases i + 1 when i >= -1
{
  if i < 0 {
    -1
  } else if i < l.len() && is_tick_begin(l[i]) {
    i
  } else {
    find_last_tick_begin(l, i - 1)
  }
}

pub open spec fn has_poll_task_for_id_after(l: Log, tid: TID, start: int) -> bool {
  exists |i: int| #![trigger l[i]] start <= i < l.len() && is_poll_task_for_id_at(l, i, tid)
}

pub open spec fn has_drain_with_task_id_after(l: Log, source: DrainSource, tid: TID, start: int) -> bool {
  exists |i: int| #![trigger l[i]]
    start <= i < l.len() &&
    is_drain_at(l, i) &&
    get_drain_source(l[i]) == source &&
    task_id_in_drain_at(l, i, tid)
}

// === Last poll tracking ===

pub open spec fn has_poll_for_id(l: Log, tid: TID) -> bool {
  exists |i: int| #![trigger l[i]] 0 <= i < l.len() && is_poll_task_for_id_at(l, i, tid)
}

pub open spec fn last_poll_idx_for_id(l: Log, tid: TID) -> int {
  choose |i: int|
    0 <= i < l.len() &&
    is_poll_task_for_id_at(l, i, tid) &&
    forall |j: int| i < j < l.len() ==> !is_poll_task_for_id_at(l, j, tid)
}

pub open spec fn last_poll_is_pending(l: Log, tid: TID) -> bool {
  has_poll_for_id(l, tid) &&
  is_poll_pending_for_id_at(l, last_poll_idx_for_id(l, tid), tid)
}

// === TID Uniqueness ===

// A TID is unique if it is delivered at most once by PopInjection-with-Some.
// NOTE: Drain contents are NOT constrained here — a drain may repeat a tid;
// current proofs only need injection-uniqueness (fifo tracks one positional
// occurrence).
pub open spec fn tid_unique_in_pop_injection(l: Log, tid: TID) -> bool {
  forall |i: int, j: int|
    #![trigger l[i], l[j]]
    0 <= i < l.len() && 0 <= j < l.len() && i != j &&
    is_pop_injection_at(l, i) && get_pop_injection_task(l[i]).is_some() &&
    get_pop_injection_task(l[i]).unwrap().id == tid &&
    is_pop_injection_at(l, j) && get_pop_injection_task(l[j]).is_some() &&
    get_pop_injection_task(l[j]).unwrap().id == tid
    ==> false
}

// Modeling note: the model treats spawned task ids as external input, so
// uniqueness is assumed at the env layer. The implementation independently
// ENFORCES injection-TID freshness via the verified ledger check in
// pop_injection_action (see TCB_and_limitations.md, executor table) — the
// assumption mirrors a checked property rather than adding a new demand.
pub open spec fn tid_unique(l: Log, tid: TID) -> bool {
  tid_unique_in_pop_injection(l, tid)
}

// (The old `tid_unique_persistent` / `queue_length_bounded_persistent` /
// `queue_bound_holds` quantified over ALL log extensions and were constant
// false — an adversarial extension always exists. Removed; single-state forms
// live in lion-liveness's executor/proof/queue_bound_single_state.rs and the
// composed layer.)

// Trigger wrapper for single-state queue-bound predicates.
pub open spec fn fifo_queue_at_for_persistent(l: Log, i: int) -> Seq<TID> {
  crate::fifo_queue::fifo_queue_at(l, i)
}

// Count PollTask events in a range [start, end).
pub open spec fn count_poll_tasks_in_range(l: Log, start: int, end: int) -> nat
  decreases (if end > start { end - start } else { 0 }) as nat
{
  if start >= end || start < 0 || end > l.len() {
    0
  } else if is_poll_task_at(l, start) {
    1 + count_poll_tasks_in_range(l, start + 1, end)
  } else {
    count_poll_tasks_in_range(l, start + 1, end)
  }
}

// === Arrival predicates (paper: nested_liveness_v2.tex lines 221-227) ===

// Check if l' is a prefix-extension of l
pub open spec fn is_prefix_of(l: Log, l_prime: Log) -> bool {
  l.len() <= l_prime.len() &&
  l =~= l_prime.subrange(0, l.len() as int)
}

// Count ALL PopInjection events in a range (regardless of Some/None result)
pub open spec fn count_pop_injection_between(l: Log, start: int, end: int) -> nat
  decreases (if end > start { end - start } else { 0 }) as nat
{
  if start >= end || start < 0 || end > l.len() {
    0
  } else if is_pop_injection_at(l, start) {
    1 + count_pop_injection_between(l, start + 1, end)
  } else {
    count_pop_injection_between(l, start + 1, end)
  }
}

pub open spec fn count_drain_between(l: Log, source: DrainSource, start: int, end: int) -> nat
  decreases (if end > start { end - start } else { 0 }) as nat
{
  if start >= end || start < 0 || end > l.len() {
    0
  } else if is_drain_at(l, start) && get_drain_source(l[start]) == source {
    1 + count_drain_between(l, source, start + 1, end)
  } else {
    count_drain_between(l, source, start + 1, end)
  }
}

// Number of scheduler tick-ends in [start, end). Mirrors count_drain_between but
// tests is_tick_end_at. Because each executor step's new content is EXACTLY ONE
// complete tick cycle (one Tick::End, the last event — see executor_progress /
// is_complete_tick_cycle), this counts the number of scheduler ticks elapsed over
// the range, which is the faithful "how long has the task waited" clock used by the
// TaskWake bounded-arrival clause (wake-routing Phase C).
pub open spec fn count_tick_ends_between(l: Log, start: int, end: int) -> nat
  decreases (if end > start { end - start } else { 0 }) as nat
{
  if start >= end || start < 0 || end > l.len() {
    0
  } else if is_tick_end_at(l, start) {
    1 + count_tick_ends_between(l, start + 1, end)
  } else {
    count_tick_ends_between(l, start + 1, end)
  }
}

pub open spec fn count_tick_ends_after(l: Log, start: int) -> nat {
  count_tick_ends_between(l, start, l.len() as int)
}

}
