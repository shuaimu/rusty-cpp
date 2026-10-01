use vstd::prelude::*;
use crate::spec::log::*;
use crate::proof::invariants::*;
use crate::types::{PollResult, TID, TaskView};

verus! {

pub proof fn prefix_transitive(a: Log, b: Log, c: Log)
  requires
    a.len() <= b.len(),
    b.len() <= c.len(),
    a =~= b.subrange(0, a.len() as int),
    b =~= c.subrange(0, b.len() as int),
  ensures
    a =~= c.subrange(0, a.len() as int),
{
}

pub proof fn prefix_preserves_at(log1: Log, log2: Log, pos: int)
  requires
    log1.len() <= log2.len(),
    log1 =~= log2.subrange(0, log1.len() as int),
    0 <= pos < log1.len(),
  ensures
    log1[pos] == log2[pos],
{
}

pub proof fn drain_not_park(e: ExecutorEvent)
  requires is_drain(e),
  ensures !is_park(e),
{}

pub proof fn pop_injection_not_park(e: ExecutorEvent)
  requires is_pop_injection(e),
  ensures !is_park(e),
{}

pub proof fn poll_task_not_park(e: ExecutorEvent)
  requires is_poll_task(e),
  ensures !is_park(e),
{}

pub proof fn drain_not_poll_task(e: ExecutorEvent)
  requires is_drain(e),
  ensures !is_poll_task(e),
{}

pub proof fn pop_injection_not_poll_task(e: ExecutorEvent)
  requires is_pop_injection(e),
  ensures !is_poll_task(e),
{}

pub proof fn e4_witness_survives_extension(old_log: Log, new_log: Log, i: int, tid: crate::types::TID)
  requires
    0 <= i,
    i <= old_log.len(),
    old_log.len() <= new_log.len(),
    old_log =~= new_log.subrange(0, old_log.len() as int),
    tid_was_injected_before(old_log, i, tid),
  ensures
    tid_was_injected_before(new_log, i, tid),
{
  let j = choose |j: int| 0 <= j < i &&
    is_pop_injection_at(old_log, j) &&
    get_pop_injection_task(old_log[j]).is_some() &&
    get_pop_injection_task(old_log[j]).unwrap().id == tid;
  assert(old_log[j] == new_log[j]);
  assert(0 <= j && j < i);
  assert(is_pop_injection_at(new_log, j) &&
    get_pop_injection_task(new_log[j]).is_some() &&
    get_pop_injection_task(new_log[j]).unwrap().id == tid);
}

pub proof fn data_inv_preserved_by_extension(old_log: Log, new_log: Log, queue: Seq<crate::types::TID>)
  requires
    all_queue_tids_injected(old_log, queue),
    old_log.len() <= new_log.len(),
    old_log =~= new_log.subrange(0, old_log.len() as int),
  ensures
    all_queue_tids_injected(new_log, queue),
{
  assert forall |k: int| 0 <= k < queue.len() as int implies
    tid_was_injected_before(new_log, new_log.len() as int, queue[k])
  by {
    assert(tid_was_injected_before(old_log, old_log.len() as int, queue[k]));
    let j = choose |j: int| 0 <= j < old_log.len() &&
      is_pop_injection_at(old_log, j) &&
      get_pop_injection_task(old_log[j]).is_some() &&
      get_pop_injection_task(old_log[j]).unwrap().id == queue[k];
    assert(old_log[j] == new_log[j]);
    assert(is_pop_injection_at(new_log, j));
    assert(get_pop_injection_task(new_log[j]).is_some());
    assert(get_pop_injection_task(new_log[j]).unwrap().id == queue[k]);
  }
}

pub proof fn tid_invalid_survives_extension(old_log: Log, new_log: Log, i: int, tid: crate::types::TID)
  requires
    0 <= i,
    i <= old_log.len(),
    old_log.len() <= new_log.len(),
    old_log =~= new_log.subrange(0, old_log.len() as int),
    tid_is_invalid(old_log, i, tid),
  ensures
    tid_is_invalid(new_log, i, tid),
{
  if tid_returned_ready_before(old_log, i, tid) {
    let j = choose |j: int| 0 <= j < i &&
      is_poll_task_at(old_log, j) && get_poll_task_id(old_log[j]) == tid &&
      get_poll_result(old_log[j]) == PollResult::<()>::Ready(());
    assert(old_log[j] == new_log[j]);
    assert(is_poll_task_at(new_log, j));
    assert(get_poll_task_id(new_log[j]) == tid);
    assert(get_poll_result(new_log[j]) == PollResult::<()>::Ready(()));
  }
  if !tid_was_injected_before(old_log, i, tid) {
    assert(!tid_was_injected_before(new_log, i, tid)) by {
      assert forall |j: int| !(0 <= j < i &&
        is_pop_injection_at(new_log, j) &&
        get_pop_injection_task(new_log[j]).is_some() &&
        get_pop_injection_task(new_log[j]).unwrap().id == tid)
      by {
        if 0 <= j && j < i {
          assert(new_log[j] == old_log[j]);
          assert(!(is_pop_injection_at(old_log, j) &&
            get_pop_injection_task(old_log[j]).is_some() &&
            get_pop_injection_task(old_log[j]).unwrap().id == tid));
        }
      }
    }
  }
}

pub proof fn tid_not_invalid_survives_extension(old_log: Log, new_log: Log, i: int, tid: crate::types::TID)
  requires
    0 <= i,
    i <= old_log.len(),
    old_log.len() <= new_log.len(),
    old_log =~= new_log.subrange(0, old_log.len() as int),
    !tid_is_invalid(old_log, i, tid),
  ensures
    !tid_is_invalid(new_log, i, tid),
{
  assert(!tid_returned_ready_before(old_log, i, tid));
  assert(tid_was_injected_before(old_log, i, tid));
  e4_witness_survives_extension(old_log, new_log, i, tid);
  assert(!tid_returned_ready_before(new_log, i, tid)) by {
    assert forall |j: int| !(0 <= j < i &&
      is_poll_task_at(new_log, j) &&
      get_poll_task_id(new_log[j]) == tid &&
      get_poll_result(new_log[j]) == PollResult::<()>::Ready(()))
    by {
      if 0 <= j && j < i {
        assert(new_log[j] == old_log[j]);
        assert(!(is_poll_task_at(old_log, j) &&
          get_poll_task_id(old_log[j]) == tid &&
          get_poll_result(old_log[j]) == PollResult::<()>::Ready(())));
      }
    }
  }
}

pub proof fn tid_invalid_survives_one_step(old_log: Log, new_log: Log, tid: TID)
  requires
    new_log.len() == old_log.len() + 1,
    old_log =~= new_log.subrange(0, old_log.len() as int),
    tid_is_invalid(old_log, old_log.len() as int, tid),
  ensures
    tid_is_invalid(new_log, old_log.len() as int, tid),
{
  tid_invalid_survives_extension(old_log, new_log, old_log.len() as int, tid);
}

pub proof fn not_ready_in_old_from_new(old_log: Log, new_log: Log, t: TID)
  requires
    new_log.len() == old_log.len() + 1,
    old_log =~= new_log.subrange(0, old_log.len() as int),
    !tid_returned_ready_before(new_log, new_log.len() as int, t),
  ensures
    !tid_returned_ready_before(old_log, old_log.len() as int, t),
{
  if tid_returned_ready_before(old_log, old_log.len() as int, t) {
    let k = choose |k: int| 0 <= k < old_log.len() as int &&
      is_poll_task_at(old_log, k) && get_poll_task_id(old_log[k]) == t &&
      get_poll_result(old_log[k]) == PollResult::<()>::Ready(());
    assert(old_log[k] == new_log[k]);
    assert(is_poll_task_at(new_log, k));
    assert(get_poll_task_id(new_log[k]) == t);
    assert(get_poll_result(new_log[k]) == PollResult::<()>::Ready(()));
    assert(tid_returned_ready_before(new_log, new_log.len() as int, t));
  }
}

pub proof fn slab_inv_preserved_by_non_slab_event(
  slab: Map<TID, TaskView>, old_log: Log, new_log: Log,
)
  requires
    slab_matches_log(slab, old_log),
    new_log.len() == old_log.len() + 1,
    old_log =~= new_log.subrange(0, old_log.len() as int),
    !is_pop_injection(new_log[old_log.len() as int]) ||
      !get_pop_injection_task(new_log[old_log.len() as int]).is_some(),
    !is_poll_task(new_log[old_log.len() as int]),
  ensures
    slab_matches_log(slab, new_log),
{
  assert forall |tid: TID|
    slab.contains_key(tid) <==>
      (tid_was_injected_before(new_log, new_log.len() as int, tid) &&
       !tid_returned_ready_before(new_log, new_log.len() as int, tid))
  by {
    if slab.contains_key(tid) {
      assert(tid_was_injected_before(old_log, old_log.len() as int, tid));
      assert(!tid_returned_ready_before(old_log, old_log.len() as int, tid));
      e4_witness_survives_extension(old_log, new_log, old_log.len() as int, tid);
      not_ready_extends(old_log, new_log, tid);
    }
    if tid_was_injected_before(new_log, new_log.len() as int, tid) &&
       !tid_returned_ready_before(new_log, new_log.len() as int, tid) {
      let j = choose |j: int| 0 <= j < new_log.len() as int &&
        is_pop_injection_at(new_log, j) &&
        get_pop_injection_task(new_log[j]).is_some() &&
        get_pop_injection_task(new_log[j]).unwrap().id == tid;
      assert(j < old_log.len() as int) by {
        if j == old_log.len() as int {
          assert(!is_pop_injection(new_log[j]) ||
            !get_pop_injection_task(new_log[j]).is_some());
        }
      }
      assert(old_log[j] == new_log[j]);
      assert(is_pop_injection_at(old_log, j));
      assert(get_pop_injection_task(old_log[j]).is_some());
      assert(get_pop_injection_task(old_log[j]).unwrap().id == tid);
      assert(tid_was_injected_before(old_log, old_log.len() as int, tid));
      not_ready_in_old_from_new(old_log, new_log, tid);
    }
  }
}

pub proof fn not_ready_extends(old_log: Log, new_log: Log, t: TID)
  requires
    new_log.len() == old_log.len() + 1,
    old_log =~= new_log.subrange(0, old_log.len() as int),
    !is_poll_task(new_log[old_log.len() as int]),
    !tid_returned_ready_before(old_log, old_log.len() as int, t),
  ensures
    !tid_returned_ready_before(new_log, new_log.len() as int, t),
{
  if tid_returned_ready_before(new_log, new_log.len() as int, t) {
    let j = choose |j: int| 0 <= j < new_log.len() as int &&
      is_poll_task_at(new_log, j) && get_poll_task_id(new_log[j]) == t &&
      get_poll_result(new_log[j]) == PollResult::<()>::Ready(());
    if j < old_log.len() as int {
      assert(new_log[j] == old_log[j]);
      assert(is_poll_task_at(old_log, j));
      assert(get_poll_task_id(old_log[j]) == t);
      assert(get_poll_result(old_log[j]) == PollResult::<()>::Ready(()));
      assert(tid_returned_ready_before(old_log, old_log.len() as int, t));
    }
  }
}

pub proof fn slab_inv_preserved_by_injection(
  slab: Map<TID, TaskView>,
  old_log: Log,
  new_log: Log,
  tid: TID,
  task_view: TaskView,
)
  requires
    slab_matches_log(slab, old_log),
    new_log.len() == old_log.len() + 1,
    old_log =~= new_log.subrange(0, old_log.len() as int),
    is_pop_injection_at(new_log, old_log.len() as int),
    get_pop_injection_task(new_log[old_log.len() as int]).is_some(),
    get_pop_injection_task(new_log[old_log.len() as int]).unwrap().id == tid,
    task_view.id == tid,
    !tid_was_injected_before(old_log, old_log.len() as int, tid),
    !tid_returned_ready_before(old_log, old_log.len() as int, tid),
  ensures
    slab_matches_log(slab.insert(tid, task_view), new_log),
{
  assert(!is_poll_task(new_log[old_log.len() as int])) by {
    assert(is_pop_injection(new_log[old_log.len() as int]));
    pop_injection_not_poll_task(new_log[old_log.len() as int]);
  }

  assert forall |t: TID|
    slab.insert(tid, task_view).contains_key(t) <==>
      (tid_was_injected_before(new_log, new_log.len() as int, t) &&
       !tid_returned_ready_before(new_log, new_log.len() as int, t))
  by {
    if t == tid {
      assert(tid_was_injected_before(new_log, new_log.len() as int, tid));
      not_ready_extends(old_log, new_log, tid);
    } else {
      if slab.contains_key(t) {
        assert(tid_was_injected_before(old_log, old_log.len() as int, t));
        assert(!tid_returned_ready_before(old_log, old_log.len() as int, t));
        e4_witness_survives_extension(old_log, new_log, old_log.len() as int, t);
        not_ready_extends(old_log, new_log, t);
      }
      if tid_was_injected_before(new_log, new_log.len() as int, t) &&
         !tid_returned_ready_before(new_log, new_log.len() as int, t) {
        let j = choose |j: int| 0 <= j < new_log.len() as int &&
          is_pop_injection_at(new_log, j) &&
          get_pop_injection_task(new_log[j]).is_some() &&
          get_pop_injection_task(new_log[j]).unwrap().id == t;
        assert(j < old_log.len() as int) by {
          if j == old_log.len() as int {
            assert(get_pop_injection_task(new_log[j]).unwrap().id == tid);
          }
        }
        assert(old_log[j] == new_log[j]);
        assert(is_pop_injection_at(old_log, j));
        assert(get_pop_injection_task(old_log[j]).is_some());
        assert(get_pop_injection_task(old_log[j]).unwrap().id == t);
        assert(tid_was_injected_before(old_log, old_log.len() as int, t));
        not_ready_in_old_from_new(old_log, new_log, t);
      }
    }
  }
}

pub proof fn e4_enhanced_survives_extension(old_log: Log, new_log: Log, i: int, tid: crate::types::TID)
  requires
    0 <= i,
    i < old_log.len(),
    old_log.len() <= new_log.len(),
    old_log =~= new_log.subrange(0, old_log.len() as int),
    is_poll_task_at(old_log, i),
    tid_was_injected_before(old_log, i, get_poll_task_id(old_log[i])),
    ({
      let tid = get_poll_task_id(old_log[i]);
      let result = get_poll_result(old_log[i]);
      result == PollResult::<()>::Invalid <==> tid_is_invalid(old_log, i, tid)
    }),
  ensures
    ({
      let tid = get_poll_task_id(new_log[i]);
      let result = get_poll_result(new_log[i]);
      tid_was_injected_before(new_log, i, tid) &&
      (result == PollResult::<()>::Invalid <==> tid_is_invalid(new_log, i, tid))
    }),
{
  let tid = get_poll_task_id(old_log[i]);
  e4_witness_survives_extension(old_log, new_log, i, tid);
  assert(old_log[i] == new_log[i]);
  if tid_is_invalid(old_log, i, tid) {
    tid_invalid_survives_extension(old_log, new_log, i, tid);
  }
  if !tid_is_invalid(old_log, i, tid) {
    tid_not_invalid_survives_extension(old_log, new_log, i, tid);
  }
}

pub proof fn slab_inv_preserved_by_poll_pending(
  old_slab: Map<TID, TaskView>,
  old_log: Log,
  new_log: Log,
  tid: TID,
  task_view: TaskView,
)
  requires
    slab_matches_log(old_slab, old_log),
    new_log.len() == old_log.len() + 1,
    old_log =~= new_log.subrange(0, old_log.len() as int),
    is_poll_task_at(new_log, old_log.len() as int),
    get_poll_task_id(new_log[old_log.len() as int]) == tid,
    get_poll_result(new_log[old_log.len() as int]) == PollResult::<()>::Pending,
    old_slab.contains_key(tid),
  ensures
    slab_matches_log(old_slab.remove(tid).insert(tid, task_view), new_log),
{
  let new_slab = old_slab.remove(tid).insert(tid, task_view);
  assert forall |t: TID|
    new_slab.contains_key(t) <==>
      (tid_was_injected_before(new_log, new_log.len() as int, t) &&
       !tid_returned_ready_before(new_log, new_log.len() as int, t))
  by {
    assert(new_slab.contains_key(t) == old_slab.contains_key(t));
    if old_slab.contains_key(t) {
      assert(tid_was_injected_before(old_log, old_log.len() as int, t));
      assert(!tid_returned_ready_before(old_log, old_log.len() as int, t));
      e4_witness_survives_extension(old_log, new_log, old_log.len() as int, t);
      not_ready_extends_poll_pending(old_log, new_log, t, tid);
    }
    if tid_was_injected_before(new_log, new_log.len() as int, t) &&
       !tid_returned_ready_before(new_log, new_log.len() as int, t) {
      let j = choose |j: int| 0 <= j < new_log.len() as int &&
        is_pop_injection_at(new_log, j) &&
        get_pop_injection_task(new_log[j]).is_some() &&
        get_pop_injection_task(new_log[j]).unwrap().id == t;
      assert(j < old_log.len() as int) by {
        if j == old_log.len() as int {
          assert(is_poll_task(new_log[j]));
          poll_task_not_pop_injection(new_log[j]);
        }
      }
      assert(old_log[j] == new_log[j]);
      assert(is_pop_injection_at(old_log, j));
      assert(get_pop_injection_task(old_log[j]).is_some());
      assert(get_pop_injection_task(old_log[j]).unwrap().id == t);
      assert(tid_was_injected_before(old_log, old_log.len() as int, t));
      not_ready_in_old_from_new_poll(old_log, new_log, t, tid);
    }
  }
}

pub proof fn slab_inv_preserved_by_poll_ready(
  old_slab: Map<TID, TaskView>,
  old_log: Log,
  new_log: Log,
  tid: TID,
)
  requires
    slab_matches_log(old_slab, old_log),
    new_log.len() == old_log.len() + 1,
    old_log =~= new_log.subrange(0, old_log.len() as int),
    is_poll_task_at(new_log, old_log.len() as int),
    get_poll_task_id(new_log[old_log.len() as int]) == tid,
    get_poll_result(new_log[old_log.len() as int]) == PollResult::<()>::Ready(()),
    old_slab.contains_key(tid),
  ensures
    slab_matches_log(old_slab.remove(tid), new_log),
{
  let new_slab = old_slab.remove(tid);
  assert forall |t: TID|
    new_slab.contains_key(t) <==>
      (tid_was_injected_before(new_log, new_log.len() as int, t) &&
       !tid_returned_ready_before(new_log, new_log.len() as int, t))
  by {
    if t == tid {
      assert(!new_slab.contains_key(t));
      assert(tid_returned_ready_before(new_log, new_log.len() as int, t)) by {
        assert(is_poll_task_at(new_log, old_log.len() as int));
        assert(get_poll_task_id(new_log[old_log.len() as int]) == t);
        assert(get_poll_result(new_log[old_log.len() as int]) == PollResult::<()>::Ready(()));
      }
    } else {
      assert(new_slab.contains_key(t) == old_slab.contains_key(t));
      if old_slab.contains_key(t) {
        assert(tid_was_injected_before(old_log, old_log.len() as int, t));
        assert(!tid_returned_ready_before(old_log, old_log.len() as int, t));
        e4_witness_survives_extension(old_log, new_log, old_log.len() as int, t);
        not_ready_extends_poll_other(old_log, new_log, t, tid);
      }
      if tid_was_injected_before(new_log, new_log.len() as int, t) &&
         !tid_returned_ready_before(new_log, new_log.len() as int, t) {
        let j = choose |j: int| 0 <= j < new_log.len() as int &&
          is_pop_injection_at(new_log, j) &&
          get_pop_injection_task(new_log[j]).is_some() &&
          get_pop_injection_task(new_log[j]).unwrap().id == t;
        assert(j < old_log.len() as int) by {
          if j == old_log.len() as int {
            assert(is_poll_task(new_log[j]));
            poll_task_not_pop_injection(new_log[j]);
          }
        }
        assert(old_log[j] == new_log[j]);
        assert(is_pop_injection_at(old_log, j));
        assert(get_pop_injection_task(old_log[j]).is_some());
        assert(get_pop_injection_task(old_log[j]).unwrap().id == t);
        assert(tid_was_injected_before(old_log, old_log.len() as int, t));
        not_ready_in_old_from_new_poll(old_log, new_log, t, tid);
      }
    }
  }
}

pub proof fn slab_inv_preserved_by_poll_invalid(
  slab: Map<TID, TaskView>,
  old_log: Log,
  new_log: Log,
  tid: TID,
)
  requires
    slab_matches_log(slab, old_log),
    new_log.len() == old_log.len() + 1,
    old_log =~= new_log.subrange(0, old_log.len() as int),
    is_poll_task_at(new_log, old_log.len() as int),
    get_poll_task_id(new_log[old_log.len() as int]) == tid,
    get_poll_result(new_log[old_log.len() as int]) == PollResult::<()>::Invalid,
    !slab.contains_key(tid),
  ensures
    slab_matches_log(slab, new_log),
{
  assert forall |t: TID|
    slab.contains_key(t) <==>
      (tid_was_injected_before(new_log, new_log.len() as int, t) &&
       !tid_returned_ready_before(new_log, new_log.len() as int, t))
  by {
    if slab.contains_key(t) {
      assert(t != tid);
      assert(tid_was_injected_before(old_log, old_log.len() as int, t));
      assert(!tid_returned_ready_before(old_log, old_log.len() as int, t));
      e4_witness_survives_extension(old_log, new_log, old_log.len() as int, t);
      not_ready_extends_poll_other(old_log, new_log, t, tid);
    }
    if tid_was_injected_before(new_log, new_log.len() as int, t) &&
       !tid_returned_ready_before(new_log, new_log.len() as int, t) {
      let j = choose |j: int| 0 <= j < new_log.len() as int &&
        is_pop_injection_at(new_log, j) &&
        get_pop_injection_task(new_log[j]).is_some() &&
        get_pop_injection_task(new_log[j]).unwrap().id == t;
      assert(j < old_log.len() as int) by {
        if j == old_log.len() as int {
          assert(is_poll_task(new_log[j]));
          poll_task_not_pop_injection(new_log[j]);
        }
      }
      assert(old_log[j] == new_log[j]);
      assert(is_pop_injection_at(old_log, j));
      assert(get_pop_injection_task(old_log[j]).is_some());
      assert(get_pop_injection_task(old_log[j]).unwrap().id == t);
      assert(tid_was_injected_before(old_log, old_log.len() as int, t));
      not_ready_in_old_from_new_poll(old_log, new_log, t, tid);
    }
  }
}

pub proof fn not_ready_extends_poll_pending(old_log: Log, new_log: Log, t: TID, poll_tid: TID)
  requires
    new_log.len() == old_log.len() + 1,
    old_log =~= new_log.subrange(0, old_log.len() as int),
    is_poll_task_at(new_log, old_log.len() as int),
    get_poll_task_id(new_log[old_log.len() as int]) == poll_tid,
    get_poll_result(new_log[old_log.len() as int]) == PollResult::<()>::Pending,
    !tid_returned_ready_before(old_log, old_log.len() as int, t),
  ensures
    !tid_returned_ready_before(new_log, new_log.len() as int, t),
{
  if tid_returned_ready_before(new_log, new_log.len() as int, t) {
    let j = choose |j: int| 0 <= j < new_log.len() as int &&
      is_poll_task_at(new_log, j) && get_poll_task_id(new_log[j]) == t &&
      get_poll_result(new_log[j]) == PollResult::<()>::Ready(());
    if j < old_log.len() as int {
      assert(new_log[j] == old_log[j]);
      assert(is_poll_task_at(old_log, j));
      assert(get_poll_task_id(old_log[j]) == t);
      assert(get_poll_result(old_log[j]) == PollResult::<()>::Ready(()));
      assert(tid_returned_ready_before(old_log, old_log.len() as int, t));
    } else {
      assert(j == old_log.len() as int);
      assert(get_poll_result(new_log[j]) == PollResult::<()>::Pending);
    }
  }
}

pub proof fn not_ready_extends_poll_other(old_log: Log, new_log: Log, t: TID, poll_tid: TID)
  requires
    new_log.len() == old_log.len() + 1,
    old_log =~= new_log.subrange(0, old_log.len() as int),
    is_poll_task_at(new_log, old_log.len() as int),
    get_poll_task_id(new_log[old_log.len() as int]) == poll_tid,
    t != poll_tid,
    !tid_returned_ready_before(old_log, old_log.len() as int, t),
  ensures
    !tid_returned_ready_before(new_log, new_log.len() as int, t),
{
  if tid_returned_ready_before(new_log, new_log.len() as int, t) {
    let j = choose |j: int| 0 <= j < new_log.len() as int &&
      is_poll_task_at(new_log, j) && get_poll_task_id(new_log[j]) == t &&
      get_poll_result(new_log[j]) == PollResult::<()>::Ready(());
    if j < old_log.len() as int {
      assert(new_log[j] == old_log[j]);
      assert(is_poll_task_at(old_log, j));
      assert(get_poll_task_id(old_log[j]) == t);
      assert(get_poll_result(old_log[j]) == PollResult::<()>::Ready(()));
      assert(tid_returned_ready_before(old_log, old_log.len() as int, t));
    } else {
      assert(j == old_log.len() as int);
      assert(get_poll_task_id(new_log[j]) == poll_tid);
      assert(t == poll_tid);
    }
  }
}

pub proof fn not_ready_in_old_from_new_poll(old_log: Log, new_log: Log, t: TID, poll_tid: TID)
  requires
    new_log.len() == old_log.len() + 1,
    old_log =~= new_log.subrange(0, old_log.len() as int),
    is_poll_task_at(new_log, old_log.len() as int),
    get_poll_task_id(new_log[old_log.len() as int]) == poll_tid,
    !tid_returned_ready_before(new_log, new_log.len() as int, t),
  ensures
    !tid_returned_ready_before(old_log, old_log.len() as int, t),
{
  if tid_returned_ready_before(old_log, old_log.len() as int, t) {
    let k = choose |k: int| 0 <= k < old_log.len() as int &&
      is_poll_task_at(old_log, k) && get_poll_task_id(old_log[k]) == t &&
      get_poll_result(old_log[k]) == PollResult::<()>::Ready(());
    assert(old_log[k] == new_log[k]);
    assert(is_poll_task_at(new_log, k));
    assert(get_poll_task_id(new_log[k]) == t);
    assert(get_poll_result(new_log[k]) == PollResult::<()>::Ready(()));
    assert(tid_returned_ready_before(new_log, new_log.len() as int, t));
  }
}

pub proof fn poll_task_not_pop_injection(e: ExecutorEvent)
  requires is_poll_task(e),
  ensures !is_pop_injection(e),
{}

}
