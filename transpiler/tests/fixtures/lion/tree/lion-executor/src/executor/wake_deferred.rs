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
  pub fn wake_deferred(&mut self)
    requires
      all_queue_tids_injected(old(self).log@, old(self).local_queue@),
      fifo_queue_matches(old(self).log@, old(self).local_queue@),
      slab_matches_log(old(self).task_slab@, old(self).log@),
      old(self).task_slab.wf(),
      ledger_matches_log(old(self).ledger, old(self).log@),
    ensures
      self.ledger == old(self).ledger,
      ledger_matches_log(self.ledger, self.log@),
      self.log@.len() == old(self).log@.len() + 1,
      old(self).log@ =~= self.log@.subrange(0, old(self).log@.len() as int),
      is_drain_deferred_at(self.log@, old(self).log@.len() as int),
      all_queue_tids_injected(self.log@, self.local_queue@),
      fifo_queue_matches(self.log@, self.local_queue@),
      self.local_queue@.len() >= old(self).local_queue@.len(),
      slab_matches_log(self.task_slab@, self.log@),
      self.task_slab.wf(),
  {
    let ghost pre_log = self.log@;
    let ghost pre_queue = self.local_queue@;
    self.drain_deferred_into_local();

    proof {
      fifo_queue_prefix_preserved(pre_log, self.log@, pre_log.len() as int);
      fifo_queue_after_drain(self.log@, self.log@.len() as int);
      // fifo_queue_at(self.log@, self.log@.len()) ==
      //   fifo_queue_at(pre_log, pre_log.len()) + get_drain_task_ids(self.log@[pre_log.len()])
      // == pre_queue + task_ids
      // The external_body ensures: exists |task_ids| log == old.log.push(Drain{task_ids}) && queue == old.queue + task_ids
      // So get_drain_task_ids(self.log@[pre_log.len()]) == task_ids
      // And self.local_queue@ == pre_queue + task_ids
      // Therefore fifo_queue_at(self.log@, self.log@.len()) == pre_queue + task_ids == self.local_queue@
      let task_ids = choose |task_ids: Seq<crate::types::TID>| {
        self.log@ == pre_log.push(
          ExecutorEvent::Outbound(OutboundCall::Drain {
            source: DrainSource::Deferred,
            task_ids: task_ids,
          })
        ) &&
        self.local_queue@ =~= pre_queue + task_ids &&
        all_queue_tids_injected(self.log@, self.local_queue@)
      };
      assert(get_drain_task_ids(self.log@[pre_log.len() as int]) == task_ids);
      assert(fifo_queue_at(self.log@, self.log@.len() as int) =~= pre_queue + task_ids);
      assert(self.local_queue@ =~= pre_queue + task_ids);

      assert(is_drain_deferred_at(self.log@, pre_log.len() as int));
      assert(!is_pop_injection(self.log@[pre_log.len() as int])) by {
        drain_not_poll_task(self.log@[pre_log.len() as int]);
      }
      slab_inv_preserved_by_non_slab_event(self.task_slab@, pre_log, self.log@);
    }
  }
}

} // end verus!
