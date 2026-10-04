use vstd::prelude::*;
use crate::spec::log::*;
use crate::spec::fifo_queue::*;
use crate::types::{TID, TaskView, PollResult};
use crate::framework::action_safety::*;
use crate::framework::local_liveness::*;

// ---------------------------------------------------------------------------
// Invariant templates (paper Listing 4): each executor property is an instance
// of ActionSafety<Log> or LocalLiveness<Log>, shared via lion-executor-spec.
// The template closures use the same predicates as the inlined inv_* below, so
// *_satisfied(P(), l) unfolds to the corresponding inv_*(l). The equivalence is
// made explicit in executor_inv_unfold below.
// ---------------------------------------------------------------------------

// action-safety instances (shared; ghost-gated — these are spec fns, absent
// from plain cargo builds)
#[cfg(verus_keep_ghost)]
pub use lion_executor_spec::invariants::fifo_task_selection::fifo_task_selection;
#[cfg(verus_keep_ghost)]
pub use lion_executor_spec::invariants::valid_task_polling::valid_task_polling;
#[cfg(verus_keep_ghost)]
pub use lion_executor_spec::invariants::poll_within_tick::poll_within_tick;
#[cfg(verus_keep_ghost)]
pub use lion_executor_spec::invariants::tick_has_park::tick_has_park;
#[cfg(verus_keep_ghost)]
pub use lion_executor_spec::invariants::tick_has_pop_injection::tick_has_pop_injection;
#[cfg(verus_keep_ghost)]
pub use lion_executor_spec::invariants::tick_has_drain_deferred::tick_has_drain_deferred;
#[cfg(verus_keep_ghost)]
pub use lion_executor_spec::invariants::tick_has_drain_task_wake::tick_has_drain_task_wake;

// local-liveness instances (shared)
#[cfg(verus_keep_ghost)]
pub use lion_executor_spec::invariants::park_drain_reactor_wake::park_drain_reactor_wake;
#[cfg(verus_keep_ghost)]
pub use lion_executor_spec::invariants::tick_polls_if_runnable::tick_polls_if_runnable;

// helper predicates shared with the templates
#[cfg(verus_keep_ghost)]
pub use lion_executor_spec::invariants::valid_task_polling::{
  tid_was_injected_before, tid_returned_ready_before, tid_is_invalid};

