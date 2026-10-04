use crate::types::{TaskId, PollResult, Task, TID, TaskView};
use crate::spec::log::*;
use crate::proof::invariants::*;
use crate::proof::helpers::*;
use crate::tls;
use super::Executor;
use vstd::prelude::*;

verus! {

#[verifier::external_body]
fn clear_task_notified(task_id: TaskId) {
  tls::clear_notified(task_id);
}

impl Executor {
  pub fn poll_task(&mut self, task_id: TaskId)
    requires
      slab_matches_log(old(self).task_slab@, old(self).log@),
      old(self).task_slab.wf(),
    ensures
      self.log@.len() == old(self).log@.len() + 1,
      old(self).log@ =~= self.log@.subrange(0, old(self).log@.len() as int),
      is_poll_task_at(self.log@, old(self).log@.len() as int),
      get_poll_task_id(self.log@[old(self).log@.len() as int]) == task_id@,
      !is_tick_begin(self.log@[old(self).log@.len() as int]),
      !is_tick_end(self.log@[old(self).log@.len() as int]),
      !is_park(self.log@[old(self).log@.len() as int]),
      self.local_queue@ =~= old(self).local_queue@,
      slab_matches_log(self.task_slab@, self.log@),
      self.task_slab.wf(),
      self.ledger == old(self).ledger,
      ({
        let pos = old(self).log@.len() as int;
        let result = get_poll_result(self.log@[pos]);
        result == PollResult::<()>::Invalid <==>
          tid_is_invalid(self.log@, pos, task_id@)
      }),
  {
    clear_task_notified(task_id);
    let ghost old_slab = self.task_slab@;
    let ghost old_log = self.log@;
    let ghost tid = task_id@;
    match self.task_slab.remove(task_id.0) {
      Some(task) => {
        proof {
          assert(old_slab.contains_key(tid));
          assert(tid_was_injected_before(old_log, old_log.len() as int, tid));
          assert(!tid_returned_ready_before(old_log, old_log.len() as int, tid));
        }
        let (result, task_back) = self.poll_task_action(task_id, Some(task));
        let ghost post_log = self.log@;
        let ghost pos = old_log.len() as int;
        proof {
          assert(!tid_is_invalid(post_log, pos, tid)) by {
            assert(tid_was_injected_before(old_log, old_log.len() as int, tid));
            e4_witness_survives_extension(old_log, post_log, pos, tid);
            assert(!tid_returned_ready_before(post_log, pos, tid)) by {
              if tid_returned_ready_before(post_log, pos, tid) {
                let k = choose |k: int| 0 <= k < pos &&
                  is_poll_task_at(post_log, k) && get_poll_task_id(post_log[k]) == tid &&
                  get_poll_result(post_log[k]) == PollResult::<()>::Ready(());
                assert(post_log[k] == old_log[k]);
                assert(is_poll_task_at(old_log, k));
                assert(get_poll_task_id(old_log[k]) == tid);
                assert(get_poll_result(old_log[k]) == PollResult::<()>::Ready(()));
                assert(tid_returned_ready_before(old_log, old_log.len() as int, tid));
              }
            }
          }
        }
        match result {
          PollResult::Ready(()) => {
            proof {
              slab_inv_preserved_by_poll_ready(old_slab, old_log, post_log, tid);
            }
          }
          _ => {
            let task_to_insert = task_back.unwrap();
            self.task_slab.insert(task_id.0, task_to_insert);
            proof {
              assert(get_poll_result(post_log[pos]) == PollResult::<()>::Pending);
              slab_inv_preserved_by_poll_pending(old_slab, old_log, post_log, tid, self.task_slab@[tid]);
            }
          }
        }
      }
      None => {
        proof {
          assert(!old_slab.contains_key(tid));
          assert(tid_is_invalid(old_log, old_log.len() as int, tid)) by {
            if tid_was_injected_before(old_log, old_log.len() as int, tid) &&
               !tid_returned_ready_before(old_log, old_log.len() as int, tid) {
              assert(old_slab.contains_key(tid));
            }
          }
        }
        self.poll_task_invalid_action(task_id);
        proof {
          let pos = old_log.len() as int;
          tid_invalid_survives_one_step(old_log, self.log@, tid);
          slab_inv_preserved_by_poll_invalid(self.task_slab@, old_log, self.log@, tid);
        }
      }
    }
  }
}

} // end verus!
