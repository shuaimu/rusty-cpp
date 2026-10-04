use super::Executor;
use crate::spec::log::*;
use crate::spec::fifo_queue::*;
use crate::proof::invariants::*;
use crate::proof::preservation::*;
use crate::proof::helpers::*;
use crate::proof::fifo_helpers::*;
use crate::types::PollResult;
use vstd::prelude::*;

verus! {

impl Executor {
  // TERMINATION (unproven — the only exec loop in the repo exempted from the
  // decreases check; disclosed in TCB_and_limitations.md §2): the drain loop
  // below runs until the injection channel is empty. Termination rests on the
  // arrival set at each tick being finite — the same finite-arrival modeling
  // the liveness proof's injection schedule makes explicit. An unbounded
  // producer that outpaces the drain forever could starve this loop.
  #[verifier::exec_allows_no_decreases_clause]
  pub fn pop_injection(&mut self)
    requires
      all_queue_tids_injected(old(self).log@, old(self).local_queue@),
      fifo_queue_matches(old(self).log@, old(self).local_queue@),
      slab_matches_log(old(self).task_slab@, old(self).log@),
      old(self).task_slab.wf(),
      ledger_matches_log(old(self).ledger, old(self).log@),
      inv_valid_task_polling(old(self).log@),
    ensures
      self.log@.len() >= old(self).log@.len() + 1,
      old(self).log@ =~= self.log@.subrange(0, old(self).log@.len() as int),
      forall |k: int| old(self).log@.len() as int <= k < self.log@.len() as int ==>
        is_pop_injection(self.log@[k]) &&
        !is_tick_begin(self.log@[k]) && !is_tick_end(self.log@[k]) &&
        !is_park(self.log@[k]),
      all_queue_tids_injected(self.log@, self.local_queue@),
      self.local_queue@.len() >= old(self).local_queue@.len(),
      self.local_queue@.len() == old(self).local_queue@.len() ==>
        forall |k: int| old(self).log@.len() as int <= k < self.log@.len() as int &&
          is_pop_injection_at(self.log@, k) ==> !get_pop_injection_task(self.log@[k]).is_some(),
      fifo_queue_matches(self.log@, self.local_queue@),
      slab_matches_log(self.task_slab@, self.log@),
      self.task_slab.wf(),
      ledger_matches_log(self.ledger, self.log@),
      inv_valid_task_polling(self.log@),
  {
    let ghost entry_log = self.log@;
    let ghost entry_len = self.log@.len();
    let ghost entry_queue = self.local_queue@;

    let ghost pre_slab = self.task_slab@;
    let task_opt = self.pop_injection_action();
    proof {
      assert(entry_log =~= self.log@.subrange(0, entry_len as int));
      assert(!is_poll_task_at(self.log@, entry_len as int));
      e4_recover_no_new_polls(entry_log, self.log@);
    }
    if let Some(task) = task_opt {
      let task_id = task.id();
      let ghost tid = task_id@;
      let ghost task_view = task@;
      self.task_slab.insert(task_id.0, task);
      self.local_queue.push_back(task_id);

      proof {
        let inj_pos = entry_len as int;
        assert(is_pop_injection_at(self.log@, inj_pos));
        assert(get_pop_injection_task(self.log@[inj_pos]).is_some());
        assert(task_view.id == tid);
        assert(get_pop_injection_task(self.log@[inj_pos]).unwrap().id == tid);

        slab_inv_preserved_by_injection(pre_slab, entry_log, self.log@, tid, task_view);

        assert forall |k: int| 0 <= k < self.local_queue@.len() as int implies
          tid_was_injected_before(self.log@, self.log@.len() as int, self.local_queue@[k])
        by {
          if k < entry_queue.len() as int {
            assert(self.local_queue@[k] == entry_queue[k]);
            assert(tid_was_injected_before(entry_log, entry_log.len() as int, entry_queue[k]));
            let j = choose |j: int| 0 <= j < entry_log.len() as int &&
              is_pop_injection_at(entry_log, j) &&
              get_pop_injection_task(entry_log[j]).is_some() &&
              get_pop_injection_task(entry_log[j]).unwrap().id == entry_queue[k];
            assert(self.log@[j] == entry_log[j]);
            assert(is_pop_injection_at(self.log@, j));
          } else {
            assert(self.local_queue@[k] == tid);
            assert(get_pop_injection_task(self.log@[inj_pos]).unwrap().id == self.local_queue@[k]);
          }
        }

        fifo_queue_prefix_preserved(entry_log, self.log@, entry_len as int);
        fifo_queue_after_push_injection(self.log@, self.log@.len() as int, tid);
        assert(fifo_queue_at(self.log@, self.log@.len() as int) =~= entry_queue.push(tid));
        assert(self.local_queue@ =~= entry_queue.push(tid));
      }
    } else {
      proof {
        data_inv_preserved_by_extension(entry_log, self.log@, self.local_queue@);
        assert forall |k: int| entry_len as int <= k < self.log@.len() as int &&
          is_pop_injection_at(self.log@, k) implies
          !get_pop_injection_task(self.log@[k]).is_some()
        by {}

        fifo_queue_prefix_preserved(entry_log, self.log@, entry_len as int);
        fifo_queue_after_failed_injection(self.log@, self.log@.len() as int);
        assert(fifo_queue_at(self.log@, self.log@.len() as int) =~= entry_queue);

        slab_inv_preserved_by_non_slab_event(self.task_slab@, entry_log, self.log@);
      }
      return;
    }

    loop
      invariant
        self.log@.len() >= entry_len + 1,
        entry_log =~= self.log@.subrange(0, entry_len as int),
        forall |k: int| entry_len as int <= k < self.log@.len() as int ==>
          is_pop_injection(self.log@[k]) &&
          !is_tick_begin(self.log@[k]) && !is_tick_end(self.log@[k]) &&
          !is_park(self.log@[k]),
        all_queue_tids_injected(self.log@, self.local_queue@),
        self.local_queue@.len() >= entry_queue.len() as int + 1,
        fifo_queue_matches(self.log@, self.local_queue@),
        slab_matches_log(self.task_slab@, self.log@),
        self.task_slab.wf(),
        ledger_matches_log(self.ledger, self.log@),
        inv_valid_task_polling(self.log@),
    {
      let ghost pre_log = self.log@;
      let ghost pre_queue = self.local_queue@;
      let ghost pre_slab2 = self.task_slab@;
      let task_opt = self.pop_injection_action();
      proof {
        prefix_transitive(entry_log, pre_log, self.log@);
        assert(pre_log =~= self.log@.subrange(0, pre_log.len() as int));
        assert(!is_poll_task_at(self.log@, pre_log.len() as int));
        e4_recover_no_new_polls(pre_log, self.log@);
      }
      if let Some(task) = task_opt {
        let task_id = task.id();
        let ghost tid = task_id@;
        let ghost task_view = task@;
        self.task_slab.insert(task_id.0, task);
        self.local_queue.push_back(task_id);

        proof {
          let inj_pos = pre_log.len() as int;
          assert(is_pop_injection_at(self.log@, inj_pos));
          assert(get_pop_injection_task(self.log@[inj_pos]).is_some());
          assert(task_view.id == tid);
          assert(get_pop_injection_task(self.log@[inj_pos]).unwrap().id == tid);

          slab_inv_preserved_by_injection(pre_slab2, pre_log, self.log@, tid, task_view);

          assert forall |k: int| 0 <= k < self.local_queue@.len() as int implies
            tid_was_injected_before(self.log@, self.log@.len() as int, self.local_queue@[k])
          by {
            if k < pre_queue.len() as int {
              assert(self.local_queue@[k] == pre_queue[k]);
              assert(tid_was_injected_before(pre_log, pre_log.len() as int, pre_queue[k]));
              let j = choose |j: int| 0 <= j < pre_log.len() as int &&
                is_pop_injection_at(pre_log, j) &&
                get_pop_injection_task(pre_log[j]).is_some() &&
                get_pop_injection_task(pre_log[j]).unwrap().id == pre_queue[k];
              assert(self.log@[j] == pre_log[j]);
              assert(is_pop_injection_at(self.log@, j));
            } else {
              assert(self.local_queue@[k] == tid);
              assert(get_pop_injection_task(self.log@[inj_pos]).unwrap().id == self.local_queue@[k]);
            }
          }

          fifo_queue_prefix_preserved(pre_log, self.log@, pre_log.len() as int);
          fifo_queue_after_push_injection(self.log@, self.log@.len() as int, tid);
          assert(fifo_queue_at(self.log@, self.log@.len() as int) =~= pre_queue.push(tid));
          assert(self.local_queue@ =~= pre_queue.push(tid));
        }
      } else {
        proof {
          data_inv_preserved_by_extension(pre_log, self.log@, self.local_queue@);
          fifo_queue_prefix_preserved(pre_log, self.log@, pre_log.len() as int);
          fifo_queue_after_failed_injection(self.log@, self.log@.len() as int);
          assert(fifo_queue_at(self.log@, self.log@.len() as int) =~= pre_queue);
          slab_inv_preserved_by_non_slab_event(self.task_slab@, pre_log, self.log@);
        }
        break;
      }
    }
  }

  #[verifier::rlimit(30)]
  fn poll_loop(&mut self, event_interval: usize, count: &mut usize)
    requires
      *old(count) <= event_interval,
      all_queue_tids_injected(old(self).log@, old(self).local_queue@),
      fifo_queue_matches(old(self).log@, old(self).local_queue@),
      slab_matches_log(old(self).task_slab@, old(self).log@),
      old(self).task_slab.wf(),
      ledger_matches_log(old(self).ledger, old(self).log@),
      inv_valid_task_polling(old(self).log@),
    ensures
      ledger_matches_log(self.ledger, self.log@),
      inv_valid_task_polling(self.log@),
      *count <= event_interval,
      old(self).log@.len() <= self.log@.len(),
      old(self).log@ =~= self.log@.subrange(0, old(self).log@.len() as int),
      forall |k: int| old(self).log@.len() as int <= k < self.log@.len() as int ==>
        !is_tick_begin(self.log@[k]) && !is_tick_end(self.log@[k]) &&
        !is_park(self.log@[k]),
      all_queue_tids_injected(self.log@, self.local_queue@),
      fifo_queue_matches(self.log@, self.local_queue@),
      slab_matches_log(self.task_slab@, self.log@),
      self.task_slab.wf(),
      forall |i: int| old(self).log@.len() as int <= i < self.log@.len() as int &&
        is_poll_task_at(self.log@, i) ==> {
        let tid = get_poll_task_id(self.log@[i]);
        let result = get_poll_result(self.log@[i]);
        tid_was_injected_before(self.log@, i, tid) &&
        (result == PollResult::<()>::Invalid <==> tid_is_invalid(self.log@, i, tid))
      },
      forall |i: int| old(self).log@.len() as int <= i < self.log@.len() as int &&
        is_poll_task_at(self.log@, i) ==>
        is_fifo_head_at(self.log@, i, get_poll_task_id(self.log@[i])),
      old(self).local_queue@.len() > 0 && *old(count) < event_interval ==>
        (exists |q: int| #![auto] old(self).log@.len() as int <= q < self.log@.len() as int &&
          is_poll_task_at(self.log@, q)),
      forall |p: int| old(self).log@.len() as int <= p < self.log@.len() as int &&
        is_pop_injection_at(self.log@, p) && get_pop_injection_task(self.log@[p]).is_some() ==>
        (exists |q: int| #![auto] old(self).log@.len() as int <= q < self.log@.len() as int &&
          is_poll_task_at(self.log@, q)),
  {
    let ghost entry_log = self.log@;
    let ghost entry_len = self.log@.len();
    let ghost mut poll_witness: int = 0;

    if *count >= event_interval {
      return;
    }

    let ghost pre_first = self.log@;
    let ghost mut first_mid: Log = self.log@;
    let ghost mut first_poll_pos: int = 0;
    match self.next_task() {
      Some(task_id) => {
        proof {
          first_mid = self.log@;
          first_poll_pos = self.log@.len() as int;
          assert(is_fifo_head_at(self.log@, self.log@.len() as int, task_id@));
        }
        let ghost mid = self.log@;
        let ghost poll_pos = mid.len() as int;
        let ghost task_tid = task_id@;

        self.poll_task(task_id);
        proof {
          prefix_transitive(entry_log, pre_first, mid);
          prefix_transitive(entry_log, mid, self.log@);
          data_inv_preserved_by_extension(mid, self.log@, self.local_queue@);
          poll_witness = poll_pos;
          assert(is_poll_task_at(self.log@, poll_witness));
          assert(get_poll_task_id(self.log@[poll_pos]) == task_tid);
          e4_witness_survives_extension(mid, self.log@, poll_pos, task_tid);
          ledger_preserved_by_non_pop(self.ledger, mid, self.log@);
          e4_preserved_by_good_poll_push(mid, self.log@);

          fifo_queue_prefix_preserved(mid, self.log@, poll_pos);
          assert(is_fifo_head_at(self.log@, poll_pos, task_tid));

          fifo_queue_after_poll(self.log@, self.log@.len() as int);
          let pre_poll_queue = fifo_queue_at(self.log@, poll_pos);
          assert(pre_poll_queue =~= Seq::empty().push(task_tid) + self.local_queue@);
          remove_first_occurrence_head(pre_poll_queue, task_tid);
          assert(fifo_queue_at(self.log@, self.log@.len() as int) =~= self.local_queue@);
        }
        *count += 1;
      }
      None => {
        return;
      }
    }

    proof {
      assert forall |k: int| entry_len as int <= k < self.log@.len() as int implies
        !is_tick_begin(self.log@[k]) && !is_tick_end(self.log@[k]) && !is_park(self.log@[k])
      by {
        if k < first_mid.len() as int {
          prefix_preserves_at(first_mid, self.log@, k);
        }
      }
      assert forall |i: int| entry_len as int <= i < self.log@.len() as int &&
        is_poll_task_at(self.log@, i) implies ({
          let tid = get_poll_task_id(self.log@[i]);
          let result = get_poll_result(self.log@[i]);
          tid_was_injected_before(self.log@, i, tid) &&
          (result == PollResult::<()>::Invalid <==> tid_is_invalid(self.log@, i, tid))
        })
      by {
        if i < first_mid.len() as int {
          prefix_preserves_at(first_mid, self.log@, i);
        } else {
          assert(i == first_poll_pos);
        }
      }
      assert forall |i: int| entry_len as int <= i < self.log@.len() as int &&
        is_poll_task_at(self.log@, i) implies
        is_fifo_head_at(self.log@, i, get_poll_task_id(self.log@[i]))
      by {
        if i < first_mid.len() as int {
          prefix_preserves_at(first_mid, self.log@, i);
        } else {
          assert(i == first_poll_pos);
        }
      }
    }

    while *count < event_interval
      invariant
        *count <= event_interval,
        self.log@.len() >= entry_len,
        entry_log =~= self.log@.subrange(0, entry_len as int),
        forall |k: int| entry_len as int <= k < self.log@.len() as int ==>
          !is_tick_begin(self.log@[k]) && !is_tick_end(self.log@[k]) &&
          !is_park(self.log@[k]),
        all_queue_tids_injected(self.log@, self.local_queue@),
        fifo_queue_matches(self.log@, self.local_queue@),
        slab_matches_log(self.task_slab@, self.log@),
        self.task_slab.wf(),
        forall |i: int| entry_len as int <= i < self.log@.len() as int &&
          is_poll_task_at(self.log@, i) ==> {
          let tid = get_poll_task_id(self.log@[i]);
          let result = get_poll_result(self.log@[i]);
          tid_was_injected_before(self.log@, i, tid) &&
          (result == PollResult::<()>::Invalid <==> tid_is_invalid(self.log@, i, tid))
        },
        forall |i: int| entry_len as int <= i < self.log@.len() as int &&
          is_poll_task_at(self.log@, i) ==>
          is_fifo_head_at(self.log@, i, get_poll_task_id(self.log@[i])),
        entry_len as int <= poll_witness && poll_witness < self.log@.len() as int,
        is_poll_task_at(self.log@, poll_witness),
        ledger_matches_log(self.ledger, self.log@),
        inv_valid_task_polling(self.log@),
      decreases event_interval - *count,
    {
      let ghost pre = self.log@;
      proof {
        // Capture invariant facts while pre == self.log@
        assert forall |i: int| entry_len as int <= i < pre.len() as int &&
          is_poll_task_at(pre, i) implies
          is_fifo_head_at(pre, i, get_poll_task_id(pre[i]))
        by {
          assert(is_poll_task_at(self.log@, i));
          assert(is_fifo_head_at(self.log@, i, get_poll_task_id(self.log@[i])));
        }
      }
      match self.next_task() {
        Some(task_id) => {
          let ghost mid = self.log@;
          let ghost poll_pos = mid.len() as int;
          let ghost task_tid = task_id@;

          proof {
            assert(is_fifo_head_at(mid, mid.len() as int, task_tid));
          }

          self.poll_task(task_id);
          proof {
            prefix_transitive(entry_log, pre, mid);
            prefix_transitive(entry_log, mid, self.log@);
            data_inv_preserved_by_extension(mid, self.log@, self.local_queue@);

            assert forall |k: int| entry_len as int <= k < self.log@.len() as int implies
              !is_tick_begin(self.log@[k]) && !is_tick_end(self.log@[k]) && !is_park(self.log@[k])
            by {
              if k < pre.len() as int { prefix_preserves_at(pre, self.log@, k); }
              else if k < mid.len() as int { prefix_preserves_at(mid, self.log@, k); }
            }

            assert(get_poll_task_id(self.log@[poll_pos]) == task_tid);
            e4_witness_survives_extension(mid, self.log@, poll_pos, task_tid);
            ledger_preserved_by_non_pop(self.ledger, mid, self.log@);
            e4_preserved_by_good_poll_push(mid, self.log@);

            fifo_queue_prefix_preserved(mid, self.log@, poll_pos);
            assert(is_fifo_head_at(self.log@, poll_pos, task_tid));

            fifo_queue_after_poll(self.log@, self.log@.len() as int);
            let pre_poll_queue = fifo_queue_at(self.log@, poll_pos);
            assert(pre_poll_queue =~= Seq::empty().push(task_tid) + self.local_queue@);
            remove_first_occurrence_head(pre_poll_queue, task_tid);
            assert(fifo_queue_at(self.log@, self.log@.len() as int) =~= self.local_queue@);

            assert forall |i: int| entry_len as int <= i < self.log@.len() as int &&
              is_poll_task_at(self.log@, i) implies ({
                let tid = get_poll_task_id(self.log@[i]);
                let result = get_poll_result(self.log@[i]);
                tid_was_injected_before(self.log@, i, tid) &&
                (result == PollResult::<()>::Invalid <==> tid_is_invalid(self.log@, i, tid))
              })
            by {
              if i < pre.len() as int {
                prefix_preserves_at(pre, self.log@, i);
                assert(is_poll_task_at(pre, i));
                e4_enhanced_survives_extension(pre, self.log@, i, get_poll_task_id(pre[i]));
              } else if i < mid.len() as int {
                prefix_preserves_at(mid, self.log@, i);
              } else {
                assert(i == poll_pos);
              }
            }

            fifo_head_preserved_after_poll(pre, mid, self.log@, entry_len as int, poll_pos, task_tid);

            prefix_preserves_at(pre, self.log@, poll_witness);
            poll_witness = poll_pos;
          }
        }
        None => {
          proof {
            prefix_transitive(entry_log, pre, self.log@);
            assert forall |k: int| entry_len as int <= k < self.log@.len() as int implies
              !is_tick_begin(self.log@[k]) && !is_tick_end(self.log@[k]) && !is_park(self.log@[k])
            by {
              if k < pre.len() as int { prefix_preserves_at(pre, self.log@, k); }
            }
            assert forall |i: int| entry_len as int <= i < self.log@.len() as int &&
              is_poll_task_at(self.log@, i) implies ({
                let tid = get_poll_task_id(self.log@[i]);
                let result = get_poll_result(self.log@[i]);
                tid_was_injected_before(self.log@, i, tid) &&
                (result == PollResult::<()>::Invalid <==> tid_is_invalid(self.log@, i, tid))
              })
            by {
              if i < pre.len() as int {
                prefix_preserves_at(pre, self.log@, i);
                assert(is_poll_task_at(pre, i));
                e4_enhanced_survives_extension(pre, self.log@, i, get_poll_task_id(pre[i]));
              }
            }
            fifo_head_preserved_no_new_polls(pre, self.log@, entry_len as int);
            prefix_preserves_at(pre, self.log@, poll_witness);
          }
          break;
        }
      }
      *count += 1;
    }
  }

  pub fn tick(&mut self)
    requires
      executor_inv(old(self).log@),
      all_queue_tids_injected(old(self).log@, old(self).local_queue@),
      fifo_queue_matches(old(self).log@, old(self).local_queue@),
      slab_matches_log(old(self).task_slab@, old(self).log@),
      old(self).event_interval >= 1,
      old(self).task_slab.wf(),
      ledger_matches_log(old(self).ledger, old(self).log@),
    ensures
      executor_inv(self.log@),
      all_queue_tids_injected(self.log@, self.local_queue@),
      fifo_queue_matches(self.log@, self.local_queue@),
      slab_matches_log(self.task_slab@, self.log@),
      self.task_slab.wf(),
      ledger_matches_log(self.ledger, self.log@),
  {
    let ghost old_log = self.log@;
    let ghost tick_start = old_log.len() as int;
    let event_interval = self.event_interval;

    proof {
      eq_valid_task_polling(old_log);
    }

    self.tick_begin_action();
    proof {
      data_inv_preserved_by_extension(old_log, self.log@, self.local_queue@);
      fifo_queue_prefix_preserved(old_log, self.log@, old_log.len() as int);
      fifo_queue_noop_event(self.log@, self.log@.len() as int);
      slab_inv_preserved_by_non_slab_event(self.task_slab@, old_log, self.log@);
      ledger_preserved_by_non_pop(self.ledger, old_log, self.log@);
      assert(old_log =~= self.log@.subrange(0, old_log.len() as int));
      assert(!is_poll_task_at(self.log@, old_log.len() as int));
      e4_recover_no_new_polls(old_log, self.log@);
    }

    let ghost after_tick_begin = self.log@;
    let ghost queue_before_deferred = self.local_queue@;

    self.wake_deferred();

    let ghost after_deferred = self.log@;
    proof {
      prefix_transitive(old_log, old_log.push(
        ExecutorEvent::Inbound(InboundCall::Tick { result: None })
      ), after_deferred);
      assert(!is_poll_task_at(after_deferred, after_tick_begin.len() as int)) by {
        drain_not_poll_task(after_deferred[after_tick_begin.len() as int]);
      }
      e4_recover_no_new_polls(after_tick_begin, after_deferred);
    }

    self.pop_injection();
    let ghost after_pop = self.log@;
    proof { prefix_transitive(old_log, after_deferred, after_pop); }

    let mut count: usize = 0;
    let ghost queue_before_loop1 = self.local_queue@;

    self.poll_loop(event_interval, &mut count);
    let ghost after_loop1 = self.log@;
    proof { prefix_transitive(old_log, after_pop, after_loop1); }

    self.park();
    let ghost after_park = self.log@;
    let ghost park_pos = after_loop1.len() as int;
    let ghost drain_reactor_pos = after_loop1.len() as int + 1;
    let ghost drain_task_pos = after_loop1.len() as int + 2;
    proof {
      prefix_transitive(old_log, after_loop1, after_park);
      assert forall |k: int| after_loop1.len() as int <= k < after_park.len() as int implies
        !is_poll_task_at(after_park, k)
      by {
        if k == drain_reactor_pos || k == drain_task_pos {
          drain_not_poll_task(after_park[k]);
        }
      }
      e4_recover_no_new_polls(after_loop1, after_park);
    }

    self.poll_loop(event_interval, &mut count);
    let ghost after_loop2 = self.log@;
    proof { prefix_transitive(old_log, after_park, after_loop2); }

    self.tick_end_action();
    let ghost tick_end_pos = after_loop2.len() as int;
    proof {
      data_inv_preserved_by_extension(after_loop2, self.log@, self.local_queue@);
      prefix_transitive(old_log, after_loop2, self.log@);
      slab_inv_preserved_by_non_slab_event(self.task_slab@, after_loop2, self.log@);
      ledger_preserved_by_non_pop(self.ledger, after_loop2, self.log@);

      fifo_queue_prefix_preserved(after_loop2, self.log@, after_loop2.len() as int);
      fifo_queue_noop_event(self.log@, self.log@.len() as int);

      let deferred_pos = tick_start + 1;
      let first_pop_pos = tick_start + 2;

      prefix_preserves_at(after_tick_begin, after_deferred, tick_start);
      prefix_preserves_at(after_deferred, self.log@, tick_start);
      assert(is_tick_begin(self.log@[tick_start]));

      prefix_preserves_at(after_deferred, self.log@, deferred_pos);
      prefix_preserves_at(after_pop, self.log@, first_pop_pos);
      prefix_preserves_at(after_park, self.log@, park_pos);
      prefix_preserves_at(after_park, self.log@, drain_reactor_pos);
      prefix_preserves_at(after_park, self.log@, drain_task_pos);

      assert forall |k: int| tick_start < k < tick_end_pos implies
        !is_tick_begin_at(self.log@, k) && !is_tick_end_at(self.log@, k)
      by {
        if k < after_deferred.len() as int {
          prefix_preserves_at(after_deferred, self.log@, k);
        } else if k < after_pop.len() as int {
          prefix_preserves_at(after_pop, self.log@, k);
        } else if k < after_loop1.len() as int {
          prefix_preserves_at(after_loop1, self.log@, k);
        } else if k < after_park.len() as int {
          prefix_preserves_at(after_park, self.log@, k);
        } else if k < after_loop2.len() as int {
          prefix_preserves_at(after_loop2, self.log@, k);
        }
      }

      assert forall |k: int| tick_start <= k <= tick_end_pos && k != park_pos implies
        !is_park_at(self.log@, k)
      by {
        if k == tick_start {
          prefix_preserves_at(after_deferred, self.log@, k);
          assert(is_tick_begin(self.log@[k]));
        } else if k < after_deferred.len() as int {
          prefix_preserves_at(after_deferred, self.log@, k);
          assert(is_drain_deferred_at(self.log@, k));
          drain_not_park(self.log@[k]);
        } else if k < after_pop.len() as int {
          prefix_preserves_at(after_pop, self.log@, k);
        } else if k < after_loop1.len() as int {
          prefix_preserves_at(after_loop1, self.log@, k);
        } else if k == park_pos {
        } else if k == drain_reactor_pos {
          prefix_preserves_at(after_park, self.log@, k);
          assert(is_drain_reactor_wake_at(self.log@, k));
          drain_not_park(self.log@[k]);
        } else if k == drain_task_pos {
          prefix_preserves_at(after_park, self.log@, k);
          assert(is_drain_task_wake_at(self.log@, k));
          drain_not_park(self.log@[k]);
        } else if k < after_loop2.len() as int {
          prefix_preserves_at(after_loop2, self.log@, k);
        } else {
          assert(k == tick_end_pos);
          assert(is_tick_end(self.log@[k]));
        }
      }

      assert(old_log_compatible(old_log, self.log@, tick_start));
      assert(well_formed_tick_segment(
        self.log@, tick_start, deferred_pos, first_pop_pos,
        park_pos, drain_reactor_pos, drain_task_pos, tick_end_pos,
      ));

      // E4: all new PollTask events have valid task polling
      assert forall |i: int| tick_start <= i <= tick_end_pos &&
        is_poll_task_at(self.log@, i) implies ({
          let tid = get_poll_task_id(self.log@[i]);
          let result = get_poll_result(self.log@[i]);
          tid_was_injected_before(self.log@, i, tid) &&
          (result == PollResult::<()>::Invalid <==> tid_is_invalid(self.log@, i, tid))
        })
      by {
        if i == tick_start {
          prefix_preserves_at(after_deferred, self.log@, i);
          assert(is_tick_begin(self.log@[i]));
        } else if i < after_deferred.len() as int {
          prefix_preserves_at(after_deferred, self.log@, i);
          assert(is_drain_deferred_at(self.log@, i));
          drain_not_poll_task(self.log@[i]);
        } else if i < after_pop.len() as int {
          prefix_preserves_at(after_pop, self.log@, i);
          assert(is_pop_injection(after_pop[i]));
          pop_injection_not_poll_task(after_pop[i]);
        } else if i < after_loop1.len() as int {
          prefix_preserves_at(after_loop1, self.log@, i);
          assert(is_poll_task_at(after_loop1, i));
          let tid_i = get_poll_task_id(after_loop1[i]);
          let result_i = get_poll_result(after_loop1[i]);
          assert(tid_was_injected_before(after_loop1, i, tid_i));
          assert(result_i == PollResult::<()>::Invalid <==> tid_is_invalid(after_loop1, i, tid_i));
          e4_enhanced_survives_extension(after_loop1, self.log@, i, tid_i);
        } else if i == park_pos {
          prefix_preserves_at(after_park, self.log@, i);
          assert(is_park(self.log@[i]));
        } else if i == drain_reactor_pos {
          prefix_preserves_at(after_park, self.log@, i);
          drain_not_poll_task(self.log@[i]);
        } else if i == drain_task_pos {
          prefix_preserves_at(after_park, self.log@, i);
          drain_not_poll_task(self.log@[i]);
        } else if i < after_loop2.len() as int {
          prefix_preserves_at(after_loop2, self.log@, i);
          assert(is_poll_task_at(after_loop2, i));
          let tid_i = get_poll_task_id(after_loop2[i]);
          let result_i = get_poll_result(after_loop2[i]);
          assert(tid_was_injected_before(after_loop2, i, tid_i));
          assert(result_i == PollResult::<()>::Invalid <==> tid_is_invalid(after_loop2, i, tid_i));
          e4_enhanced_survives_extension(after_loop2, self.log@, i, tid_i);
        } else {
          assert(i == tick_end_pos);
        }
      }

      // E9: all new PollTask events select FIFO head
      assert forall |i: int| tick_start <= i <= tick_end_pos &&
        is_poll_task_at(self.log@, i) implies ({
          let tid = get_poll_task_id(self.log@[i]);
          is_fifo_head_at(self.log@, i, tid)
        })
      by {
        if i == tick_start {
          prefix_preserves_at(after_deferred, self.log@, i);
          assert(is_tick_begin(self.log@[i]));
        } else if i < after_deferred.len() as int {
          prefix_preserves_at(after_deferred, self.log@, i);
          drain_not_poll_task(self.log@[i]);
        } else if i < after_pop.len() as int {
          prefix_preserves_at(after_pop, self.log@, i);
          assert(is_pop_injection(after_pop[i]));
          pop_injection_not_poll_task(after_pop[i]);
        } else if i < after_loop1.len() as int {
          prefix_preserves_at(after_loop1, self.log@, i);
          assert(is_poll_task_at(after_loop1, i));
          assert(after_pop.len() as int <= i);
          let tid_i = get_poll_task_id(after_loop1[i]);
          fifo_queue_prefix_preserved(after_loop1, self.log@, i);
        } else if i == park_pos {
          prefix_preserves_at(after_park, self.log@, i);
          assert(is_park(self.log@[i]));
        } else if i == drain_reactor_pos {
          prefix_preserves_at(after_park, self.log@, i);
          drain_not_poll_task(self.log@[i]);
        } else if i == drain_task_pos {
          prefix_preserves_at(after_park, self.log@, i);
          drain_not_poll_task(self.log@[i]);
        } else if i < after_loop2.len() as int {
          prefix_preserves_at(after_loop2, self.log@, i);
          assert(is_poll_task_at(after_loop2, i));
          assert(after_park.len() as int <= i);
          let tid_i = get_poll_task_id(after_loop2[i]);
          fifo_queue_prefix_preserved(after_loop2, self.log@, i);
        } else {
          assert(i == tick_end_pos);
        }
      }

      // E2: tick_polls_if_runnable
      let ghost new_tick_has_poll: bool =
        exists |q: int| tick_start < q < tick_end_pos && is_poll_task_at(self.log@, q);

      fifo_queue_prefix_preserved(old_log, self.log@, tick_start);
      assert(fifo_queue_at(self.log@, tick_start) == fifo_queue_at(old_log, old_log.len() as int));

      assert(new_tick_has_poll || fifo_queue_at(self.log@, tick_start).len() == 0) by {
        if fifo_queue_at(self.log@, tick_start).len() > 0 {
          // Chain: old local_queue was non-empty → queue stays non-empty through each step
          // tick_begin preserves queue; wake_deferred grows; pop_injection grows
          assert(queue_before_deferred.len() > 0);
          assert(queue_before_loop1.len() > 0);
          // poll_loop1: non-empty queue + count=0 < event_interval → produces PollTask
          // poll_loop1 postcondition gives exists |q| after_pop.len() <= q < after_loop1.len() && is_poll_task_at(after_loop1, q)
          let q = choose |q: int| after_pop.len() as int <= q < after_loop1.len() as int &&
            is_poll_task_at(after_loop1, q);
          // q > tick_start since after_pop.len() >= tick_start + 3
          // q < tick_end_pos since after_loop1.len() <= after_loop2.len() == tick_end_pos
          prefix_preserves_at(after_loop1, self.log@, q);
          assert(is_poll_task_at(self.log@, q));
          assert(tick_start < q && q < tick_end_pos);
        }
      }

      assert(new_tick_has_poll ==> exists |q: int| #![auto] tick_start < q < tick_end_pos &&
        is_poll_task_at(self.log@, q)) by {
        if new_tick_has_poll {
          let q = choose |q: int| tick_start < q < tick_end_pos && is_poll_task_at(self.log@, q);
          assert(tick_start < q && q < tick_end_pos && is_poll_task_at(self.log@, q));
        }
      }

      executor_inv_preserved(
        old_log, self.log@, tick_start, deferred_pos, first_pop_pos,
        park_pos, drain_reactor_pos, drain_task_pos, tick_end_pos,
        new_tick_has_poll,
      );
    }
  }
}

} // end verus!