verus! {

// E1: After park, drain_reactor_wake occurs before tick_end (same tick)
pub open spec fn inv_park_drain_reactor_wake(l: Log) -> bool {
  forall |i: int| is_park_at(l, i) ==>
    exists |j: int| #![auto]
      j > i && is_drain_reactor_wake_at(l, j) &&
      no_tick_end_between(l, i, j)
}

// E2: TICK_POLLS_IF_RUNNABLE
// If the FIFO queue is non-empty at tick_begin, then a PollTask occurs before tick_end
pub open spec fn inv_tick_polls_if_runnable(l: Log) -> bool {
  forall |i: int|
    is_tick_begin_at(l, i) && fifo_queue_at(l, i).len() > 0 ==>
    exists |j: int| #![auto]
      j > i && is_poll_task_at(l, j) &&
      (forall |k: int| i < k < j ==> !#[trigger] is_tick_end_at(l, k))
}

// E3: Every PollTask has a preceding tick_begin (polls only happen within ticks)
pub open spec fn inv_poll_within_tick(l: Log) -> bool {
  forall |i: int| is_poll_task_at(l, i) ==>
    exists |tb: int| #![auto]
      0 <= tb < i && is_tick_begin_at(l, tb) &&
      no_tick_begin_between(l, tb, i)
}

// E4: VALID_TASK_POLLING (enhanced)
// Every polled task was previously injected, and
// result == Invalid <==> tid_is_invalid
pub open spec fn inv_valid_task_polling(l: Log) -> bool {
  forall |i: int| is_poll_task_at(l, i) ==> {
    let tid = get_poll_task_id(l[i]);
    let result = get_poll_result(l[i]);
    tid_was_injected_before(l, i, tid) &&
    (result == PollResult::<()>::Invalid <==> tid_is_invalid(l, i, tid))
  }
}

// E5: Every tick contains a park
pub open spec fn inv_tick_has_park(l: Log) -> bool {
  forall |i: int| is_tick_end_at(l, i) ==>
    exists |p: int| #![auto]
      0 <= p < i && is_park_at(l, p) &&
      no_tick_begin_between(l, p, i)
}

// E6: Every tick contains a pop_injection
pub open spec fn inv_tick_has_pop_injection(l: Log) -> bool {
  forall |i: int| is_tick_end_at(l, i) ==>
    exists |p: int| #![auto]
      0 <= p < i && is_pop_injection_at(l, p) &&
      no_tick_begin_between(l, p, i)
}

// E7: Every tick contains a drain_deferred
pub open spec fn inv_tick_has_drain_deferred(l: Log) -> bool {
  forall |i: int| is_tick_end_at(l, i) ==>
    exists |d: int| #![auto]
      0 <= d < i && is_drain_deferred_at(l, d) &&
      no_tick_begin_between(l, d, i)
}

// E8: Every tick contains a drain_task_wake
pub open spec fn inv_tick_has_drain_task_wake(l: Log) -> bool {
  forall |i: int| is_tick_end_at(l, i) ==>
    exists |d: int| #![auto]
      0 <= d < i && is_drain_task_wake_at(l, d) &&
      no_tick_begin_between(l, d, i)
}

// E9: FIFO_TASK_SELECTION
// Every PollTask selects the FIFO head
pub open spec fn inv_fifo_task_selection(l: Log) -> bool {
  forall |i: int| is_poll_task_at(l, i) ==> {
    let tid = get_poll_task_id(l[i]);
    is_fifo_head_at(l, i, tid)
  }
}

// Combined structural invariant (E1, E5-E8)
pub open spec fn structural_inv(l: Log) -> bool {
  local_liveness_satisfied(park_drain_reactor_wake(), l) &&
  action_safety_satisfied(tick_has_park(), l) &&
  action_safety_satisfied(tick_has_pop_injection(), l) &&
  action_safety_satisfied(tick_has_drain_deferred(), l) &&
  action_safety_satisfied(tick_has_drain_task_wake(), l)
}

// Combined semantic invariant (E2-E4, E9)
pub open spec fn semantic_inv(l: Log) -> bool {
  local_liveness_satisfied(tick_polls_if_runnable(), l) &&
  action_safety_satisfied(poll_within_tick(), l) &&
  action_safety_satisfied(valid_task_polling(), l) &&
  action_safety_satisfied(fifo_task_selection(), l)
}

// Full executor invariant: all properties
pub open spec fn executor_inv(l: Log) -> bool {
  structural_inv(l) &&
  semantic_inv(l)
}

// Per-property equivalence helpers: each template instance is logically the
// same predicate as the matching inlined inv_*. We prove both directions
// pointwise: a single closure application (p.acceptance)(l, i) beta-reduces to
// the underlying predicate, which lets the two foralls line up index by index.

pub proof fn eq_park_drain_reactor_wake(l: Log)
  ensures local_liveness_satisfied(park_drain_reactor_wake(), l) <==> inv_park_drain_reactor_wake(l)
{
  let p = park_drain_reactor_wake();
  if local_liveness_satisfied(p, l) {
    assert forall |i: int| is_park_at(l, i) implies
      (exists |j: int| #![auto] j > i && is_drain_reactor_wake_at(l, j) && no_tick_end_between(l, i, j)) by {
      assert((p.acceptance)(l, i));
      let j = choose |j: int| #![trigger (p.fulfillment)(l, i, j)] j > i && (p.fulfillment)(l, i, j) && (p.timely)(l, i, j);
      assert((p.fulfillment)(l, i, j));
      assert((p.timely)(l, i, j));
    }
  }
  if inv_park_drain_reactor_wake(l) {
    assert forall |i: int| #[trigger] (p.acceptance)(l, i) implies
      (exists |j: int| #![trigger (p.fulfillment)(l, i, j)] j > i && (p.fulfillment)(l, i, j) && (p.timely)(l, i, j)) by {
      assert(is_park_at(l, i));
      let j = choose |j: int| #![auto] j > i && is_drain_reactor_wake_at(l, j) && no_tick_end_between(l, i, j);
      assert((p.fulfillment)(l, i, j));
      assert((p.timely)(l, i, j));
    }
  }
}

pub proof fn eq_tick_polls_if_runnable(l: Log)
  ensures local_liveness_satisfied(tick_polls_if_runnable(), l) <==> inv_tick_polls_if_runnable(l)
{
  let p = tick_polls_if_runnable();
  if local_liveness_satisfied(p, l) {
    assert forall |i: int| is_tick_begin_at(l, i) && fifo_queue_at(l, i).len() > 0 implies
      (exists |j: int| #![auto] j > i && is_poll_task_at(l, j) &&
        (forall |k: int| i < k < j ==> !#[trigger] is_tick_end_at(l, k))) by {
      assert((p.acceptance)(l, i));
      let j = choose |j: int| #![trigger (p.fulfillment)(l, i, j)] j > i && (p.fulfillment)(l, i, j) && (p.timely)(l, i, j);
      assert((p.fulfillment)(l, i, j));
      assert((p.timely)(l, i, j));
    }
  }
  if inv_tick_polls_if_runnable(l) {
    assert forall |i: int| #[trigger] (p.acceptance)(l, i) implies
      (exists |j: int| #![trigger (p.fulfillment)(l, i, j)] j > i && (p.fulfillment)(l, i, j) && (p.timely)(l, i, j)) by {
      assert(is_tick_begin_at(l, i) && fifo_queue_at(l, i).len() > 0);
      let j = choose |j: int| #![auto] j > i && is_poll_task_at(l, j) &&
        (forall |k: int| i < k < j ==> !#[trigger] is_tick_end_at(l, k));
      assert((p.fulfillment)(l, i, j));
      assert((p.timely)(l, i, j));
    }
  }
}

pub proof fn eq_tick_has_park(l: Log)
  ensures action_safety_satisfied(tick_has_park(), l) <==> inv_tick_has_park(l)
{
  let p = tick_has_park();
  if action_safety_satisfied(p, l) {
    assert forall |i: int| is_tick_end_at(l, i) implies
      (exists |q: int| #![auto] 0 <= q < i && is_park_at(l, q) && no_tick_begin_between(l, q, i)) by {
      assert((p.acceptance)(l, i));
      assert((p.validity)(l, i));
    }
  }
  if inv_tick_has_park(l) {
    assert forall |i: int| #[trigger] (p.acceptance)(l, i) implies (p.validity)(l, i) by {
      assert(is_tick_end_at(l, i));
      assert(exists |q: int| #![auto] 0 <= q < i && is_park_at(l, q) && no_tick_begin_between(l, q, i));
    }
  }
}

pub proof fn eq_tick_has_pop_injection(l: Log)
  ensures action_safety_satisfied(tick_has_pop_injection(), l) <==> inv_tick_has_pop_injection(l)
{
  let p = tick_has_pop_injection();
  if action_safety_satisfied(p, l) {
    assert forall |i: int| is_tick_end_at(l, i) implies
      (exists |q: int| #![auto] 0 <= q < i && is_pop_injection_at(l, q) && no_tick_begin_between(l, q, i)) by {
      assert((p.acceptance)(l, i));
      assert((p.validity)(l, i));
    }
  }
  if inv_tick_has_pop_injection(l) {
    assert forall |i: int| #[trigger] (p.acceptance)(l, i) implies (p.validity)(l, i) by {
      assert(is_tick_end_at(l, i));
      assert(exists |q: int| #![auto] 0 <= q < i && is_pop_injection_at(l, q) && no_tick_begin_between(l, q, i));
    }
  }
}

pub proof fn eq_tick_has_drain_deferred(l: Log)
  ensures action_safety_satisfied(tick_has_drain_deferred(), l) <==> inv_tick_has_drain_deferred(l)
{
  let p = tick_has_drain_deferred();
  if action_safety_satisfied(p, l) {
    assert forall |i: int| is_tick_end_at(l, i) implies
      (exists |d: int| #![auto] 0 <= d < i && is_drain_deferred_at(l, d) && no_tick_begin_between(l, d, i)) by {
      assert((p.acceptance)(l, i));
      assert((p.validity)(l, i));
    }
  }
  if inv_tick_has_drain_deferred(l) {
    assert forall |i: int| #[trigger] (p.acceptance)(l, i) implies (p.validity)(l, i) by {
      assert(is_tick_end_at(l, i));
      assert(exists |d: int| #![auto] 0 <= d < i && is_drain_deferred_at(l, d) && no_tick_begin_between(l, d, i));
    }
  }
}

pub proof fn eq_tick_has_drain_task_wake(l: Log)
  ensures action_safety_satisfied(tick_has_drain_task_wake(), l) <==> inv_tick_has_drain_task_wake(l)
{
  let p = tick_has_drain_task_wake();
  if action_safety_satisfied(p, l) {
    assert forall |i: int| is_tick_end_at(l, i) implies
      (exists |d: int| #![auto] 0 <= d < i && is_drain_task_wake_at(l, d) && no_tick_begin_between(l, d, i)) by {
      assert((p.acceptance)(l, i));
      assert((p.validity)(l, i));
    }
  }
  if inv_tick_has_drain_task_wake(l) {
    assert forall |i: int| #[trigger] (p.acceptance)(l, i) implies (p.validity)(l, i) by {
      assert(is_tick_end_at(l, i));
      assert(exists |d: int| #![auto] 0 <= d < i && is_drain_task_wake_at(l, d) && no_tick_begin_between(l, d, i));
    }
  }
}

pub proof fn eq_poll_within_tick(l: Log)
  ensures action_safety_satisfied(poll_within_tick(), l) <==> inv_poll_within_tick(l)
{
  let p = poll_within_tick();
  if action_safety_satisfied(p, l) {
    assert forall |i: int| is_poll_task_at(l, i) implies
      (exists |tb: int| #![auto] 0 <= tb < i && is_tick_begin_at(l, tb) && no_tick_begin_between(l, tb, i)) by {
      assert((p.acceptance)(l, i));
      assert((p.validity)(l, i));
    }
  }
  if inv_poll_within_tick(l) {
    assert forall |i: int| #[trigger] (p.acceptance)(l, i) implies (p.validity)(l, i) by {
      assert(is_poll_task_at(l, i));
      assert(exists |tb: int| #![auto] 0 <= tb < i && is_tick_begin_at(l, tb) && no_tick_begin_between(l, tb, i));
    }
  }
}

pub proof fn eq_valid_task_polling(l: Log)
  ensures action_safety_satisfied(valid_task_polling(), l) <==> inv_valid_task_polling(l)
{
  let p = valid_task_polling();
  if action_safety_satisfied(p, l) {
    assert forall |i: int| is_poll_task_at(l, i) implies {
      let tid = get_poll_task_id(l[i]);
      let result = get_poll_result(l[i]);
      tid_was_injected_before(l, i, tid) &&
      (result == PollResult::<()>::Invalid <==> tid_is_invalid(l, i, tid))
    } by {
      assert((p.acceptance)(l, i));
      assert((p.validity)(l, i));
    }
  }
  if inv_valid_task_polling(l) {
    assert forall |i: int| #[trigger] (p.acceptance)(l, i) implies (p.validity)(l, i) by {
      assert(is_poll_task_at(l, i));
    }
  }
}

pub proof fn eq_fifo_task_selection(l: Log)
  ensures action_safety_satisfied(fifo_task_selection(), l) <==> inv_fifo_task_selection(l)
{
  let p = fifo_task_selection();
  if action_safety_satisfied(p, l) {
    assert forall |i: int| is_poll_task_at(l, i) implies
      is_fifo_head_at(l, i, get_poll_task_id(l[i])) by {
      assert((p.acceptance)(l, i));
      assert((p.validity)(l, i));
    }
  }
  if inv_fifo_task_selection(l) {
    assert forall |i: int| #[trigger] (p.acceptance)(l, i) implies (p.validity)(l, i) by {
      assert(is_poll_task_at(l, i));
    }
  }
}

// Equivalence bridge: the template form of executor_inv unfolds, property by
// property, to the inlined inv_* conjunction. Consumers that reason in terms of
// inv_* (preservation, tick) cross between the two forms through this lemma.
pub proof fn executor_inv_unfold(l: Log)
  ensures
    executor_inv(l) <==> (
      inv_park_drain_reactor_wake(l) &&
      inv_tick_has_park(l) &&
      inv_tick_has_pop_injection(l) &&
      inv_tick_has_drain_deferred(l) &&
      inv_tick_has_drain_task_wake(l) &&
      inv_tick_polls_if_runnable(l) &&
      inv_poll_within_tick(l) &&
      inv_valid_task_polling(l) &&
      inv_fifo_task_selection(l)
    ),
{
  eq_park_drain_reactor_wake(l);
  eq_tick_has_park(l);
  eq_tick_has_pop_injection(l);
  eq_tick_has_drain_deferred(l);
  eq_tick_has_drain_task_wake(l);
  eq_tick_polls_if_runnable(l);
  eq_poll_within_tick(l);
  eq_valid_task_polling(l);
  eq_fifo_task_selection(l);
}

// (tid_was_injected_before / tid_returned_ready_before / tid_is_invalid moved
// to lion-executor-spec::invariants::valid_task_polling; re-exported above.)

// Data invariant: all task_ids in local_queue were previously injected
pub open spec fn all_queue_tids_injected(l: Log, queue: Seq<TID>) -> bool {
  forall |k: int| 0 <= k < queue.len() as int ==>
    tid_was_injected_before(l, l.len() as int, queue[k])
}

// Data invariant: slab contents match log state
// A task is in the slab iff it was injected and hasn't returned Ready yet
pub open spec fn slab_matches_log(slab: Map<TID, TaskView>, l: Log) -> bool {
  forall |tid: TID|
    slab.contains_key(tid) <==>
      (tid_was_injected_before(l, l.len() as int, tid) &&
       !tid_returned_ready_before(l, l.len() as int, tid))
}

// Data invariant: the exec-side TID ledger mirrors the log's pop-injection
// history exactly. This is what turns the old pop_injection freshness AXIOM
// into a machine-checked runtime check: !ledger.spec_has(tid) at the moment
// of a pop is, by this coupling, !tid_was_injected_before.
pub open spec fn ledger_matches_log(ledger: crate::collections::TidLedger, l: Log) -> bool {
  forall |t: TID| ledger.spec_has(t) <==> tid_was_injected_before(l, l.len() as int, t)
}

// Appending an event that is not a successful PopInjection leaves the
// pop-injection history — and hence the ledger coupling — untouched.
pub proof fn ledger_preserved_by_non_pop(ledger: crate::collections::TidLedger, l1: Log, l2: Log)
  requires
    ledger_matches_log(ledger, l1),
    l2.len() == l1.len() + 1,
    forall |k: int| 0 <= k < l1.len() ==> l2[k] == l1[k],
    !is_pop_injection_at(l2, l1.len() as int) ||
      !get_pop_injection_task(l2[l1.len() as int]).is_some(),
  ensures ledger_matches_log(ledger, l2),
{
  assert forall |t: TID| ledger.spec_has(t) <==>
    tid_was_injected_before(l2, l2.len() as int, t)
  by {
    if tid_was_injected_before(l2, l2.len() as int, t) {
      let j = choose |j: int| #![trigger l2[j]] 0 <= j < l2.len() &&
        is_pop_injection_at(l2, j) &&
        get_pop_injection_task(l2[j]).is_some() &&
        get_pop_injection_task(l2[j]).unwrap().id == t;
      assert(j < l1.len());
      assert(l1[j] == l2[j]);
      assert(tid_was_injected_before(l1, l1.len() as int, t));
    }
    if tid_was_injected_before(l1, l1.len() as int, t) {
      let j = choose |j: int| #![trigger l1[j]] 0 <= j < l1.len() &&
        is_pop_injection_at(l1, j) &&
        get_pop_injection_task(l1[j]).is_some() &&
        get_pop_injection_task(l1[j]).unwrap().id == t;
      assert(l2[j] == l1[j]);
      assert(tid_was_injected_before(l2, l2.len() as int, t));
    }
  }
}

// A successful PopInjection of `tid` plus marking `tid` in the ledger
// re-establishes the coupling.
pub proof fn ledger_updated_by_pop_some(
  old_ledger: crate::collections::TidLedger,
  new_ledger: crate::collections::TidLedger,
  l1: Log, l2: Log, tid: TID,
)
  requires
    ledger_matches_log(old_ledger, l1),
    forall |t: TID| new_ledger.spec_has(t) <==> (old_ledger.spec_has(t) || t == tid),
    l2.len() == l1.len() + 1,
    forall |k: int| 0 <= k < l1.len() ==> l2[k] == l1[k],
    is_pop_injection_at(l2, l1.len() as int),
    get_pop_injection_task(l2[l1.len() as int]).is_some(),
    get_pop_injection_task(l2[l1.len() as int]).unwrap().id == tid,
  ensures ledger_matches_log(new_ledger, l2),
{
  assert forall |t: TID| new_ledger.spec_has(t) <==>
    tid_was_injected_before(l2, l2.len() as int, t)
  by {
    if t == tid {
      assert(is_pop_injection_at(l2, l1.len() as int));
      assert(tid_was_injected_before(l2, l2.len() as int, t));
    } else {
      if tid_was_injected_before(l2, l2.len() as int, t) {
        let j = choose |j: int| #![trigger l2[j]] 0 <= j < l2.len() &&
          is_pop_injection_at(l2, j) &&
          get_pop_injection_task(l2[j]).is_some() &&
          get_pop_injection_task(l2[j]).unwrap().id == t;
        if j < l1.len() {
          assert(l1[j] == l2[j]);
          assert(tid_was_injected_before(l1, l1.len() as int, t));
        } else {
          assert(j == l1.len());
          assert(false);
        }
      }
      if tid_was_injected_before(l1, l1.len() as int, t) {
        let j = choose |j: int| #![trigger l1[j]] 0 <= j < l1.len() &&
          is_pop_injection_at(l1, j) &&
          get_pop_injection_task(l1[j]).is_some() &&
          get_pop_injection_task(l1[j]).unwrap().id == t;
        assert(l2[j] == l1[j]);
        assert(tid_was_injected_before(l2, l2.len() as int, t));
      }
    }
  }
}

// E4 gives: a task never popped from injection can never have returned Ready
// (every PollTask was preceded by its PopInjection). This is what lets the
// single-bitmap ledger discharge BOTH freshness conjuncts of a pop.
pub proof fn not_injected_implies_not_ready(l: Log, tid: TID)
  requires
    inv_valid_task_polling(l),
    !tid_was_injected_before(l, l.len() as int, tid),
  ensures !tid_returned_ready_before(l, l.len() as int, tid),
{
  if tid_returned_ready_before(l, l.len() as int, tid) {
    let k = choose |k: int| #![trigger l[k]] 0 <= k < l.len() &&
      is_poll_task_at(l, k) &&
      get_poll_task_id(l[k]) == tid &&
      get_poll_result(l[k]) == PollResult::<()>::Ready(());
    assert(tid_was_injected_before(l, k, tid));
    let j = choose |j: int| #![trigger l[j]] 0 <= j < k &&
      is_pop_injection_at(l, j) &&
      get_pop_injection_task(l[j]).is_some() &&
      get_pop_injection_task(l[j]).unwrap().id == tid;
    assert(tid_was_injected_before(l, l.len() as int, tid));
  }
}

// Coupling survives any extension that contains no successful PopInjection.
pub proof fn ledger_recover_no_new_pops(ledger: crate::collections::TidLedger, l1: Log, l2: Log)
  requires
    ledger_matches_log(ledger, l1),
    l1.len() <= l2.len(),
    l1 =~= l2.subrange(0, l1.len() as int),
    forall |k: int| #![trigger l2[k]] l1.len() as int <= k < l2.len() ==>
      !is_pop_injection_at(l2, k) || !get_pop_injection_task(l2[k]).is_some(),
  ensures ledger_matches_log(ledger, l2),
{
  assert forall |t: TID| ledger.spec_has(t) <==>
    tid_was_injected_before(l2, l2.len() as int, t)
  by {
    if tid_was_injected_before(l2, l2.len() as int, t) {
      let j = choose |j: int| #![trigger l2[j]] 0 <= j < l2.len() &&
        is_pop_injection_at(l2, j) &&
        get_pop_injection_task(l2[j]).is_some() &&
        get_pop_injection_task(l2[j]).unwrap().id == t;
      assert(j < l1.len());
      assert(l1[j] == l2[j]);
      assert(tid_was_injected_before(l1, l1.len() as int, t));
    }
    if tid_was_injected_before(l1, l1.len() as int, t) {
      let j = choose |j: int| #![trigger l1[j]] 0 <= j < l1.len() &&
        is_pop_injection_at(l1, j) &&
        get_pop_injection_task(l1[j]).is_some() &&
        get_pop_injection_task(l1[j]).unwrap().id == t;
      assert(l2[j] == l1[j]);
      assert(tid_was_injected_before(l2, l2.len() as int, t));
    }
  }
}

// E4 survives any extension that contains no PollTask events.
pub proof fn e4_recover_no_new_polls(l1: Log, l2: Log)
  requires
    inv_valid_task_polling(l1),
    l1.len() <= l2.len(),
    l1 =~= l2.subrange(0, l1.len() as int),
    forall |k: int| #![trigger l2[k]] l1.len() as int <= k < l2.len() ==>
      !is_poll_task_at(l2, k),
  ensures inv_valid_task_polling(l2),
{
  assert forall |i: int| is_poll_task_at(l2, i) implies ({
    let tid = get_poll_task_id(l2[i]);
    let result = get_poll_result(l2[i]);
    tid_was_injected_before(l2, i, tid) &&
    (result == PollResult::<()>::Invalid <==> tid_is_invalid(l2, i, tid))
  }) by {
    assert(0 <= i < l2.len());
    assert(i < l1.len());
    assert(l2[i] == l1[i]);
    assert(is_poll_task_at(l1, i));
    let tid = get_poll_task_id(l1[i]);
    assert(tid_was_injected_before(l1, i, tid));
    assert(tid_was_injected_before(l2, i, tid)) by {
      let j = choose |j: int| #![trigger l1[j]] 0 <= j < i &&
        is_pop_injection_at(l1, j) &&
        get_pop_injection_task(l1[j]).is_some() &&
        get_pop_injection_task(l1[j]).unwrap().id == tid;
      assert(l2[j] == l1[j]);
    }
    tid_invalid_prefix_agree_upto(l1, l2, i, tid);
  }
}

// tid_is_invalid at i agrees between two logs that agree on [0, i).
pub proof fn tid_invalid_prefix_agree_upto(l1: Log, l2: Log, i: int, tid: TID)
  requires
    0 <= i <= l1.len(),
    i <= l2.len(),
    forall |k: int| 0 <= k < i ==> l2[k] == l1[k],
  ensures tid_is_invalid(l2, i, tid) <==> tid_is_invalid(l1, i, tid),
{
  assert(tid_was_injected_before(l2, i, tid) <==> tid_was_injected_before(l1, i, tid)) by {
    if tid_was_injected_before(l2, i, tid) {
      let j = choose |j: int| #![trigger l2[j]] 0 <= j < i &&
        is_pop_injection_at(l2, j) &&
        get_pop_injection_task(l2[j]).is_some() &&
        get_pop_injection_task(l2[j]).unwrap().id == tid;
      assert(l1[j] == l2[j]);
    }
    if tid_was_injected_before(l1, i, tid) {
      let j = choose |j: int| #![trigger l1[j]] 0 <= j < i &&
        is_pop_injection_at(l1, j) &&
        get_pop_injection_task(l1[j]).is_some() &&
        get_pop_injection_task(l1[j]).unwrap().id == tid;
      assert(l2[j] == l1[j]);
    }
  }
  assert(tid_returned_ready_before(l2, i, tid) <==> tid_returned_ready_before(l1, i, tid)) by {
    if tid_returned_ready_before(l2, i, tid) {
      let k = choose |k: int| #![trigger l2[k]] 0 <= k < i &&
        is_poll_task_at(l2, k) &&
        get_poll_task_id(l2[k]) == tid &&
        get_poll_result(l2[k]) == PollResult::<()>::Ready(());
      assert(l1[k] == l2[k]);
    }
    if tid_returned_ready_before(l1, i, tid) {
      let k = choose |k: int| #![trigger l1[k]] 0 <= k < i &&
        is_poll_task_at(l1, k) &&
        get_poll_task_id(l1[k]) == tid &&
        get_poll_result(l1[k]) == PollResult::<()>::Ready(());
      assert(l2[k] == l1[k]);
    }
  }
}

// E4 survives appending a PollTask event that itself satisfies E4's clauses.
pub proof fn e4_preserved_by_good_poll_push(l1: Log, l2: Log)
  requires
    inv_valid_task_polling(l1),
    l2.len() == l1.len() + 1,
    forall |k: int| 0 <= k < l1.len() ==> l2[k] == l1[k],
    is_poll_task_at(l2, l1.len() as int),
    tid_was_injected_before(l2, l1.len() as int, get_poll_task_id(l2[l1.len() as int])),
    (get_poll_result(l2[l1.len() as int]) == PollResult::<()>::Invalid <==>
      tid_is_invalid(l2, l1.len() as int, get_poll_task_id(l2[l1.len() as int]))),
  ensures inv_valid_task_polling(l2),
{
  assert forall |i: int| is_poll_task_at(l2, i) implies ({
    let tid = get_poll_task_id(l2[i]);
    let result = get_poll_result(l2[i]);
    tid_was_injected_before(l2, i, tid) &&
    (result == PollResult::<()>::Invalid <==> tid_is_invalid(l2, i, tid))
  }) by {
    if i < l1.len() {
      assert(l2[i] == l1[i]);
      assert(is_poll_task_at(l1, i));
      let tid = get_poll_task_id(l1[i]);
      assert(tid_was_injected_before(l1, i, tid));
      assert(tid_was_injected_before(l2, i, tid)) by {
        let j = choose |j: int| #![trigger l1[j]] 0 <= j < i &&
          is_pop_injection_at(l1, j) &&
          get_pop_injection_task(l1[j]).is_some() &&
          get_pop_injection_task(l1[j]).unwrap().id == tid;
        assert(l2[j] == l1[j]);
      }
      tid_invalid_prefix_agree_upto(l1, l2, i, tid);
    }
  }
}

// E4 survives appending any non-PollTask event.
pub proof fn e4_preserved_by_non_poll_push(l1: Log, l2: Log)
  requires
    inv_valid_task_polling(l1),
    l2.len() == l1.len() + 1,
    forall |k: int| 0 <= k < l1.len() ==> l2[k] == l1[k],
    !is_poll_task_at(l2, l1.len() as int),
  ensures inv_valid_task_polling(l2),
{
  assert forall |i: int| is_poll_task_at(l2, i) implies ({
    let tid = get_poll_task_id(l2[i]);
    let result = get_poll_result(l2[i]);
    tid_was_injected_before(l2, i, tid) &&
    (result == PollResult::<()>::Invalid <==> tid_is_invalid(l2, i, tid))
  }) by {
    assert(i < l1.len());
    assert(l2[i] == l1[i]);
    assert(is_poll_task_at(l1, i));
    let tid = get_poll_task_id(l1[i]);
    assert(tid_was_injected_before(l1, i, tid));
    assert forall |j: int| 0 <= j < i implies l2[j] == l1[j] by {}
    assert(tid_was_injected_before(l2, i, tid)) by {
      let j = choose |j: int| #![trigger l1[j]] 0 <= j < i &&
        is_pop_injection_at(l1, j) &&
        get_pop_injection_task(l1[j]).is_some() &&
        get_pop_injection_task(l1[j]).unwrap().id == tid;
      assert(l2[j] == l1[j]);
    }
    assert(tid_is_invalid(l2, i, tid) <==> tid_is_invalid(l1, i, tid)) by {
      tid_invalid_prefix_agree(l1, l2, i, tid);
    }
  }
}

// tid_is_invalid at position i only inspects the prefix before i.
pub proof fn tid_invalid_prefix_agree(l1: Log, l2: Log, i: int, tid: TID)
  requires
    0 <= i <= l1.len(),
    l2.len() >= l1.len(),
    forall |k: int| 0 <= k < l1.len() ==> l2[k] == l1[k],
  ensures tid_is_invalid(l2, i, tid) <==> tid_is_invalid(l1, i, tid),
{
  assert(tid_was_injected_before(l2, i, tid) <==> tid_was_injected_before(l1, i, tid)) by {
    if tid_was_injected_before(l2, i, tid) {
      let j = choose |j: int| #![trigger l2[j]] 0 <= j < i &&
        is_pop_injection_at(l2, j) &&
        get_pop_injection_task(l2[j]).is_some() &&
        get_pop_injection_task(l2[j]).unwrap().id == tid;
      assert(l1[j] == l2[j]);
    }
    if tid_was_injected_before(l1, i, tid) {
      let j = choose |j: int| #![trigger l1[j]] 0 <= j < i &&
        is_pop_injection_at(l1, j) &&
        get_pop_injection_task(l1[j]).is_some() &&
        get_pop_injection_task(l1[j]).unwrap().id == tid;
      assert(l2[j] == l1[j]);
    }
  }
  assert(tid_returned_ready_before(l2, i, tid) <==> tid_returned_ready_before(l1, i, tid)) by {
    if tid_returned_ready_before(l2, i, tid) {
      let k = choose |k: int| #![trigger l2[k]] 0 <= k < i &&
        is_poll_task_at(l2, k) &&
        get_poll_task_id(l2[k]) == tid &&
        get_poll_result(l2[k]) == PollResult::<()>::Ready(());
      assert(l1[k] == l2[k]);
    }
    if tid_returned_ready_before(l1, i, tid) {
      let k = choose |k: int| #![trigger l1[k]] 0 <= k < i &&
        is_poll_task_at(l1, k) &&
        get_poll_task_id(l1[k]) == tid &&
        get_poll_result(l1[k]) == PollResult::<()>::Ready(());
      assert(l2[k] == l1[k]);
    }
  }
}

}
