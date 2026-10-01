use crate::types::{TaskId, TaskView, TID};
use crate::spec::log::*;
use crate::spec::fifo_queue::*;
use crate::proof::invariants::*;
use crate::proof::helpers::*;
use crate::proof::fifo_helpers::*;
use super::Executor;
use vstd::prelude::*;

verus! {

impl Executor {
  pub fn next_task(&mut self) -> (result: Option<TaskId>)
    requires
      all_queue_tids_injected(old(self).log@, old(self).local_queue@),
      fifo_queue_matches(old(self).log@, old(self).local_queue@),
      slab_matches_log(old(self).task_slab@, old(self).log@),
      old(self).task_slab.wf(),
      ledger_matches_log(old(self).ledger, old(self).log@),
      inv_valid_task_polling(old(self).log@),
    ensures
      ledger_matches_log(self.ledger, self.log@),
      inv_valid_task_polling(self.log@),
      old(self).log@.len() <= self.log@.len(),
      old(self).log@ =~= self.log@.subrange(0, old(self).log@.len() as int),
      forall |k: int| old(self).log@.len() as int <= k < self.log@.len() as int ==>
        !is_tick_begin(self.log@[k]) && !is_tick_end(self.log@[k]) &&
        !is_park(self.log@[k]) && !is_poll_task(self.log@[k]),
      all_queue_tids_injected(self.log@, self.local_queue@),
      slab_matches_log(self.task_slab@, self.log@),
      self.task_slab.wf(),
      result.is_some() ==>
        tid_was_injected_before(self.log@, self.log@.len() as int, result.unwrap()@),
      old(self).local_queue@.len() > 0 ==> result.is_some(),
      result.is_none() ==>
        forall |k: int| old(self).log@.len() as int <= k < self.log@.len() as int &&
          is_pop_injection_at(self.log@, k) ==>
          !get_pop_injection_task(self.log@[k]).is_some(),
      result.is_some() ==> is_fifo_head_at(self.log@, self.log@.len() as int, result.unwrap()@),
      result.is_some() ==>
        fifo_queue_at(self.log@, self.log@.len() as int) ==
          Seq::empty().push(result.unwrap()@) + self.local_queue@,
      result.is_none() ==> fifo_queue_matches(self.log@, self.local_queue@),
  {
    // Path 1: Pop directly from local queue
    if let Some(task_id) = self.local_queue.pop_front() {
      proof {
        assert(task_id@ == old(self).local_queue@[0]);
        let queue = fifo_queue_at(self.log@, self.log@.len() as int);
        assert(queue == old(self).local_queue@);
        assert(queue.len() > 0);
        assert(queue[0] == task_id@);
        assert(is_fifo_head_at(self.log@, self.log@.len() as int, task_id@));
        assert(self.local_queue@ =~= old(self).local_queue@.subrange(1, old(self).local_queue@.len() as int));
        assert(queue =~= Seq::empty().push(task_id@) + self.local_queue@);
      }
      return Some(task_id);
    }

    // Queue was empty since pop_front returned None
    let ghost pre_drain = self.log@;
    let ghost pre_drain_queue = self.local_queue@;

    self.drain_reactor_ready_into_local();
    let ghost mid = self.log@;
    let ghost mid_queue = self.local_queue@;
    proof {
      slab_inv_preserved_by_non_slab_event(self.task_slab@, pre_drain, mid);
      assert(!is_poll_task_at(mid, pre_drain.len() as int));
      e4_recover_no_new_polls(pre_drain, mid);
    }

    self.drain_task_ready_into_local();
    let ghost after_drains = self.log@;
    let ghost after_drains_queue = self.local_queue@;
    proof {
      slab_inv_preserved_by_non_slab_event(self.task_slab@, mid, self.log@);
      assert(!is_poll_task_at(self.log@, mid.len() as int));
      e4_recover_no_new_polls(mid, self.log@);
    }

    proof {
      prefix_transitive(pre_drain, mid, self.log@);

      // pre_drain == old(self).log@, old(self).local_queue@ was empty
      // So fifo_queue_at(pre_drain, pre_drain.len()) == Seq::empty()

      // Reactor drain: fifo_queue tracks the drain event
      fifo_queue_prefix_preserved(pre_drain, mid, pre_drain.len() as int);
      fifo_queue_after_drain(mid, mid.len() as int);
      let reactor_tids = choose |task_ids: Seq<TID>| {
        mid == pre_drain.push(
          ExecutorEvent::Outbound(OutboundCall::Drain {
            source: DrainSource::ReactorWake,
            task_ids: task_ids,
          })
        ) &&
        mid_queue =~= pre_drain_queue + task_ids &&
        all_queue_tids_injected(mid, mid_queue)
      };
      assert(get_drain_task_ids(mid[pre_drain.len() as int]) == reactor_tids);
      assert(fifo_queue_at(mid, mid.len() as int) =~= pre_drain_queue + reactor_tids);
      assert(mid_queue =~= pre_drain_queue + reactor_tids);

      // Task drain: fifo_queue tracks the drain event
      fifo_queue_prefix_preserved(mid, self.log@, mid.len() as int);
      fifo_queue_after_drain(self.log@, self.log@.len() as int);
      let task_tids = choose |task_ids: Seq<TID>| {
        self.log@ == mid.push(
          ExecutorEvent::Outbound(OutboundCall::Drain {
            source: DrainSource::TaskWake,
            task_ids: task_ids,
          })
        ) &&
        after_drains_queue =~= mid_queue + task_ids &&
        all_queue_tids_injected(self.log@, after_drains_queue)
      };
      assert(get_drain_task_ids(self.log@[mid.len() as int]) == task_tids);
      assert(fifo_queue_at(self.log@, self.log@.len() as int) =~= mid_queue + task_tids);
      assert(after_drains_queue =~= mid_queue + task_tids);
      // fifo_queue_matches(after_drains, after_drains_queue) established
    }

    // Path 2: Pop after drain
    if let Some(task_id) = self.local_queue.pop_front() {
      proof {
        // pop_front doesn't change log, so fifo_queue_at is unchanged
        assert(after_drains_queue =~= fifo_queue_at(self.log@, self.log@.len() as int));
        assert(task_id@ == after_drains_queue[0]);
        assert(is_fifo_head_at(self.log@, self.log@.len() as int, task_id@));
        assert(self.local_queue@ =~= after_drains_queue.subrange(1, after_drains_queue.len() as int));
        assert(after_drains_queue =~= Seq::empty().push(task_id@) + self.local_queue@);
      }
      return Some(task_id);
    }

    // Queue is still empty after drain. pop_front returned None → after_drains_queue was empty.
    // self.log@ == after_drains (pop_front doesn't change log)
    // fifo_queue_at(self.log@, self.log@.len()) == after_drains_queue == Seq::empty() == self.local_queue@

    let ghost pre_inj_log = self.log@;
    let ghost pre_inj_slab = self.task_slab@;
    let pop_result = self.pop_injection_action();
    proof {
      assert(!is_poll_task_at(self.log@, pre_inj_log.len() as int));
      e4_recover_no_new_polls(pre_inj_log, self.log@);
    }
    if let Some(task) = pop_result {
      let task_id = task.id();
      let ghost tid = task_id@;
      let ghost task_view = task@;

      self.task_slab.insert(task_id.0, task);
      self.local_queue.push_back(task_id);

      proof {
        let inj_pos = pre_inj_log.len() as int;
        assert(is_pop_injection_at(self.log@, inj_pos));
        assert(get_pop_injection_task(self.log@[inj_pos]).is_some());
        assert(task_view.id == tid);
        assert(get_pop_injection_task(self.log@[inj_pos]).unwrap().id == tid);

        slab_inv_preserved_by_injection(pre_inj_slab, pre_inj_log, self.log@, tid, task_view);

        assert(self.local_queue@.len() == 1);
        assert(self.local_queue@[0] == tid);
        assert forall |k: int| 0 <= k < self.local_queue@.len() as int implies
          tid_was_injected_before(self.log@, self.log@.len() as int, self.local_queue@[k])
        by {
          assert(self.local_queue@[k] == tid);
          assert(0 <= inj_pos);
          assert(inj_pos < self.log@.len());
          assert(get_pop_injection_task(self.log@[inj_pos]).unwrap().id == self.local_queue@[k]);
        }

        prefix_transitive(pre_drain, pre_inj_log, self.log@);

        fifo_queue_prefix_preserved(pre_inj_log, self.log@, pre_inj_log.len() as int);
        fifo_queue_after_push_injection(self.log@, self.log@.len() as int, tid);
        assert(fifo_queue_at(self.log@, self.log@.len() as int) =~= Seq::<TID>::empty().push(tid));
        assert(self.local_queue@ =~= Seq::<TID>::empty().push(tid));
      }

      let result = self.local_queue.pop_front();
      proof {
        assert(result.is_some());
        assert(result.unwrap()@ == tid);
        assert(fifo_queue_at(self.log@, self.log@.len() as int) =~= Seq::<TID>::empty().push(tid));
        assert(is_fifo_head_at(self.log@, self.log@.len() as int, result.unwrap()@));
        assert(self.local_queue@ =~= Seq::<TID>::empty());
        assert(Seq::<TID>::empty().push(tid) =~=
          Seq::empty().push(result.unwrap()@) + self.local_queue@);
      }
      return result;
    }

    proof {
      prefix_transitive(pre_drain, pre_inj_log, self.log@);
      prefix_transitive(mid, pre_inj_log, self.log@);
      assert forall |k: int| pre_drain.len() as int <= k < self.log@.len() as int &&
        is_pop_injection_at(self.log@, k) implies
        !get_pop_injection_task(self.log@[k]).is_some()
      by {
        if k == pre_inj_log.len() as int {
        } else if k < mid.len() as int {
          prefix_preserves_at(mid, self.log@, k);
        } else {
          prefix_preserves_at(pre_inj_log, self.log@, k);
        }
      }

      fifo_queue_prefix_preserved(pre_inj_log, self.log@, pre_inj_log.len() as int);
      fifo_queue_after_failed_injection(self.log@, self.log@.len() as int);

      slab_inv_preserved_by_non_slab_event(self.task_slab@, pre_inj_log, self.log@);
    }

    None
  }
}

} // end verus!
