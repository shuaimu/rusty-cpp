use vstd::prelude::*;
use crate::events::*;
use crate::log::Log;
#[cfg(verus_keep_ghost)]
use crate::log::count_pop_injection_between;
use crate::types::*;

verus! {

// Delivery guarantee: if the schedule is non-empty and the executor has popped at least
// `q.len()` times, then all of `q` has been delivered (no None-pop starved the schedule).
// This is the "committed & available" assumption that turns pop-events into deliveries.
// VACUOUS when q is empty — so it adds nothing to existing (empty-schedule) states.
pub open spec fn pops_deliver_schedule(l: Log, q: Seq<TaskView>) -> bool {
  q.len() > 0 ==> (
    // delivered tasks so far follow the schedule ...
    is_task_prefix(injected_tasks(l), q) &&
    // ... and once popped >= q.len() times, all of q is delivered (no None starvation).
    (count_pop_injection_between(l, 0, l.len() as int) >= q.len()
      ==> injected_tasks(l).len() >= q.len())
  )
}

// Minimal logical model of the (external, otherwise-unmodelled) Injection Queue.
//
// The Injection Queue is invisible to the executor: PopInjection returns whatever the
// environment supplies, so a forall-over-arbitrary-extension `arrival` is unsatisfiable
// (a None-pop extension always exists). Here we model the environment's committed
// delivery order as a ghost schedule `q: Seq<TaskView>` and link the observed pops to it,
// which makes `arrival` a satisfiable single-state predicate.

// The tasks actually delivered by PopInjection so far (the Some-results), in order.
pub open spec fn injected_tasks(l: Log) -> Seq<TaskView>
  decreases l.len()
{
  if l.len() == 0 {
    Seq::<TaskView>::empty()
  } else {
    let prev = injected_tasks(l.subrange(0, (l.len() - 1) as int));
    let last = l[(l.len() - 1) as int];
    if is_pop_injection(last) && get_pop_injection_task(last).is_some() {
      prev.push(get_pop_injection_task(last).unwrap())
    } else {
      prev
    }
  }
}

// Prefix relation on task sequences.
pub open spec fn is_task_prefix(a: Seq<TaskView>, b: Seq<TaskView>) -> bool {
  a.len() <= b.len() && a =~= b.subrange(0, a.len() as int)
}

// The observed pops of `l` are consistent with the committed injection schedule `q`:
// the tasks delivered so far form a prefix of `q`.
pub open spec fn pop_follows_schedule(l: Log, q: Seq<TaskView>) -> bool {
  is_task_prefix(injected_tasks(l), q)
}

}
