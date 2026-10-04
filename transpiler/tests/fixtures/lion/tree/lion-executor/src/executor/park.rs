use crate::types::TaskId;
use crate::spec::log::*;
use crate::spec::fifo_queue::*;
use crate::proof::invariants::*;
use crate::proof::helpers::*;
use crate::proof::fifo_helpers::*;
use super::Executor;
use vstd::prelude::*;

verus! {

impl Executor {
  pub fn park(&mut self)
    requires
      all_queue_tids_injected(old(self).log@, old(self).local_queue@),
      fifo_queue_matches(old(self).log@, old(self).local_queue@),
      slab_matches_log(old(self).task_slab@, old(self).log@),
      old(self).task_slab.wf(),
      ledger_matches_log(old(self).ledger, old(self).log@),
    ensures
      self.ledger == old(self).ledger,
      ledger_matches_log(self.ledger, self.log@),
      self.log@.len() == old(self).log@.len() + 3,
      old(self).log@ =~= self.log@.subrange(0, old(self).log@.len() as int),
      is_park_at(self.log@, old(self).log@.len() as int),
      is_drain_reactor_wake_at(self.log@, (old(self).log@.len() + 1) as int),
      is_drain_task_wake_at(self.log@, (old(self).log@.len() + 2) as int),
      all_queue_tids_injected(self.log@, self.local_queue@),
      fifo_queue_matches(self.log@, self.local_queue@),
      slab_matches_log(self.task_slab@, self.log@),
      self.task_slab.wf(),
  {
    let has_deferred = self.has_deferred_action();
    let has_reactor_ready = self.has_reactor_ready_action();
    self.reset_and_drain_cross_thread_action();
    let has_task_ready = self.has_task_ready_action();
    let has_local_tasks = self.local_queue.len() > 0;

    let block_on_yielded = self.take_block_on_yielded_action();
    let require_timeout = !block_on_yielded && !has_deferred && !has_reactor_ready && !has_task_ready && !has_local_tasks;

    let ghost pre_park_log = self.log@;
    let ghost pre_park_queue = self.local_queue@;
    self.park_action(require_timeout);

    proof {
      ledger_preserved_by_non_pop(self.ledger, pre_park_log, self.log@);
      assert forall |k: int| 0 <= k < self.local_queue@.len() as int implies
        tid_was_injected_before(self.log@, self.log@.len() as int, self.local_queue@[k])
      by {
        assert(self.local_queue@[k] == pre_park_queue[k]);
        assert(tid_was_injected_before(pre_park_log, pre_park_log.len() as int, pre_park_queue[k]));
        let j = choose |j: int| 0 <= j < pre_park_log.len() &&
          is_pop_injection_at(pre_park_log, j) &&
          get_pop_injection_task(pre_park_log[j]).is_some() &&
          get_pop_injection_task(pre_park_log[j]).unwrap().id == pre_park_queue[k];
        assert(self.log@[j] == pre_park_log[j]);
        assert(is_pop_injection_at(self.log@, j));
      }

      fifo_queue_prefix_preserved(pre_park_log, self.log@, pre_park_log.len() as int);
      fifo_queue_noop_event(self.log@, self.log@.len() as int);

      slab_inv_preserved_by_non_slab_event(self.task_slab@, pre_park_log, self.log@);
    }

    let ghost after_park_log = self.log@;
    let ghost after_park_queue = self.local_queue@;
    self.drain_reactor_ready_into_local();
    let ghost after_reactor_log = self.log@;
    let ghost after_reactor_queue = self.local_queue@;

    proof {
      fifo_queue_prefix_preserved(after_park_log, after_reactor_log, after_park_log.len() as int);
      fifo_queue_after_drain(after_reactor_log, after_reactor_log.len() as int);
      let reactor_tids = choose |task_ids: Seq<crate::types::TID>| {
        after_reactor_log == after_park_log.push(
          ExecutorEvent::Outbound(OutboundCall::Drain {
            source: DrainSource::ReactorWake,
            task_ids: task_ids,
          })
        ) &&
        after_reactor_queue =~= after_park_queue + task_ids &&
        all_queue_tids_injected(after_reactor_log, after_reactor_queue)
      };
      assert(get_drain_task_ids(after_reactor_log[after_park_log.len() as int]) == reactor_tids);
      assert(fifo_queue_at(after_reactor_log, after_reactor_log.len() as int) =~= after_park_queue + reactor_tids);
      assert(after_reactor_queue =~= after_park_queue + reactor_tids);

      slab_inv_preserved_by_non_slab_event(self.task_slab@, after_park_log, after_reactor_log);
    }

    self.drain_task_ready_into_local();

    proof {
      fifo_queue_prefix_preserved(after_reactor_log, self.log@, after_reactor_log.len() as int);
      fifo_queue_after_drain(self.log@, self.log@.len() as int);
      let task_tids = choose |task_ids: Seq<crate::types::TID>| {
        self.log@ == after_reactor_log.push(
          ExecutorEvent::Outbound(OutboundCall::Drain {
            source: DrainSource::TaskWake,
            task_ids: task_ids,
          })
        ) &&
        self.local_queue@ =~= after_reactor_queue + task_ids &&
        all_queue_tids_injected(self.log@, self.local_queue@)
      };
      assert(get_drain_task_ids(self.log@[after_reactor_log.len() as int]) == task_tids);
      assert(fifo_queue_at(self.log@, self.log@.len() as int) =~= after_reactor_queue + task_tids);
      assert(self.local_queue@ =~= after_reactor_queue + task_tids);

      slab_inv_preserved_by_non_slab_event(self.task_slab@, after_reactor_log, self.log@);
    }
  }
}

} // end verus!
