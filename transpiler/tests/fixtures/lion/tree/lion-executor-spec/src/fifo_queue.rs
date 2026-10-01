use vstd::prelude::*;
use crate::types::TID;
use crate::log::*;
use crate::events::*;

verus! {

// FIFO queue model: the sequence of task IDs that have entered (via
// PopInjection-with-Some or Drain) but not yet been polled.

// Helper: remove first occurrence of tid from seq
pub open spec fn remove_first_occurrence(s: Seq<TID>, tid: TID) -> Seq<TID>
  decreases s.len()
{
  if s.len() == 0 {
    s
  } else if s[0] == tid {
    s.subrange(1, s.len() as int)
  } else {
    Seq::empty().push(s[0]) + remove_first_occurrence(s.subrange(1, s.len() as int), tid)
  }
}

// Compute the FIFO queue state at position i by scanning log [0, i)
pub open spec fn fifo_queue_at(l: Log, i: int) -> Seq<TID>
  decreases i
{
  if i <= 0 {
    Seq::empty()
  } else {
    let prev_queue = fifo_queue_at(l, i - 1);
    let e = l[i - 1];
    if is_pop_injection(e) && get_pop_injection_task(e).is_some() {
      // Task enters from Injection Queue
      prev_queue.push(get_pop_injection_task(e).unwrap().id)
    } else if is_drain(e) {
      // Tasks enter from Drain (ReactorWake/TaskWake/Deferred)
      let tids = get_drain_task_ids(e);
      prev_queue + tids
    } else if is_poll_task(e) {
      // Task is removed from queue (polled)
      let tid = get_poll_task_id(e);
      remove_first_occurrence(prev_queue, tid)
    } else {
      prev_queue
    }
  }
}

// Check if tid is at the head of the FIFO queue at position i
pub open spec fn is_fifo_head_at(l: Log, i: int, tid: TID) -> bool {
  let queue = fifo_queue_at(l, i);
  queue.len() > 0 && queue[0] == tid
}

pub open spec fn fifo_queue_matches(l: Log, queue: Seq<TID>) -> bool {
  fifo_queue_at(l, l.len() as int) == queue
}

pub proof fn remove_other_preserves_member(s: Seq<TID>, removed: TID, tid: TID, k: int)
  requires
    s.len() > 0,
    removed != tid,
    0 <= k < s.len(),
    s[k] == tid,
  ensures
    ({
      let result = remove_first_occurrence(s, removed);
      exists |j: int| 0 <= j < result.len() && result[j] == tid
    }),
  decreases s.len()
{
  let result = remove_first_occurrence(s, removed);
  if s[0] == removed {
    assert(result =~= s.subrange(1, s.len() as int));
    if k > 0 {
      assert(result[k - 1] == s[k]);
      assert(result[k - 1] == tid);
    } else {
      assert(s[0] == tid);
      assert(removed == tid);
    }
  } else {
    assert(result =~= Seq::empty().push(s[0]) + remove_first_occurrence(s.subrange(1, s.len() as int), removed));
    if k == 0 {
      assert(result[0] == s[0]);
      assert(result[0] == tid);
    } else {
      let sub = s.subrange(1, s.len() as int);
      assert(sub[k - 1] == s[k]);
      assert(sub[k - 1] == tid);
      if sub.len() > 0 {
        remove_other_preserves_member(sub, removed, tid, k - 1);
        let sub_result = remove_first_occurrence(sub, removed);
        let j_sub: int = choose |j: int| 0 <= j < sub_result.len() && sub_result[j] == tid;
        assert(result[j_sub + 1] == sub_result[j_sub]);
      }
    }
  }
}

}
