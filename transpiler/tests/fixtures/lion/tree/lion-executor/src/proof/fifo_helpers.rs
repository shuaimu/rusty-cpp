use vstd::prelude::*;
use crate::spec::log::*;
use crate::spec::fifo_queue::*;
use crate::types::TID;

verus! {

pub proof fn fifo_queue_prefix_preserved(old_log: Log, new_log: Log, i: int)
  requires
    old_log.len() <= new_log.len(),
    old_log =~= new_log.subrange(0, old_log.len() as int),
    0 <= i <= old_log.len(),
  ensures
    fifo_queue_at(old_log, i) == fifo_queue_at(new_log, i),
  decreases i,
{
  if i <= 0 {
  } else {
    fifo_queue_prefix_preserved(old_log, new_log, i - 1);
    assert(old_log[i - 1] == new_log[i - 1]);
  }
}

pub proof fn fifo_queue_noop_event(l: Log, i: int)
  requires
    0 < i <= l.len(),
    !is_pop_injection(l[i - 1]) || !get_pop_injection_task(l[i - 1]).is_some(),
    !is_drain(l[i - 1]),
    !is_poll_task(l[i - 1]),
  ensures
    fifo_queue_at(l, i) == fifo_queue_at(l, i - 1),
{
}

pub proof fn fifo_queue_after_push_injection(l: Log, i: int, tid: TID)
  requires
    0 < i <= l.len(),
    is_pop_injection(l[i - 1]),
    get_pop_injection_task(l[i - 1]).is_some(),
    get_pop_injection_task(l[i - 1]).unwrap().id == tid,
  ensures
    fifo_queue_at(l, i) == fifo_queue_at(l, i - 1).push(tid),
{
}

pub proof fn fifo_queue_after_drain(l: Log, i: int)
  requires
    0 < i <= l.len(),
    is_drain(l[i - 1]),
  ensures
    fifo_queue_at(l, i) == fifo_queue_at(l, i - 1) + get_drain_task_ids(l[i - 1]),
{
}

pub proof fn fifo_queue_after_poll(l: Log, i: int)
  requires
    0 < i <= l.len(),
    is_poll_task(l[i - 1]),
  ensures
    fifo_queue_at(l, i) == remove_first_occurrence(fifo_queue_at(l, i - 1), get_poll_task_id(l[i - 1])),
{
}

pub proof fn remove_first_occurrence_head(s: Seq<TID>, tid: TID)
  requires
    s.len() > 0,
    s[0] == tid,
  ensures
    remove_first_occurrence(s, tid) == s.subrange(1, s.len() as int),
{
}

pub proof fn fifo_queue_after_failed_injection(l: Log, i: int)
  requires
    0 < i <= l.len(),
    is_pop_injection(l[i - 1]),
    !get_pop_injection_task(l[i - 1]).is_some(),
  ensures
    fifo_queue_at(l, i) == fifo_queue_at(l, i - 1),
{
}

pub proof fn fifo_head_preserved_after_poll(
  pre: Log,
  mid: Log,
  post: Log,
  entry_len: int,
  poll_pos: int,
  task_tid: TID,
)
  requires
    0 <= entry_len,
    entry_len <= pre.len(),
    pre.len() <= mid.len(),
    mid.len() == poll_pos,
    post.len() == mid.len() + 1,
    pre =~= post.subrange(0, pre.len() as int),
    mid =~= post.subrange(0, mid.len() as int),
    is_poll_task_at(post, poll_pos),
    get_poll_task_id(post[poll_pos]) == task_tid,
    is_fifo_head_at(post, poll_pos, task_tid),
    forall |k: int| pre.len() as int <= k < mid.len() as int ==>
      !is_poll_task(mid[k]),
    forall |i: int| entry_len <= i < pre.len() as int &&
      is_poll_task_at(pre, i) ==>
      is_fifo_head_at(pre, i, get_poll_task_id(pre[i])),
  ensures
    forall |i: int| entry_len <= i < post.len() as int &&
      is_poll_task_at(post, i) ==>
      is_fifo_head_at(post, i, get_poll_task_id(post[i])),
{
  assert forall |i: int| entry_len <= i < post.len() as int &&
    is_poll_task_at(post, i) implies
    is_fifo_head_at(post, i, get_poll_task_id(post[i]))
  by {
    if i < pre.len() as int {
      assert(pre[i] == post[i]);
      assert(is_poll_task_at(pre, i));
      assert(is_fifo_head_at(pre, i, get_poll_task_id(pre[i])));
      let tid_i = get_poll_task_id(pre[i]);
      assert(get_poll_task_id(post[i]) == tid_i);
      fifo_queue_prefix_preserved(pre, post, i);
    } else if i < mid.len() as int {
      assert(mid[i] == post[i]);
      assert(!is_poll_task(mid[i]));
      assert(false);
    } else {
      assert(i == poll_pos);
    }
  }
}

pub proof fn fifo_head_preserved_no_new_polls(
  pre: Log,
  post: Log,
  entry_len: int,
)
  requires
    0 <= entry_len,
    entry_len <= pre.len(),
    pre.len() <= post.len(),
    pre =~= post.subrange(0, pre.len() as int),
    forall |i: int| entry_len <= i < pre.len() as int &&
      is_poll_task_at(pre, i) ==>
      is_fifo_head_at(pre, i, get_poll_task_id(pre[i])),
    forall |k: int| pre.len() as int <= k < post.len() as int ==>
      !is_poll_task(post[k]),
  ensures
    forall |i: int| entry_len <= i < post.len() as int &&
      is_poll_task_at(post, i) ==>
      is_fifo_head_at(post, i, get_poll_task_id(post[i])),
{
  assert forall |i: int| entry_len <= i < post.len() as int &&
    is_poll_task_at(post, i) implies
    is_fifo_head_at(post, i, get_poll_task_id(post[i]))
  by {
    if i < pre.len() as int {
      assert(pre[i] == post[i]);
      assert(is_poll_task_at(pre, i));
      assert(is_fifo_head_at(pre, i, get_poll_task_id(pre[i])));
      let tid_i = get_poll_task_id(pre[i]);
      fifo_queue_prefix_preserved(pre, post, i);
    }
  }
}

}
