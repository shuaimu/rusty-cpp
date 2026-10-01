use vstd::prelude::*;
use crate::spec::log::*;
use crate::spec::fifo_queue::*;
use crate::proof::invariants::*;
use crate::types::PollResult;

verus! {

pub open spec fn well_formed_tick_segment(
  l: Log,
  tick_start: int,
  deferred_pos: int,
  first_pop_pos: int,
  park_pos: int,
  drain_reactor_pos: int,
  drain_task_pos: int,
  tick_end_pos: int,
) -> bool {
  &&& 0 <= tick_start
  &&& tick_start < deferred_pos
  &&& deferred_pos < first_pop_pos
  &&& first_pop_pos <= park_pos
  &&& park_pos < drain_reactor_pos
  &&& drain_reactor_pos < drain_task_pos
  &&& drain_task_pos < tick_end_pos
  &&& tick_end_pos < l.len()
  &&& is_tick_begin_at(l, tick_start)
  &&& is_drain_deferred_at(l, deferred_pos)
  &&& is_pop_injection_at(l, first_pop_pos)
  &&& is_park_at(l, park_pos)
  &&& is_drain_reactor_wake_at(l, drain_reactor_pos)
  &&& is_drain_task_wake_at(l, drain_task_pos)
  &&& is_tick_end_at(l, tick_end_pos)
  &&& tick_end_pos == l.len() - 1
  &&& (forall |k: int| tick_start < k < tick_end_pos ==>
    !is_tick_begin_at(l, k) && !is_tick_end_at(l, k))
  &&& (forall |k: int| tick_start <= k <= tick_end_pos && k != park_pos ==>
    !is_park_at(l, k))
  &&& no_tick_end_between(l, park_pos, drain_reactor_pos)
}

pub open spec fn old_log_compatible(old_log: Log, new_log: Log, tick_start: int) -> bool {
  &&& tick_start == old_log.len()
  &&& old_log.len() < new_log.len()
  &&& old_log =~= new_log.subrange(0, old_log.len() as int)
}

// E5 preservation
proof fn tick_has_park_preserved(
  old_log: Log, new_log: Log,
  tick_start: int, deferred_pos: int, first_pop_pos: int,
  park_pos: int, drain_reactor_pos: int, drain_task_pos: int, tick_end_pos: int,
)
  requires
    inv_tick_has_park(old_log),
    old_log_compatible(old_log, new_log, tick_start),
    well_formed_tick_segment(new_log, tick_start, deferred_pos, first_pop_pos,
      park_pos, drain_reactor_pos, drain_task_pos, tick_end_pos),
  ensures inv_tick_has_park(new_log),
{
  assert forall |i: int| is_tick_end_at(new_log, i) implies
    exists |p: int| #![auto] 0 <= p < i && is_park_at(new_log, p) &&
      no_tick_begin_between(new_log, p, i)
  by {
    if i < old_log.len() as int {
      assert(is_tick_end_at(old_log, i));
      let p = choose |p: int| 0 <= p < i && is_park_at(old_log, p) &&
        no_tick_begin_between(old_log, p, i);
      assert(is_park_at(new_log, p));
      assert forall |k: int| p < k <= i implies !is_tick_begin_at(new_log, k) by {
        assert(!is_tick_begin_at(old_log, k));
      }
    } else {
      assert(i == tick_end_pos);
      assert(is_park_at(new_log, park_pos));
      assert forall |k: int| park_pos < k <= tick_end_pos implies
        !is_tick_begin_at(new_log, k) by {
        if k < tick_end_pos { assert(!is_tick_begin_at(new_log, k)); }
      }
    }
  }
}

// E6 preservation
proof fn tick_has_pop_injection_preserved(
  old_log: Log, new_log: Log,
  tick_start: int, deferred_pos: int, first_pop_pos: int,
  park_pos: int, drain_reactor_pos: int, drain_task_pos: int, tick_end_pos: int,
)
  requires
    inv_tick_has_pop_injection(old_log),
    old_log_compatible(old_log, new_log, tick_start),
    well_formed_tick_segment(new_log, tick_start, deferred_pos, first_pop_pos,
      park_pos, drain_reactor_pos, drain_task_pos, tick_end_pos),
  ensures inv_tick_has_pop_injection(new_log),
{
  assert forall |i: int| is_tick_end_at(new_log, i) implies
    exists |p: int| #![auto] 0 <= p < i && is_pop_injection_at(new_log, p) &&
      no_tick_begin_between(new_log, p, i)
  by {
    if i < old_log.len() as int {
      assert(is_tick_end_at(old_log, i));
      let p = choose |p: int| 0 <= p < i && is_pop_injection_at(old_log, p) &&
        no_tick_begin_between(old_log, p, i);
      assert(is_pop_injection_at(new_log, p));
      assert forall |k: int| p < k <= i implies !is_tick_begin_at(new_log, k) by {
        assert(!is_tick_begin_at(old_log, k));
      }
    } else {
      assert(i == tick_end_pos);
      assert(is_pop_injection_at(new_log, first_pop_pos));
      assert forall |k: int| first_pop_pos < k <= tick_end_pos implies
        !is_tick_begin_at(new_log, k) by {
        if k < tick_end_pos { assert(!is_tick_begin_at(new_log, k)); }
      }
    }
  }
}

// E7 preservation
proof fn tick_has_drain_deferred_preserved(
  old_log: Log, new_log: Log,
  tick_start: int, deferred_pos: int, first_pop_pos: int,
  park_pos: int, drain_reactor_pos: int, drain_task_pos: int, tick_end_pos: int,
)
  requires
    inv_tick_has_drain_deferred(old_log),
    old_log_compatible(old_log, new_log, tick_start),
    well_formed_tick_segment(new_log, tick_start, deferred_pos, first_pop_pos,
      park_pos, drain_reactor_pos, drain_task_pos, tick_end_pos),
  ensures inv_tick_has_drain_deferred(new_log),
{
  assert forall |i: int| is_tick_end_at(new_log, i) implies
    exists |d: int| #![auto] 0 <= d < i && is_drain_deferred_at(new_log, d) &&
      no_tick_begin_between(new_log, d, i)
  by {
    if i < old_log.len() as int {
      assert(is_tick_end_at(old_log, i));
      let d = choose |d: int| 0 <= d < i && is_drain_deferred_at(old_log, d) &&
        no_tick_begin_between(old_log, d, i);
      assert(is_drain_deferred_at(new_log, d));
      assert forall |k: int| d < k <= i implies !is_tick_begin_at(new_log, k) by {
        assert(!is_tick_begin_at(old_log, k));
      }
    } else {
      assert(i == tick_end_pos);
      assert(is_drain_deferred_at(new_log, deferred_pos));
      assert forall |k: int| deferred_pos < k <= tick_end_pos implies
        !is_tick_begin_at(new_log, k) by {
        if k < tick_end_pos { assert(!is_tick_begin_at(new_log, k)); }
      }
    }
  }
}

// E8 preservation
proof fn tick_has_drain_task_wake_preserved(
  old_log: Log, new_log: Log,
  tick_start: int, deferred_pos: int, first_pop_pos: int,
  park_pos: int, drain_reactor_pos: int, drain_task_pos: int, tick_end_pos: int,
)
  requires
    inv_tick_has_drain_task_wake(old_log),
    old_log_compatible(old_log, new_log, tick_start),
    well_formed_tick_segment(new_log, tick_start, deferred_pos, first_pop_pos,
      park_pos, drain_reactor_pos, drain_task_pos, tick_end_pos),
  ensures inv_tick_has_drain_task_wake(new_log),
{
  assert forall |i: int| is_tick_end_at(new_log, i) implies
    exists |d: int| #![auto] 0 <= d < i && is_drain_task_wake_at(new_log, d) &&
      no_tick_begin_between(new_log, d, i)
  by {
    if i < old_log.len() as int {
      assert(is_tick_end_at(old_log, i));
      let d = choose |d: int| 0 <= d < i && is_drain_task_wake_at(old_log, d) &&
        no_tick_begin_between(old_log, d, i);
      assert(is_drain_task_wake_at(new_log, d));
      assert forall |k: int| d < k <= i implies !is_tick_begin_at(new_log, k) by {
        assert(!is_tick_begin_at(old_log, k));
      }
    } else {
      assert(i == tick_end_pos);
      assert(is_drain_task_wake_at(new_log, drain_task_pos));
      assert forall |k: int| drain_task_pos < k <= tick_end_pos implies
        !is_tick_begin_at(new_log, k) by {
        if k < tick_end_pos { assert(!is_tick_begin_at(new_log, k)); }
      }
    }
  }
}

// E1 preservation
proof fn park_drain_reactor_wake_preserved(
  old_log: Log, new_log: Log,
  tick_start: int, deferred_pos: int, first_pop_pos: int,
  park_pos: int, drain_reactor_pos: int, drain_task_pos: int, tick_end_pos: int,
)
  requires
    inv_park_drain_reactor_wake(old_log),
    old_log_compatible(old_log, new_log, tick_start),
    well_formed_tick_segment(new_log, tick_start, deferred_pos, first_pop_pos,
      park_pos, drain_reactor_pos, drain_task_pos, tick_end_pos),
  ensures inv_park_drain_reactor_wake(new_log),
{
  assert forall |i: int| is_park_at(new_log, i) implies
    exists |j: int| #![auto] j > i && is_drain_reactor_wake_at(new_log, j) &&
      no_tick_end_between(new_log, i, j)
  by {
    if i < old_log.len() as int {
      assert(is_park_at(old_log, i));
      let j = choose |j: int| j > i && is_drain_reactor_wake_at(old_log, j) &&
        no_tick_end_between(old_log, i, j);
      assert(j < old_log.len());
      assert(is_drain_reactor_wake_at(new_log, j));
      assert forall |k: int| i < k < j implies !is_tick_end_at(new_log, k) by {
        assert(!is_tick_end_at(old_log, k));
      }
    } else {
      assert(i == park_pos);
      assert(is_drain_reactor_wake_at(new_log, drain_reactor_pos));
      assert forall |k: int| park_pos < k < drain_reactor_pos implies
        !is_tick_end_at(new_log, k) by {
        assert(!is_tick_end_at(new_log, k));
      }
    }
  }
}

// E3 preservation: poll_within_tick
proof fn poll_within_tick_preserved(
  old_log: Log, new_log: Log,
  tick_start: int, deferred_pos: int, first_pop_pos: int,
  park_pos: int, drain_reactor_pos: int, drain_task_pos: int, tick_end_pos: int,
)
  requires
    inv_poll_within_tick(old_log),
    old_log_compatible(old_log, new_log, tick_start),
    well_formed_tick_segment(new_log, tick_start, deferred_pos, first_pop_pos,
      park_pos, drain_reactor_pos, drain_task_pos, tick_end_pos),
  ensures inv_poll_within_tick(new_log),
{
  assert forall |i: int| is_poll_task_at(new_log, i) implies
    exists |tb: int| #![auto]
      0 <= tb < i && is_tick_begin_at(new_log, tb) &&
      no_tick_begin_between(new_log, tb, i)
  by {
    if i < old_log.len() as int {
      assert(is_poll_task_at(old_log, i));
      let tb = choose |tb: int| 0 <= tb < i && is_tick_begin_at(old_log, tb) &&
        no_tick_begin_between(old_log, tb, i);
      assert(is_tick_begin_at(new_log, tb));
      assert forall |k: int| tb < k <= i implies !is_tick_begin_at(new_log, k) by {
        assert(!is_tick_begin_at(old_log, k));
      }
    } else {
      assert(tick_start < i);
      assert(is_tick_begin_at(new_log, tick_start));
      assert forall |k: int| tick_start < k <= i implies
        !is_tick_begin_at(new_log, k) by {
        assert(!is_tick_begin_at(new_log, k));
      }
    }
  }
}

// E4 preservation: valid_task_polling
proof fn valid_task_polling_preserved(
  old_log: Log, new_log: Log,
  tick_start: int, tick_end_pos: int,
)
  requires
    inv_valid_task_polling(old_log),
    old_log_compatible(old_log, new_log, tick_start),
    tick_end_pos == new_log.len() as int - 1,
    forall |i: int| tick_start <= i <= tick_end_pos && is_poll_task_at(new_log, i) ==> {
      let tid = get_poll_task_id(new_log[i]);
      let result = get_poll_result(new_log[i]);
      tid_was_injected_before(new_log, i, tid) &&
      (result == PollResult::<()>::Invalid <==> tid_is_invalid(new_log, i, tid))
    },
  ensures inv_valid_task_polling(new_log),
{
  assert forall |i: int| is_poll_task_at(new_log, i) implies ({
    let tid = get_poll_task_id(new_log[i]);
    let result = get_poll_result(new_log[i]);
    tid_was_injected_before(new_log, i, tid) &&
    (result == PollResult::<()>::Invalid <==> tid_is_invalid(new_log, i, tid))
  })
  by {
    if i < old_log.len() as int {
      assert(old_log[i] == new_log[i]);
      assert(is_poll_task_at(old_log, i));
      crate::proof::helpers::e4_enhanced_survives_extension(old_log, new_log, i, get_poll_task_id(old_log[i]));
    }
  }
}

// E2 preservation: tick_polls_if_runnable
proof fn tick_polls_if_runnable_preserved(
  old_log: Log, new_log: Log,
  tick_start: int, deferred_pos: int, first_pop_pos: int,
  park_pos: int, drain_reactor_pos: int, drain_task_pos: int, tick_end_pos: int,
  new_tick_has_poll: bool,
)
  requires
    inv_tick_polls_if_runnable(old_log),
    old_log_compatible(old_log, new_log, tick_start),
    well_formed_tick_segment(new_log, tick_start, deferred_pos, first_pop_pos,
      park_pos, drain_reactor_pos, drain_task_pos, tick_end_pos),
    // For the new tick: if fifo_queue_at(new_log, tick_start) is non-empty, there's a PollTask
    new_tick_has_poll || fifo_queue_at(new_log, tick_start).len() == 0,
    new_tick_has_poll ==> exists |q: int| #![auto] tick_start < q < tick_end_pos &&
      is_poll_task_at(new_log, q),
  ensures inv_tick_polls_if_runnable(new_log),
{
  assert forall |i: int|
    is_tick_begin_at(new_log, i) && fifo_queue_at(new_log, i).len() > 0 implies
    exists |j: int| #![auto]
      j > i && is_poll_task_at(new_log, j) &&
      (forall |k: int| i < k < j ==> !#[trigger] is_tick_end_at(new_log, k))
  by {
    if i < old_log.len() as int {
      assert(is_tick_begin_at(old_log, i));
      // fifo_queue_at(new_log, i) == fifo_queue_at(old_log, i) because positions < i are the same
      crate::proof::fifo_helpers::fifo_queue_prefix_preserved(old_log, new_log, i);
      assert(fifo_queue_at(old_log, i).len() > 0);
      let j = choose |j: int| j > i && is_poll_task_at(old_log, j) &&
        (forall |k: int| i < k < j ==> !is_tick_end_at(old_log, k));
      assert(j < old_log.len());
      assert(old_log[j] == new_log[j]);
      assert(is_poll_task_at(new_log, j));
      assert forall |k: int| i < k < j implies !is_tick_end_at(new_log, k) by {
        assert(new_log[k] == old_log[k]);
        assert(!is_tick_end_at(old_log, k));
      }
    } else {
      assert(i == tick_start);
      // This is the new tick
    }
  }
}

// E9 preservation: fifo_task_selection
proof fn fifo_task_selection_preserved(
  old_log: Log, new_log: Log,
  tick_start: int, tick_end_pos: int,
)
  requires
    inv_fifo_task_selection(old_log),
    old_log_compatible(old_log, new_log, tick_start),
    tick_end_pos == new_log.len() as int - 1,
    forall |i: int| tick_start <= i <= tick_end_pos && is_poll_task_at(new_log, i) ==> {
      let tid = get_poll_task_id(new_log[i]);
      is_fifo_head_at(new_log, i, tid)
    },
  ensures inv_fifo_task_selection(new_log),
{
  assert forall |i: int| is_poll_task_at(new_log, i) implies ({
    let tid = get_poll_task_id(new_log[i]);
    is_fifo_head_at(new_log, i, tid)
  })
  by {
    if i < old_log.len() as int {
      assert(old_log[i] == new_log[i]);
      assert(is_poll_task_at(old_log, i));
      let tid = get_poll_task_id(old_log[i]);
      assert(get_poll_task_id(new_log[i]) == tid);
      assert(is_fifo_head_at(old_log, i, tid));
      crate::proof::fifo_helpers::fifo_queue_prefix_preserved(old_log, new_log, i);
    }
  }
}

pub proof fn structural_inv_preserved(
  old_log: Log, new_log: Log,
  tick_start: int, deferred_pos: int, first_pop_pos: int,
  park_pos: int, drain_reactor_pos: int, drain_task_pos: int, tick_end_pos: int,
)
  requires
    structural_inv(old_log),
    old_log_compatible(old_log, new_log, tick_start),
    well_formed_tick_segment(new_log, tick_start, deferred_pos, first_pop_pos,
      park_pos, drain_reactor_pos, drain_task_pos, tick_end_pos),
  ensures structural_inv(new_log),
{
  crate::proof::invariants::eq_park_drain_reactor_wake(old_log);
  crate::proof::invariants::eq_tick_has_park(old_log);
  crate::proof::invariants::eq_tick_has_pop_injection(old_log);
  crate::proof::invariants::eq_tick_has_drain_deferred(old_log);
  crate::proof::invariants::eq_tick_has_drain_task_wake(old_log);
  tick_has_park_preserved(old_log, new_log, tick_start, deferred_pos,
    first_pop_pos, park_pos, drain_reactor_pos, drain_task_pos, tick_end_pos);
  tick_has_pop_injection_preserved(old_log, new_log, tick_start, deferred_pos,
    first_pop_pos, park_pos, drain_reactor_pos, drain_task_pos, tick_end_pos);
  tick_has_drain_deferred_preserved(old_log, new_log, tick_start, deferred_pos,
    first_pop_pos, park_pos, drain_reactor_pos, drain_task_pos, tick_end_pos);
  tick_has_drain_task_wake_preserved(old_log, new_log, tick_start, deferred_pos,
    first_pop_pos, park_pos, drain_reactor_pos, drain_task_pos, tick_end_pos);
  park_drain_reactor_wake_preserved(old_log, new_log, tick_start, deferred_pos,
    first_pop_pos, park_pos, drain_reactor_pos, drain_task_pos, tick_end_pos);
  crate::proof::invariants::eq_park_drain_reactor_wake(new_log);
  crate::proof::invariants::eq_tick_has_park(new_log);
  crate::proof::invariants::eq_tick_has_pop_injection(new_log);
  crate::proof::invariants::eq_tick_has_drain_deferred(new_log);
  crate::proof::invariants::eq_tick_has_drain_task_wake(new_log);
}

pub proof fn semantic_inv_preserved(
  old_log: Log, new_log: Log,
  tick_start: int, deferred_pos: int, first_pop_pos: int,
  park_pos: int, drain_reactor_pos: int, drain_task_pos: int, tick_end_pos: int,
  new_tick_has_poll: bool,
)
  requires
    semantic_inv(old_log),
    old_log_compatible(old_log, new_log, tick_start),
    well_formed_tick_segment(new_log, tick_start, deferred_pos, first_pop_pos,
      park_pos, drain_reactor_pos, drain_task_pos, tick_end_pos),
    // E4 for new events
    forall |i: int| tick_start <= i <= tick_end_pos && is_poll_task_at(new_log, i) ==> {
      let tid = get_poll_task_id(new_log[i]);
      let result = get_poll_result(new_log[i]);
      tid_was_injected_before(new_log, i, tid) &&
      (result == PollResult::<()>::Invalid <==> tid_is_invalid(new_log, i, tid))
    },
    // E9 for new events
    forall |i: int| tick_start <= i <= tick_end_pos && is_poll_task_at(new_log, i) ==> {
      let tid = get_poll_task_id(new_log[i]);
      is_fifo_head_at(new_log, i, tid)
    },
    // E2 for new tick
    new_tick_has_poll || fifo_queue_at(new_log, tick_start).len() == 0,
    new_tick_has_poll ==> exists |q: int| #![auto] tick_start < q < tick_end_pos &&
      is_poll_task_at(new_log, q),
  ensures semantic_inv(new_log),
{
  crate::proof::invariants::eq_tick_polls_if_runnable(old_log);
  crate::proof::invariants::eq_poll_within_tick(old_log);
  crate::proof::invariants::eq_valid_task_polling(old_log);
  crate::proof::invariants::eq_fifo_task_selection(old_log);
  poll_within_tick_preserved(old_log, new_log, tick_start, deferred_pos,
    first_pop_pos, park_pos, drain_reactor_pos, drain_task_pos, tick_end_pos);
  valid_task_polling_preserved(old_log, new_log, tick_start, tick_end_pos);
  tick_polls_if_runnable_preserved(old_log, new_log, tick_start, deferred_pos,
    first_pop_pos, park_pos, drain_reactor_pos, drain_task_pos, tick_end_pos,
    new_tick_has_poll);
  fifo_task_selection_preserved(old_log, new_log, tick_start, tick_end_pos);
  crate::proof::invariants::eq_tick_polls_if_runnable(new_log);
  crate::proof::invariants::eq_poll_within_tick(new_log);
  crate::proof::invariants::eq_valid_task_polling(new_log);
  crate::proof::invariants::eq_fifo_task_selection(new_log);
}

pub proof fn executor_inv_preserved(
  old_log: Log, new_log: Log,
  tick_start: int, deferred_pos: int, first_pop_pos: int,
  park_pos: int, drain_reactor_pos: int, drain_task_pos: int, tick_end_pos: int,
  new_tick_has_poll: bool,
)
  requires
    executor_inv(old_log),
    old_log_compatible(old_log, new_log, tick_start),
    well_formed_tick_segment(new_log, tick_start, deferred_pos, first_pop_pos,
      park_pos, drain_reactor_pos, drain_task_pos, tick_end_pos),
    forall |i: int| tick_start <= i <= tick_end_pos && is_poll_task_at(new_log, i) ==> {
      let tid = get_poll_task_id(new_log[i]);
      let result = get_poll_result(new_log[i]);
      tid_was_injected_before(new_log, i, tid) &&
      (result == PollResult::<()>::Invalid <==> tid_is_invalid(new_log, i, tid))
    },
    forall |i: int| tick_start <= i <= tick_end_pos && is_poll_task_at(new_log, i) ==> {
      let tid = get_poll_task_id(new_log[i]);
      is_fifo_head_at(new_log, i, tid)
    },
    new_tick_has_poll || fifo_queue_at(new_log, tick_start).len() == 0,
    new_tick_has_poll ==> exists |q: int| #![auto] tick_start < q < tick_end_pos &&
      is_poll_task_at(new_log, q),
  ensures executor_inv(new_log),
{
  structural_inv_preserved(old_log, new_log, tick_start, deferred_pos,
    first_pop_pos, park_pos, drain_reactor_pos, drain_task_pos, tick_end_pos);
  semantic_inv_preserved(old_log, new_log, tick_start, deferred_pos,
    first_pop_pos, park_pos, drain_reactor_pos, drain_task_pos, tick_end_pos,
    new_tick_has_poll);
}

}
