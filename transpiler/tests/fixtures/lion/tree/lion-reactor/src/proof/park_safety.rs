use vstd::prelude::*;
use crate::spec::log::*;
use crate::spec::types::*;
use crate::spec::predicates::*;
use crate::invariants::*;
use crate::invariants::timer_waker_validity::*;
use crate::invariants::io_waker_validity;
use crate::invariants::io_waker_validity::*;
use crate::invariants::data_inv::*;
use crate::invariants::park_poll_once::*;
use crate::invariants::io_ready_in_park::*;
use crate::proof::preservation::*;
use crate::proof::safety_preservation::*;
use crate::proof::preservation_ext::*;

verus! {

proof fn safety_r7_prefix(l: Log, e: ReactorEvent, i: int)
  requires
    reactor_safety_inv(l),
    is_succ_register_timer_at(l.push(e), i),
    !is_succ_register_timer_at(l.push(e), l.len() as int),
    i < l.len() as int,
  ensures ({
    let l2 = l.push(e);
    let rid = get_register_timer_rid(l2[i]);
    no_prior_timer_registration(l2, rid, i)
  })
{
  let l2 = l.push(e);
  let n = l.len() as int;
  assert(l2[i] == l[i]);
  assert(is_succ_register_timer_at(l, i));
  let rid = get_register_timer_rid(l[i]);
  assert(no_prior_timer_registration(l, rid, i));
  no_prior_timer_reg_preserved(l, e, rid, i);
}

proof fn safety_r8_prefix(l: Log, e: ReactorEvent, i: int)
  requires
    reactor_safety_inv(l),
    io_api_registered_at(l.push(e), i),
    !io_api_registered_at(l.push(e), l.len() as int),
    i < l.len() as int,
  ensures ({
    let l2 = l.push(e);
    let rid = get_io_api_register_rid(l2[i]);
    no_prior_io_api_registration(l2, rid, i)
  })
{
  let l2 = l.push(e);
  let n = l.len() as int;
  assert(l2[i] == l[i]);
  assert(io_api_registered_at(l, i));
  let rid = get_io_api_register_rid(l[i]);
  assert(no_prior_io_api_registration(l, rid, i));
  no_prior_io_api_reg_preserved(l, e, rid, i);
}

proof fn safety_r9a_prefix(l: Log, e: ReactorEvent, i: int)
  requires
    reactor_safety_inv(l),
    is_succ_register_timer_at(l.push(e), i),
    !io_api_registered_at(l.push(e), l.len() as int),
    i < l.len() as int,
  ensures ({
    let l2 = l.push(e);
    let rid = get_register_timer_rid(l2[i]);
    no_io_api_with_rid_before(l2, rid, i)
  })
{
  let l2 = l.push(e);
  assert(l2[i] == l[i]);
  assert(is_succ_register_timer_at(l, i));
  let rid = get_register_timer_rid(l[i]);
  assert(no_io_api_with_rid_before(l, rid, i));
  no_io_api_with_rid_preserved(l, e, rid, i);
}

proof fn safety_r9b_prefix(l: Log, e: ReactorEvent, i: int)
  requires
    reactor_safety_inv(l),
    io_api_registered_at(l.push(e), i),
    !is_succ_register_timer_at(l.push(e), l.len() as int),
    i < l.len() as int,
  ensures ({
    let l2 = l.push(e);
    let rid = get_io_api_register_rid(l2[i]);
    no_timer_with_rid_before(l2, rid, i)
  })
{
  let l2 = l.push(e);
  assert(l2[i] == l[i]);
  assert(io_api_registered_at(l, i));
  let rid = get_io_api_register_rid(l[i]);
  assert(no_timer_with_rid_before(l, rid, i));
  no_timer_with_rid_preserved(l, e, rid, i);
}

proof fn safety_r5_prefix(l: Log, e: ReactorEvent, i: int)
  requires
    reactor_safety_inv(l),
    is_timer_wake_at(l.push(e), i),
    i < l.len() as int,
  ensures ({
    let l2 = l.push(e);
    let rid = get_wake_task_source_rid(l2[i]);
    let waker = get_wake_task_waker(l2[i]);
    exists |j: int| 0 <= j < i &&
      is_succ_register_timer_at(l2, j) &&
      get_register_timer_rid(l2[j]) == rid &&
      get_register_timer_waker(l2[j]) == waker &&
      timer_active_at(l2, j, i)
  })
{
  let l2 = l.push(e);
  let n = l.len() as int;
  assert(l2[i] == l[i]);
  assert(is_wake_task_at(l, i));
  let j_wit = choose |j: int| #![trigger is_succ_register_timer_at(l2, j)]
    0 <= j < i &&
    is_succ_register_timer_at(l2, j) &&
    get_register_timer_rid(l2[j]) == get_wake_task_source_rid(l2[i]) &&
    timer_active_at(l2, j, i);
  assert(j_wit < n);
  assert(l2[j_wit] == l[j_wit]);
  assert(is_succ_register_timer_at(l, j_wit));
  assert(timer_active_at(l2, j_wit, i));
  let rid_wit = get_register_timer_rid(l2[j_wit]);
  assert forall |k: int| j_wit < k < i implies
    !timer_retired_at(l, rid_wit, k)
  by {
    assert(k < n);
    assert(l2[k] == l[k]);
    not_timer_retired_transfer(l2, l, rid_wit, k);
  };
  assert(timer_active_at(l, j_wit, i));
  assert(is_timer_wake_at(l, i));

  let rid = get_wake_task_source_rid(l[i]);
  let waker = get_wake_task_waker(l[i]);
  let j0 = choose |j: int| #![trigger is_succ_register_timer_at(l, j)]
    0 <= j < i &&
    is_succ_register_timer_at(l, j) &&
    get_register_timer_rid(l[j]) == rid &&
    get_register_timer_waker(l[j]) == waker &&
    timer_active_at(l, j, i);
  assert(l2[j0] == l[j0]);
  assert(is_succ_register_timer_at(l2, j0));
  assert forall |k: int| j0 < k < i implies
    !timer_retired_at(l2, rid, k)
  by {
    assert(k < n);
    assert(l2[k] == l[k]);
    not_timer_retired_transfer(l, l2, rid, k);
  };
}

#[verifier::rlimit(30)]
proof fn safety_r6_prefix(l: Log, e: ReactorEvent, i: int)
  requires
    reactor_safety_inv(l),
    is_io_api_wake_at(l.push(e), i),
    i < l.len() as int,
  ensures ({
    let l2 = l.push(e);
    let rid = get_wake_task_source_rid(l2[i]);
    let waker = get_wake_task_waker(l2[i]);
    exists |sw_idx: int| 0 <= sw_idx < i &&
      is_succ_set_waker_at(l2, sw_idx) &&
      get_set_waker_rid(l2[sw_idx]) == rid &&
      get_set_waker_waker(l2[sw_idx]) == waker &&
      io_api_active_at_set_waker(l2, rid, sw_idx)
  })
{
  let l2 = l.push(e);
  let n = l.len() as int;
  assert(l2[i] == l[i]);
  assert(is_wake_task_at(l, i));
  let j_wit = choose |j: int| #![trigger io_api_registered_at(l2, j)]
    0 <= j < i &&
    io_api_registered_at(l2, j) &&
    get_io_api_register_rid(l2[j]) == get_wake_task_source_rid(l2[i]) &&
    io_api_active_at(l2, j, i);
  assert(j_wit < n);
  assert(l2[j_wit] == l[j_wit]);
  assert(io_api_registered_at(l, j_wit));
  assert(io_api_active_at(l2, j_wit, i));
  assert forall |k: int| j_wit < k < i implies !(
    io_api_deregistered_at(l, k) &&
    get_io_api_deregister_rid(l[k]) == get_io_api_register_rid(l[j_wit])
  ) by {
    assert(k < n);
    assert(l2[k] == l[k]);
    assert(l2[j_wit] == l[j_wit]);
    if io_api_deregistered_at(l, k) && get_io_api_deregister_rid(l[k]) == get_io_api_register_rid(l[j_wit]) {
      assert(io_api_deregistered_at(l2, k));
      assert(get_io_api_deregister_rid(l2[k]) == get_io_api_register_rid(l2[j_wit]));
    }
  };
  assert(io_api_active_at(l, j_wit, i));
  assert(is_io_api_wake_at(l, i));

  let rid = get_wake_task_source_rid(l[i]);
  let waker = get_wake_task_waker(l[i]);
  let sw = choose |sw: int| #![trigger is_succ_set_waker_at(l, sw)]
    0 <= sw < i &&
    is_succ_set_waker_at(l, sw) &&
    get_set_waker_rid(l[sw]) == rid &&
    get_set_waker_waker(l[sw]) == waker &&
    io_api_active_at_set_waker(l, rid, sw);
  assert(l2[sw] == l[sw]);
  assert(is_succ_set_waker_at(l2, sw));
  let reg = choose |reg: int| 0 <= reg < sw &&
    io_api_registered_at(l, reg) &&
    get_io_api_register_rid(l[reg]) == rid &&
    io_api_active_at(l, reg, sw);
  assert(l2[reg] == l[reg]);
  assert forall |k: int| reg < k < sw implies !(
    io_api_deregistered_at(l2, k) &&
    get_io_api_deregister_rid(l2[k]) == get_io_api_register_rid(l2[reg])
  ) by {
    assert(k < n);
    assert(l2[k] == l[k]);
    if io_api_deregistered_at(l2, k) {
      assert(io_api_deregistered_at(l, k));
    }
  };
  assert(io_api_active_at(l2, reg, sw));
  assert(io_api_active_at_set_waker(l2, rid, sw));
}

proof fn safety_r14_prefix(l: Log, e: ReactorEvent, i: int)
  requires
    reactor_safety_inv(l),
    is_wake_task_at(l.push(e), i),
    i < l.len() as int,
  ensures ({
    let l2 = l.push(e);
    let rid = get_wake_task_source_rid(l2[i]);
    (exists |j: int| 0 <= j < i &&
      is_succ_register_timer_at(l2, j) &&
      get_register_timer_rid(l2[j]) == rid)
    ||
    (exists |j: int| 0 <= j < i &&
      io_api_registered_at(l2, j) &&
      get_io_api_register_rid(l2[j]) == rid)
  })
{
  let l2 = l.push(e);
  let n = l.len() as int;
  assert(l2[i] == l[i]);
  assert(is_wake_task_at(l, i));
  let rid = get_wake_task_source_rid(l[i]);
  if exists |j: int| 0 <= j < i && is_succ_register_timer_at(l, j) &&
    get_register_timer_rid(l[j]) == rid
  {
    let j = choose |j: int| 0 <= j < i && is_succ_register_timer_at(l, j) &&
      get_register_timer_rid(l[j]) == rid;
    assert(l2[j] == l[j]);
    assert(is_succ_register_timer_at(l2, j));
  } else {
    let j = choose |j: int| 0 <= j < i && io_api_registered_at(l, j) &&
      get_io_api_register_rid(l[j]) == rid;
    assert(l2[j] == l[j]);
    assert(io_api_registered_at(l2, j));
  }
}

#[verifier::rlimit(40)]
pub proof fn reactor_safety_inv_preserved_by_non_wake_non_register(l: Log, e: ReactorEvent)
  requires
    reactor_safety_inv(l),
    !is_wake_task_at(l.push(e), l.len() as int),
    !is_succ_register_timer_at(l.push(e), l.len() as int),
    !io_api_registered_at(l.push(e), l.len() as int),
  ensures
    reactor_safety_inv(l.push(e)),
{
  let l2 = l.push(e);
  let n = l.len() as int;

  assert forall |i: int| #![auto] is_succ_register_timer_at(l2, i) implies {
    let rid = get_register_timer_rid(l2[i]);
    no_prior_timer_registration(l2, rid, i)
  } by { assert(i < n); safety_r7_prefix(l, e, i); }

  assert forall |i: int| #![auto] io_api_registered_at(l2, i) implies {
    let rid = get_io_api_register_rid(l2[i]);
    no_prior_io_api_registration(l2, rid, i)
  } by { assert(i < n); safety_r8_prefix(l, e, i); }

  assert forall |i: int| #![auto] is_succ_register_timer_at(l2, i) implies {
    let rid = get_register_timer_rid(l2[i]);
    no_io_api_with_rid_before(l2, rid, i)
  } by { assert(i < n); safety_r9a_prefix(l, e, i); }

  assert forall |i: int| #![auto] io_api_registered_at(l2, i) implies {
    let rid = get_io_api_register_rid(l2[i]);
    no_timer_with_rid_before(l2, rid, i)
  } by { assert(i < n); safety_r9b_prefix(l, e, i); }

  assert forall |i: int| #![auto] is_timer_wake_at(l2, i) implies {
    let rid = get_wake_task_source_rid(l2[i]);
    let waker = get_wake_task_waker(l2[i]);
    exists |j: int| 0 <= j < i &&
      is_succ_register_timer_at(l2, j) &&
      get_register_timer_rid(l2[j]) == rid &&
      get_register_timer_waker(l2[j]) == waker &&
      timer_active_at(l2, j, i)
  } by { assert(i < n); safety_r5_prefix(l, e, i); }

  assert forall |i: int| #![auto] is_io_api_wake_at(l2, i) implies {
    let rid = get_wake_task_source_rid(l2[i]);
    let waker = get_wake_task_waker(l2[i]);
    exists |sw_idx: int| 0 <= sw_idx < i &&
      is_succ_set_waker_at(l2, sw_idx) &&
      get_set_waker_rid(l2[sw_idx]) == rid &&
      get_set_waker_waker(l2[sw_idx]) == waker &&
      io_api_active_at_set_waker(l2, rid, sw_idx)
  } by { assert(i < n); safety_r6_prefix(l, e, i); }

  assert forall |i: int| #![auto] is_wake_task_at(l2, i) implies {
    let rid = get_wake_task_source_rid(l2[i]);
    (exists |j: int| 0 <= j < i &&
      is_succ_register_timer_at(l2, j) &&
      get_register_timer_rid(l2[j]) == rid)
    ||
    (exists |j: int| 0 <= j < i &&
      io_api_registered_at(l2, j) &&
      get_io_api_register_rid(l2[j]) == rid)
  } by { assert(i < n); safety_r14_prefix(l, e, i); }
}

#[verifier::rlimit(40)]
pub proof fn reactor_safety_inv_preserved_by_wake_task(
  l: Log,
  e: ReactorEvent,
  timer_reg_witness: Option<int>,
  io_reg_witness: Option<int>,
)
  requires
    reactor_safety_inv(l),
    is_wake_task_at(l.push(e), l.len() as int),
    !is_succ_register_timer_at(l.push(e), l.len() as int),
    !io_api_registered_at(l.push(e), l.len() as int),
    !is_deregister_timer_at(l.push(e), l.len() as int),
    !io_api_deregistered_at(l.push(e), l.len() as int),
    is_timer_wake_at(l.push(e), l.len() as int) ==> {
      let rid = get_wake_task_source_rid(l.push(e)[l.len() as int]);
      let waker = get_wake_task_waker(l.push(e)[l.len() as int]);
      exists |j: int| 0 <= j < l.len() &&
        is_succ_register_timer_at(l, j) &&
        get_register_timer_rid(l[j]) == rid &&
        get_register_timer_waker(l[j]) == waker &&
        timer_active_at(l, j, l.len() as int)
    },
    io_waker_validity::is_io_api_wake_at(l.push(e), l.len() as int) ==> {
      let rid = get_wake_task_source_rid(l.push(e)[l.len() as int]);
      let waker = get_wake_task_waker(l.push(e)[l.len() as int]);
      exists |sw_idx: int| 0 <= sw_idx < l.len() &&
        is_succ_set_waker_at(l, sw_idx) &&
        get_set_waker_rid(l[sw_idx]) == rid &&
        get_set_waker_waker(l[sw_idx]) == waker &&
        io_api_active_at_set_waker(l, rid, sw_idx)
    },
    ({
      let rid = get_wake_task_source_rid(l.push(e)[l.len() as int]);
      match timer_reg_witness {
        Some(j) => 0 <= j < l.len() as int &&
          is_succ_register_timer_at(l, j) &&
          get_register_timer_rid(l[j]) == rid,
        None => true,
      }
    }),
    ({
      let rid = get_wake_task_source_rid(l.push(e)[l.len() as int]);
      match io_reg_witness {
        Some(j) => 0 <= j < l.len() as int &&
          io_api_registered_at(l, j) &&
          get_io_api_register_rid(l[j]) == rid,
        None => true,
      }
    }),
    timer_reg_witness.is_some() || io_reg_witness.is_some(),
  ensures
    reactor_safety_inv(l.push(e)),
{
  let l2 = l.push(e);
  let n = l.len() as int;

  // R7
  assert forall |i: int| #![auto] is_succ_register_timer_at(l2, i) implies {
    let rid = get_register_timer_rid(l2[i]);
    no_prior_timer_registration(l2, rid, i)
  } by { assert(i < n); safety_r7_prefix(l, e, i); }

  // R8
  assert forall |i: int| #![auto] io_api_registered_at(l2, i) implies {
    let rid = get_io_api_register_rid(l2[i]);
    no_prior_io_api_registration(l2, rid, i)
  } by { assert(i < n); safety_r8_prefix(l, e, i); }

  // R9a
  assert forall |i: int| #![auto] is_succ_register_timer_at(l2, i) implies {
    let rid = get_register_timer_rid(l2[i]);
    no_io_api_with_rid_before(l2, rid, i)
  } by { assert(i < n); safety_r9a_prefix(l, e, i); }

  // R9b
  assert forall |i: int| #![auto] io_api_registered_at(l2, i) implies {
    let rid = get_io_api_register_rid(l2[i]);
    no_timer_with_rid_before(l2, rid, i)
  } by { assert(i < n); safety_r9b_prefix(l, e, i); }

  // R5: timer waker validity
  assert forall |i: int| #![auto] is_timer_wake_at(l2, i) implies {
    let rid = get_wake_task_source_rid(l2[i]);
    let waker = get_wake_task_waker(l2[i]);
    exists |j: int| 0 <= j < i &&
      is_succ_register_timer_at(l2, j) &&
      get_register_timer_rid(l2[j]) == rid &&
      get_register_timer_waker(l2[j]) == waker &&
      timer_active_at(l2, j, i)
  } by {
    if i < n {
      safety_r5_prefix(l, e, i);
    } else {
      assert(i == n);
      let rid = get_wake_task_source_rid(l2[n]);
      let waker = get_wake_task_waker(l2[n]);
      let j0 = choose |j: int| 0 <= j < n &&
        is_succ_register_timer_at(l, j) &&
        get_register_timer_rid(l[j]) == rid &&
        get_register_timer_waker(l[j]) == waker &&
        timer_active_at(l, j, n);
      assert(l2[j0] == l[j0]);
      assert(is_succ_register_timer_at(l2, j0));
      assert forall |k: int| j0 < k < n implies
        !timer_retired_at(l2, rid, k)
      by {
        assert(l2[k] == l[k]);
        not_timer_retired_transfer(l, l2, rid, k);
      };
      assert(timer_active_at(l2, j0, n));
    }
  }

  // R6: IO waker validity
  assert forall |i: int| #![auto] io_waker_validity::is_io_api_wake_at(l2, i) implies {
    let rid = get_wake_task_source_rid(l2[i]);
    let waker = get_wake_task_waker(l2[i]);
    exists |sw_idx: int| 0 <= sw_idx < i &&
      is_succ_set_waker_at(l2, sw_idx) &&
      get_set_waker_rid(l2[sw_idx]) == rid &&
      get_set_waker_waker(l2[sw_idx]) == waker &&
      io_api_active_at_set_waker(l2, rid, sw_idx)
  } by {
    if i < n {
      safety_r6_prefix(l, e, i);
    } else {
      assert(i == n);
      let rid = get_wake_task_source_rid(l2[n]);
      let waker = get_wake_task_waker(l2[n]);
      let sw = choose |sw: int| 0 <= sw < n &&
        is_succ_set_waker_at(l, sw) &&
        get_set_waker_rid(l[sw]) == rid &&
        get_set_waker_waker(l[sw]) == waker &&
        io_api_active_at_set_waker(l, rid, sw);
      assert(l2[sw] == l[sw]);
      assert(is_succ_set_waker_at(l2, sw));
      let reg = choose |reg: int| 0 <= reg < sw &&
        io_api_registered_at(l, reg) &&
        get_io_api_register_rid(l[reg]) == rid &&
        io_api_active_at(l, reg, sw);
      assert(l2[reg] == l[reg]);
      assert forall |k: int| reg < k < sw implies !(
        io_api_deregistered_at(l2, k) &&
        get_io_api_deregister_rid(l2[k]) == get_io_api_register_rid(l2[reg])
      ) by {
        assert(k < n);
        assert(l2[k] == l[k]);
        if io_api_deregistered_at(l2, k) {
          assert(io_api_deregistered_at(l, k));
        }
      };
      assert(io_api_active_at(l2, reg, sw));
      assert(io_api_active_at_set_waker(l2, rid, sw));
    }
  }

  // R14: wake_has_registration
  assert forall |i: int| #![auto] is_wake_task_at(l2, i) implies {
    let rid = get_wake_task_source_rid(l2[i]);
    (exists |j: int| 0 <= j < i &&
      is_succ_register_timer_at(l2, j) &&
      get_register_timer_rid(l2[j]) == rid)
    ||
    (exists |j: int| 0 <= j < i &&
      io_api_registered_at(l2, j) &&
      get_io_api_register_rid(l2[j]) == rid)
  } by {
    if i < n {
      safety_r14_prefix(l, e, i);
    } else {
      assert(i == n);
      let rid = get_wake_task_source_rid(l2[n]);
      if timer_reg_witness.is_some() {
        let j = timer_reg_witness.unwrap();
        assert(l2[j] == l[j]);
        assert(is_succ_register_timer_at(l2, j));
        assert(get_register_timer_rid(l2[j]) == rid);
      } else {
        let j = io_reg_witness.unwrap();
        assert(l2[j] == l[j]);
        assert(io_api_registered_at(l2, j));
        assert(get_io_api_register_rid(l2[j]) == rid);
      }
    }
  }
}

pub proof fn current_park_start_no_park_in_range(l: Log, from_pos: int, to_pos: int)
  requires
    0 <= from_pos <= to_pos,
    to_pos <= l.len(),
    forall |k: int| from_pos <= k < to_pos ==> !is_park_begin_at(l, k) && !is_park_end_at(l, k),
  ensures
    current_park_start(l, to_pos) == current_park_start(l, from_pos),
  decreases to_pos - from_pos,
{
  if to_pos > from_pos {
    current_park_start_no_park_in_range(l, from_pos, to_pos - 1);
  }
}

pub proof fn current_park_start_agrees_on_prefix(l1: Log, l2: Log, i: int)
  requires
    0 <= i,
    i <= l1.len(),
    i <= l2.len(),
    forall |k: int| 0 <= k < i ==> l1[k] == l2[k],
  ensures
    current_park_start(l1, i) == current_park_start(l2, i),
  decreases i,
{
  if i > 0 {
    assert(l1[i-1] == l2[i-1]);
    if !is_park_begin_at(l1, i-1) && !is_park_end_at(l1, i-1) {
      current_park_start_agrees_on_prefix(l1, l2, i - 1);
    }
  }
}

pub proof fn count_poll_events_zero_when_no_poll(l: Log, start: int, end: int)
  requires
    0 <= start,
    end <= l.len(),
    forall |k: int| start <= k < end ==> !is_poll_events_at(l, k),
  ensures
    count_poll_events_in_range(l, start, end) == 0nat,
  decreases end - start,
{
  if start < end {
    count_poll_events_zero_when_no_poll(l, start + 1, end);
  }
}

pub proof fn count_poll_events_agrees_on_prefix(l1: Log, l2: Log, start: int, end: int)
  requires
    0 <= start,
    end <= l1.len(),
    end <= l2.len(),
    forall |k: int| start <= k < end ==> l1[k] == l2[k],
  ensures
    count_poll_events_in_range(l1, start, end) == count_poll_events_in_range(l2, start, end),
  decreases end - start,
{
  if start < end {
    assert(l1[start] == l2[start]);
    count_poll_events_agrees_on_prefix(l1, l2, start + 1, end);
  }
}

proof fn event_not_registration_at(l: Log, k: int)
  requires
    is_park_begin_at(l, k) || is_get_current_time_at(l, k) || is_poll_events_at(l, k) || is_park_end_at(l, k),
  ensures
    !is_succ_register_timer_at(l, k),
    !is_deregister_timer_at(l, k),
    !io_api_registered_at(l, k),
    !io_api_deregistered_at(l, k),
    !is_succ_set_waker_at(l, k),
    !is_io_event_ready_at(l, k),
{}

proof fn event_transfer(l1: Log, l2: Log, k: int)
  requires
    0 <= k < l1.len(),
    k < l2.len(),
    l1[k] == l2[k],
    !is_succ_register_timer_at(l1, k),
    !is_deregister_timer_at(l1, k),
    !io_api_registered_at(l1, k),
    !io_api_deregistered_at(l1, k),
    !is_succ_set_waker_at(l1, k),
    !is_io_event_ready_at(l1, k),
  ensures
    !is_succ_register_timer_at(l2, k),
    !is_deregister_timer_at(l2, k),
    !io_api_registered_at(l2, k),
    !io_api_deregistered_at(l2, k),
    !is_succ_set_waker_at(l2, k),
    !is_io_event_ready_at(l2, k),
{}

#[verifier::rlimit(40)]
pub proof fn park_error_path_no_registrations(
  log3: Log,
  log_wet: Log,
  log_final: Log,
  base: int,
)
  requires
    log3.len() == base + 3,
    base >= 0,
    log_wet.len() >= log3.len(),
    log_final.len() == log_wet.len() + 1,
    forall |k: int| #![auto] 0 <= k < log_wet.len() ==> log_final[k] == log_wet[k],
    forall |k: int| #![auto] 0 <= k < log3.len() ==> log_wet[k] == log3[k],
    is_park_begin_at(log3, base),
    is_get_current_time_at(log3, base + 1),
    is_poll_events_at(log3, base + 2),
    forall |k: int| #![auto] log3.len() as int <= k < log_wet.len() ==>
      !is_succ_register_timer_at(log_wet, k) &&
      !is_deregister_timer_at(log_wet, k) &&
      !io_api_registered_at(log_wet, k) &&
      !io_api_deregistered_at(log_wet, k) &&
      !is_succ_set_waker_at(log_wet, k) &&
      !is_io_event_ready_at(log_wet, k),
    is_park_end_at(log_final, log_wet.len() as int),
  ensures
    forall |k: int| #![auto] base <= k < log_final.len() ==>
      !is_succ_register_timer_at(log_final, k) &&
      !is_deregister_timer_at(log_final, k) &&
      !io_api_registered_at(log_final, k) &&
      !io_api_deregistered_at(log_final, k) &&
      !is_succ_set_waker_at(log_final, k) &&
      !is_io_event_ready_at(log_final, k),
{
  assert forall |k: int| #![auto] base <= k < log_final.len() implies
    !is_succ_register_timer_at(log_final, k) &&
    !is_deregister_timer_at(log_final, k) &&
    !io_api_registered_at(log_final, k) &&
    !io_api_deregistered_at(log_final, k) &&
    !is_succ_set_waker_at(log_final, k) &&
    !is_io_event_ready_at(log_final, k)
  by {
    if k < log_wet.len() as int {
      assert(log_final[k] == log_wet[k]);
      if k < log3.len() as int {
        assert(log_wet[k] == log3[k]);
        if k == base {
          event_not_registration_at(log3, k);
          event_transfer(log3, log_final, k);
        } else if k == base + 1 {
          event_not_registration_at(log3, k);
          event_transfer(log3, log_final, k);
        } else {
          event_not_registration_at(log3, k);
          event_transfer(log3, log_final, k);
        }
      } else {
        event_transfer(log_wet, log_final, k);
      }
    } else {
      event_not_registration_at(log_final, k);
    }
  };
}

proof fn event_not_registration5(l: Log, k: int)
  requires
    is_park_begin_at(l, k) || is_get_current_time_at(l, k) || is_poll_events_at(l, k) || is_park_end_at(l, k),
  ensures
    !is_succ_register_timer_at(l, k),
    !is_deregister_timer_at(l, k),
    !io_api_registered_at(l, k),
    !io_api_deregistered_at(l, k),
    !is_succ_set_waker_at(l, k),
{}

proof fn event_transfer5(l1: Log, l2: Log, k: int)
  requires
    0 <= k < l1.len(),
    k < l2.len(),
    l1[k] == l2[k],
    !is_succ_register_timer_at(l1, k),
    !is_deregister_timer_at(l1, k),
    !io_api_registered_at(l1, k),
    !io_api_deregistered_at(l1, k),
    !is_succ_set_waker_at(l1, k),
  ensures
    !is_succ_register_timer_at(l2, k),
    !is_deregister_timer_at(l2, k),
    !io_api_registered_at(l2, k),
    !io_api_deregistered_at(l2, k),
    !is_succ_set_waker_at(l2, k),
{}

#[verifier::rlimit(40)]
pub proof fn park_normal_path_no_registrations(
  log3: Log,
  log_wet: Log,
  log_pio: Log,
  log_final: Log,
  base: int,
)
  requires
    log3.len() == base + 3,
    base >= 0,
    log_wet.len() >= log3.len(),
    log_pio.len() >= log_wet.len(),
    log_final.len() == log_pio.len() + 1,
    forall |k: int| #![auto] 0 <= k < log_pio.len() ==> log_final[k] == log_pio[k],
    forall |k: int| #![auto] 0 <= k < log_wet.len() ==> log_pio[k] == log_wet[k],
    forall |k: int| #![auto] 0 <= k < log3.len() ==> log_wet[k] == log3[k],
    is_park_begin_at(log3, base),
    is_get_current_time_at(log3, base + 1),
    is_poll_events_at(log3, base + 2),
    forall |k: int| #![auto] log3.len() as int <= k < log_wet.len() ==>
      !is_succ_register_timer_at(log_wet, k) &&
      !is_deregister_timer_at(log_wet, k) &&
      !io_api_registered_at(log_wet, k) &&
      !io_api_deregistered_at(log_wet, k) &&
      !is_succ_set_waker_at(log_wet, k),
    forall |k: int| #![auto] log_wet.len() as int <= k < log_pio.len() ==>
      !is_succ_register_timer_at(log_pio, k) &&
      !is_deregister_timer_at(log_pio, k) &&
      !io_api_registered_at(log_pio, k) &&
      !io_api_deregistered_at(log_pio, k) &&
      !is_succ_set_waker_at(log_pio, k),
    is_park_end_at(log_final, log_pio.len() as int),
  ensures
    forall |k: int| #![auto] base <= k < log_final.len() ==>
      !is_succ_register_timer_at(log_final, k) &&
      !is_deregister_timer_at(log_final, k) &&
      !io_api_registered_at(log_final, k) &&
      !io_api_deregistered_at(log_final, k) &&
      !is_succ_set_waker_at(log_final, k),
{
  assert forall |k: int| #![auto] base <= k < log_final.len() implies
    !is_succ_register_timer_at(log_final, k) &&
    !is_deregister_timer_at(log_final, k) &&
    !io_api_registered_at(log_final, k) &&
    !io_api_deregistered_at(log_final, k) &&
    !is_succ_set_waker_at(log_final, k)
  by {
    if k < log_pio.len() as int {
      assert(log_final[k] == log_pio[k]);
      if k < log_wet.len() as int {
        assert(log_pio[k] == log_wet[k]);
        if k < log3.len() as int {
          assert(log_wet[k] == log3[k]);
          if k == base {
            event_not_registration5(log3, k);
            event_transfer5(log3, log_final, k);
          } else if k == base + 1 {
            event_not_registration5(log3, k);
            event_transfer5(log3, log_final, k);
          } else {
            event_not_registration5(log3, k);
            event_transfer5(log3, log_final, k);
          }
        } else {
          event_transfer5(log_wet, log_final, k);
        }
      } else {
        event_transfer5(log_pio, log_final, k);
      }
    } else {
      event_not_registration5(log_final, k);
    }
  };
}

#[verifier::rlimit(200)]
pub proof fn park_io_event_pairing_read(
  log3: Log,
  log_wet: Log,
  log_pre_end: Log,
  log_final: Log,
  base: int,
  rw: Map<ResourceIdView, WakerView>,
)
  requires
    log3.len() == base + 3,
    base >= 0,
    log_wet.len() >= log3.len(),
    log_pre_end.len() >= log_wet.len(),
    log_final.len() == log_pre_end.len() + 1,
    forall |k: int| #![auto] 0 <= k < log_pre_end.len() ==> log_final[k] == log_pre_end[k],
    forall |k: int| #![auto] 0 <= k < log_wet.len() ==> log_pre_end[k] == log_wet[k],
    forall |k: int| #![auto] 0 <= k < log3.len() ==> log_wet[k] == log3[k],
    is_park_begin_at(log3, base),
    is_get_current_time_at(log3, base + 1),
    is_poll_events_at(log3, base + 2),
    forall |k: int| #![auto] log3.len() as int <= k < log_wet.len() ==>
      !is_io_event_ready_at(log_wet, k),
    forall |p: int| #![trigger is_io_event_ready_at(log_pre_end, p)]
      log_wet.len() as int <= p < log_pre_end.len() &&
      is_io_event_ready_at(log_pre_end, p) &&
      get_io_event(log_pre_end[p]).readable &&
      rw.contains_key(get_io_event(log_pre_end[p]).resource_id) ==> {
        let rid = get_io_event(log_pre_end[p]).resource_id;
        p + 1 < log_pre_end.len() as int &&
        is_wake_task_at(log_pre_end, p + 1) &&
        get_wake_task_source_rid(log_pre_end[p + 1]) == rid &&
        get_wake_task_waker(log_pre_end[p + 1]) == rw[rid]
      },
  ensures
    forall |p: int| #![auto] base <= p < log_pre_end.len() as int &&
      is_io_event_ready_at(log_final, p) &&
      get_io_event(log_final[p]).readable &&
      rw.contains_key(get_io_event(log_final[p]).resource_id) ==> {
        let rid = get_io_event(log_final[p]).resource_id;
        p + 1 < log_final.len() as int &&
        is_wake_task_at(log_final, p + 1) &&
        get_wake_task_source_rid(log_final[p + 1]) == rid &&
        get_wake_task_waker(log_final[p + 1]) == rw[rid]
      },
{
  assert forall |p: int| #![auto] base <= p < log_pre_end.len() as int &&
    is_io_event_ready_at(log_final, p) &&
    get_io_event(log_final[p]).readable &&
    rw.contains_key(get_io_event(log_final[p]).resource_id) implies {
      let rid = get_io_event(log_final[p]).resource_id;
      p + 1 < log_final.len() as int &&
      is_wake_task_at(log_final, p + 1) &&
      get_wake_task_source_rid(log_final[p + 1]) == rid &&
      get_wake_task_waker(log_final[p + 1]) == rw[rid]
    }
  by {
    assert(log_final[p] == log_pre_end[p]);
    if p >= log_wet.len() as int {
      assert(is_io_event_ready_at(log_pre_end, p));
      assert(log_final[p + 1] == log_pre_end[p + 1]);
    } else {
      assert(log_pre_end[p] == log_wet[p]);
      if p >= log3.len() as int {
        assert(!is_io_event_ready_at(log_wet, p));
      } else {
        assert(log_wet[p] == log3[p]);
        event_not_registration_at(log3, p);
      }
      assert(!is_io_event_ready_at(log_final, p));
    }
  };
}

#[verifier::rlimit(200)]
pub proof fn park_io_event_pairing_write(
  log3: Log,
  log_wet: Log,
  log_pre_end: Log,
  log_final: Log,
  base: int,
  ww: Map<ResourceIdView, WakerView>,
)
  requires
    log3.len() == base + 3,
    base >= 0,
    log_wet.len() >= log3.len(),
    log_pre_end.len() >= log_wet.len(),
    log_final.len() == log_pre_end.len() + 1,
    forall |k: int| #![auto] 0 <= k < log_pre_end.len() ==> log_final[k] == log_pre_end[k],
    forall |k: int| #![auto] 0 <= k < log_wet.len() ==> log_pre_end[k] == log_wet[k],
    forall |k: int| #![auto] 0 <= k < log3.len() ==> log_wet[k] == log3[k],
    is_park_begin_at(log3, base),
    is_get_current_time_at(log3, base + 1),
    is_poll_events_at(log3, base + 2),
    forall |k: int| #![auto] log3.len() as int <= k < log_wet.len() ==>
      !is_io_event_ready_at(log_wet, k),
    forall |p: int| #![trigger is_io_event_ready_at(log_pre_end, p)]
      log_wet.len() as int <= p < log_pre_end.len() &&
      is_io_event_ready_at(log_pre_end, p) &&
      get_io_event(log_pre_end[p]).writable &&
      ww.contains_key(get_io_event(log_pre_end[p]).resource_id) ==> {
        let rid = get_io_event(log_pre_end[p]).resource_id;
        p + 1 < log_pre_end.len() as int &&
        is_wake_task_at(log_pre_end, p + 1) &&
        get_wake_task_source_rid(log_pre_end[p + 1]) == rid &&
        get_wake_task_waker(log_pre_end[p + 1]) == ww[rid]
      },
  ensures
    forall |p: int| #![auto] base <= p < log_pre_end.len() as int &&
      is_io_event_ready_at(log_final, p) &&
      get_io_event(log_final[p]).writable &&
      ww.contains_key(get_io_event(log_final[p]).resource_id) ==> {
        let rid = get_io_event(log_final[p]).resource_id;
        p + 1 < log_final.len() as int &&
        is_wake_task_at(log_final, p + 1) &&
        get_wake_task_source_rid(log_final[p + 1]) == rid &&
        get_wake_task_waker(log_final[p + 1]) == ww[rid]
      },
{
  assert forall |p: int| #![auto] base <= p < log_pre_end.len() as int &&
    is_io_event_ready_at(log_final, p) &&
    get_io_event(log_final[p]).writable &&
    ww.contains_key(get_io_event(log_final[p]).resource_id) implies {
      let rid = get_io_event(log_final[p]).resource_id;
      p + 1 < log_final.len() as int &&
      is_wake_task_at(log_final, p + 1) &&
      get_wake_task_source_rid(log_final[p + 1]) == rid &&
      get_wake_task_waker(log_final[p + 1]) == ww[rid]
    }
  by {
    assert(log_final[p] == log_pre_end[p]);
    if p >= log_wet.len() as int {
      assert(is_io_event_ready_at(log_pre_end, p));
      assert(log_final[p + 1] == log_pre_end[p + 1]);
    } else {
      assert(log_pre_end[p] == log_wet[p]);
      if p >= log3.len() as int {
        assert(!is_io_event_ready_at(log_wet, p));
      } else {
        assert(log_wet[p] == log3[p]);
        event_not_registration_at(log3, p);
      }
      assert(!is_io_event_ready_at(log_final, p));
    }
  };
}

pub proof fn io_event_ready_step_inv(
  l: Log, e: ReactorEvent,
  ts: Set<(InstantView, ResourceIdView, int)>,
  tm: Map<ResourceIdView, (InstantView, ResourceIdView, int)>,
  tw: Map<ResourceIdView, WakerView>,
  rw: Map<ResourceIdView, WakerView>,
  ww: Map<ResourceIdView, WakerView>,
  next_rid: nat,
)
  requires
    reactor_safety_inv(l),
    reactor_ext_inv(l),
    alloc_inv(l, next_rid),
    data_inv(ts, tm, tw, rw, ww, l),
    is_io_event_ready_at(l.push(e), l.len() as int),
    current_park_start(l, l.len() as int) >= 0,
  ensures
    reactor_safety_inv(l.push(e)),
    reactor_ext_inv(l.push(e)),
    alloc_inv(l.push(e), next_rid),
    data_inv(ts, tm, tw, rw, ww, l.push(e)),
    current_park_start(l.push(e), (l.len() + 1) as int) >= 0,
{
  data_inv_preserved_by_harmless_event(ts, tm, tw, rw, ww, l, e);
  reactor_safety_inv_preserved_by_non_wake_non_register(l, e);
  current_park_start_push(l, e, l.len() as int);
  assert(is_in_park_cycle(l.push(e), l.len() as int));
  reactor_ext_inv_preserved_by_io_event_ready(l, e);
  alloc_inv_preserved_by_non_registration(l, e, next_rid);
  park_start_preserved_by_non_park(l, e);
}

#[verifier::rlimit(200)]
pub proof fn io_wake_step_inv(
  l: Log, e: ReactorEvent,
  ts: Set<(InstantView, ResourceIdView, int)>,
  tm: Map<ResourceIdView, (InstantView, ResourceIdView, int)>,
  tw: Map<ResourceIdView, WakerView>,
  rw: Map<ResourceIdView, WakerView>,
  ww: Map<ResourceIdView, WakerView>,
  next_rid: nat,
  io_rid: ResourceIdView,
  is_read_wake: bool,
)
  requires
    reactor_safety_inv(l),
    reactor_ext_inv(l),
    alloc_inv(l, next_rid),
    data_inv(ts, tm, tw, rw, ww, l),
    is_wake_task_at(l.push(e), l.len() as int),
    get_wake_task_source_rid(l.push(e)[l.len() as int]) == io_rid,
    current_park_start(l, l.len() as int) >= 0,
    is_read_wake ==> (rw.contains_key(io_rid) && get_wake_task_waker(l.push(e)[l.len() as int]) == rw[io_rid]),
    !is_read_wake ==> (ww.contains_key(io_rid) && get_wake_task_waker(l.push(e)[l.len() as int]) == ww[io_rid]),
    io_currently_active(l, io_rid),
  ensures
    reactor_safety_inv(l.push(e)),
    reactor_ext_inv(l.push(e)),
    alloc_inv(l.push(e), next_rid),
    data_inv(ts, tm, tw, rw, ww, l.push(e)),
    current_park_start(l.push(e), (l.len() + 1) as int) >= 0,
{
  let l2 = l.push(e);
  let n = l.len() as int;

  assert(read_wakers_valid(rw, l));
  assert(write_wakers_valid(ww, l));

  let sw_idx: int = if is_read_wake {
    choose |sw: int| 0 <= sw < l.len() &&
      is_succ_set_waker_at(l, sw) &&
      get_set_waker_rid(l[sw]) == io_rid &&
      get_set_waker_interest(l[sw]).0 &&
      get_set_waker_waker(l[sw]) == rw[io_rid] &&
      io_api_active_at_set_waker(l, io_rid, sw)
  } else {
    choose |sw: int| 0 <= sw < l.len() &&
      is_succ_set_waker_at(l, sw) &&
      get_set_waker_rid(l[sw]) == io_rid &&
      get_set_waker_interest(l[sw]).1 &&
      get_set_waker_waker(l[sw]) == ww[io_rid] &&
      io_api_active_at_set_waker(l, io_rid, sw)
  };

  assert(0 <= sw_idx < l.len() as int);
  assert(is_succ_set_waker_at(l, sw_idx));
  assert(io_api_active_at_set_waker(l, io_rid, sw_idx));

  let io_reg_idx: int = choose |reg: int| 0 <= reg < sw_idx &&
    io_api_registered_at(l, reg) &&
    get_io_api_register_rid(l[reg]) == io_rid &&
    io_api_active_at(l, reg, sw_idx);

  assert(0 <= io_reg_idx < sw_idx);
  assert(io_api_registered_at(l, io_reg_idx));
  assert(get_io_api_register_rid(l[io_reg_idx]) == io_rid);
  assert(io_api_active_at(l, io_reg_idx, sw_idx));

  assert(get_wake_task_source_rid(e) == io_rid);
  timer_data_inv_preserved_by_io_wake(ts, tm, tw, l, e, io_rid);
  read_wakers_valid_preserved_by_non_set_waker(rw, l, e);
  write_wakers_valid_preserved_by_non_set_waker(ww, l, e);
  read_wakers_complete_preserved_by_non_trigger(rw, l, e);
  write_wakers_complete_preserved_by_non_trigger(ww, l, e);

  let io_reg_a = choose |r: int| 0 <= r < n &&
    io_api_registered_at(l, r) &&
    get_io_api_register_rid(l[r]) == io_rid &&
    io_api_active_at(l, r, n);
  assert(io_api_active_at(l, io_reg_a, n));

  assert(l2[io_reg_a] == l[io_reg_a]);
  assert(io_api_registered_at(l2, io_reg_a));
  assert forall |k: int| io_reg_a < k < n implies !(
    io_api_deregistered_at(l2, k) &&
    get_io_api_deregister_rid(l2[k]) == get_io_api_register_rid(l2[io_reg_a])
  ) by {
    assert(l2[k] == l[k]);
    assert(l2[io_reg_a] == l[io_reg_a]);
    if io_api_deregistered_at(l, k) && get_io_api_deregister_rid(l[k]) == get_io_api_register_rid(l[io_reg_a]) {
      assert(io_api_deregistered_at(l2, k));
    }
  };
  assert(io_api_active_at(l2, io_reg_a, n));
  assert(io_waker_validity::is_io_api_wake_at(l2, n));

  assert(l2[sw_idx] == l[sw_idx]);
  assert(is_succ_set_waker_at(l2, sw_idx));
  assert forall |k: int| io_reg_idx < k < sw_idx implies !(
    io_api_deregistered_at(l2, k) &&
    get_io_api_deregister_rid(l2[k]) == get_io_api_register_rid(l2[io_reg_idx])
  ) by {
    assert(k < n);
    assert(l2[k] == l[k]);
    assert(l2[io_reg_idx] == l[io_reg_idx]);
    assert(!(io_api_deregistered_at(l, k) && get_io_api_deregister_rid(l[k]) == get_io_api_register_rid(l[io_reg_idx])));
  };
  assert(io_api_active_at(l2, io_reg_idx, sw_idx));
  assert(io_api_active_at_set_waker(l2, io_rid, sw_idx));

  assert(!is_timer_wake_at(l2, n)) by {
    if is_timer_wake_at(l2, n) {
      let j = choose |j: int| 0 <= j < n &&
        is_succ_register_timer_at(l2, j) &&
        get_register_timer_rid(l2[j]) == io_rid &&
        timer_active_at(l2, j, n);
      assert(l2[j] == l[j]);
      assert(is_succ_register_timer_at(l, j));
      assert(get_register_timer_rid(l[j]) == io_rid);
      if j < io_reg_a {
        assert(no_timer_with_rid_before(l, io_rid, io_reg_a));
        let d = choose |d: int| j < d < io_reg_a && timer_retired_at(l, io_rid, d);
        timer_retired_preserved(l, e, io_rid, d);
      } else {
        assert(no_io_api_with_rid_before(l, io_rid, j));
        let d2 = choose |d2: int| io_reg_a < d2 < j &&
          io_api_deregistered_at(l, d2) && get_io_api_deregister_rid(l[d2]) == io_rid;
        assert(io_api_active_at(l, io_reg_a, n));
        assert(io_reg_a < d2 && d2 < n);
      }
    }
  };

  reactor_safety_inv_preserved_by_wake_task(
    l, e,
    None::<int>, Some(io_reg_idx),
  );

  reactor_ext_inv_preserved_by_non_trigger(l, e);
  alloc_inv_preserved_by_non_registration(l, e, next_rid);
  park_start_preserved_by_non_park(l, e);
}

#[verifier::rlimit(50)]
proof fn timer_heap_entries_valid_after_wake_and_remove(
  old_ts: Set<(InstantView, ResourceIdView, int)>,
  l: Log,
  e: ReactorEvent,
  timer_rid: ResourceIdView,
  entry: (InstantView, ResourceIdView, int),
)
  requires
    timer_heap_entries_valid(old_ts, l),
    reactor_safety_inv(l),
    old_ts.contains(entry),
    entry.1 == timer_rid,
    is_wake_task_at(l.push(e), l.len() as int),
    get_wake_task_source_rid(l.push(e)[l.len() as int]) == timer_rid,
    !is_deregister_timer_at(l.push(e), l.len() as int),
  ensures
    timer_heap_entries_valid(old_ts.remove(entry), l.push(e)),
{
  let l2 = l.push(e);
  let n = l.len() as int;
  let new_ts = old_ts.remove(entry);
  assert forall |d: InstantView, rid: ResourceIdView, log_idx: int|
    #![auto] new_ts.contains((d, rid, log_idx)) implies {
      timer_awaiting_wake(l2, log_idx) &&
      get_register_timer_rid(l2[log_idx]) == rid &&
      get_register_timer_deadline(l2[log_idx]) == d
    }
  by {
    assert(old_ts.contains((d, rid, log_idx)));
    assert(timer_awaiting_wake(l, log_idx));
    assert(get_register_timer_rid(l[log_idx]) == rid);
    assert(get_register_timer_deadline(l[log_idx]) == d);
    if rid == timer_rid {
      timer_heap_no_duplicate_rid(old_ts, l, d, rid, log_idx);
      assert(old_ts.contains((entry.0, timer_rid, entry.2)));
      assert(entry.0 == d && entry.2 == log_idx);
      assert((d, rid, log_idx) == entry);
      assert(!new_ts.contains(entry));
      assert(false);
    }
    assert(log_idx < n);
    assert(l2[log_idx] == l[log_idx]);
    assert forall |k: int| log_idx < k < (n + 1) implies
      !timer_retired_at(l2, rid, k)
    by {
      if k < n {
        assert(l2[k] == l[k]);
        not_timer_retired_preserved(l, e, rid, k);
      } else {
        assert(k == n);
        assert(get_wake_task_source_rid(l2[n]) == timer_rid);
        assert(rid != timer_rid);
        assert(!is_deregister_timer_at(l2, n));
        not_timer_retired(l2, rid, k);
      }
    };
    assert(timer_active_at(l2, log_idx, l2.len() as int));
    assert forall |k: int| log_idx < k < l2.len() implies !(
      is_wake_task_at(l2, k) && get_wake_task_source_rid(l2[k]) == rid
    ) by {
      not_timer_retired_implies(l2, rid, k);
    };
  };
}

#[verifier::rlimit(50)]
proof fn active_timers_in_heap_after_wake_and_remove(
  old_tm: Map<ResourceIdView, (InstantView, ResourceIdView, int)>,
  l: Log,
  e: ReactorEvent,
  timer_rid: ResourceIdView,
)
  requires
    active_timers_in_heap(old_tm, l),
    is_wake_task_at(l.push(e), l.len() as int),
    get_wake_task_source_rid(l.push(e)[l.len() as int]) == timer_rid,
    get_wake_task_source_rid(e) == timer_rid,
  ensures
    active_timers_in_heap(old_tm.remove(timer_rid), l.push(e)),
{
  let l2 = l.push(e);
  let n = l.len() as int;
  let new_tm = old_tm.remove(timer_rid);
  assert forall |log_idx: int| #![auto] timer_awaiting_wake(l2, log_idx) implies {
    let rid = get_register_timer_rid(l2[log_idx]);
    new_tm.contains_key(rid) &&
    new_tm[rid].2 == log_idx
  } by {
    let rid = get_register_timer_rid(l2[log_idx]);
    if rid == timer_rid {
      assert(is_succ_register_timer_at(l2, log_idx));
      assert(!is_succ_register_timer_at(l2, n));
      assert(log_idx != n as int);
      assert(log_idx < n);
      assert(is_wake_task_at(l2, n) && get_wake_task_source_rid(l2[n]) == rid);
      assert(false);
    }
    timer_awaiting_wake_shrink_past_non_matching_wake(l, e, log_idx);
    assert(old_tm.contains_key(rid));
    assert(old_tm[rid].2 == log_idx);
  };
}

#[verifier::rlimit(100)]
pub proof fn timer_wake_remove_step(
  l: Log,
  e: ReactorEvent,
  old_ts: Set<(InstantView, ResourceIdView, int)>,
  old_tm: Map<ResourceIdView, (InstantView, ResourceIdView, int)>,
  old_tw: Map<ResourceIdView, WakerView>,
  rw: Map<ResourceIdView, WakerView>,
  ww: Map<ResourceIdView, WakerView>,
  next_rid: nat,
  timer_rid: ResourceIdView,
)
  requires
    reactor_safety_inv(l),
    reactor_ext_inv(l),
    alloc_inv(l, next_rid),
    timer_impl_inv(old_ts, old_tm, next_rid),
    data_inv(old_ts, old_tm, old_tw, rw, ww, l),
    old_tm.contains_key(timer_rid),
    old_tw.contains_key(timer_rid),
    is_wake_task_at(l.push(e), l.len() as int),
    get_wake_task_source_rid(l.push(e)[l.len() as int]) == timer_rid,
    get_wake_task_waker(l.push(e)[l.len() as int]) == old_tw[timer_rid],
  ensures ({
    let l2 = l.push(e);
    let entry = old_tm[timer_rid];
    let new_ts = old_ts.remove(entry);
    let new_tm = old_tm.remove(timer_rid);
    let new_tw = old_tw.remove(timer_rid);
    reactor_safety_inv(l2) &&
    reactor_ext_inv(l2) &&
    alloc_inv(l2, next_rid) &&
    timer_impl_inv(new_ts, new_tm, next_rid) &&
    data_inv(new_ts, new_tm, new_tw, rw, ww, l2) &&
    new_ts.finite()
  })
{
  let l2 = l.push(e);
  let n = l.len() as int;
  let entry = old_tm[timer_rid];
  let new_ts = old_ts.remove(entry);
  let new_tm = old_tm.remove(timer_rid);
  let new_tw = old_tw.remove(timer_rid);

  assert(old_ts.contains(entry));
  assert(old_ts.contains((entry.0, entry.1, entry.2)));
  let timer_log_idx = entry.2;
  assert(timer_awaiting_wake(l, timer_log_idx));
  assert(is_succ_register_timer_at(l, timer_log_idx));
  assert(timer_active_at(l, timer_log_idx, n));
  assert(get_register_timer_rid(l[timer_log_idx]) == timer_rid);
  assert(old_tw[timer_rid] == get_register_timer_waker(l[timer_log_idx]));

  assert(l2[timer_log_idx] == l[timer_log_idx]);
  assert(is_succ_register_timer_at(l2, timer_log_idx));
  assert forall |k: int| timer_log_idx < k < n implies
    !timer_retired_at(l2, timer_rid, k)
  by {
    assert(l2[k] == l[k]);
    not_timer_retired_transfer(l, l2, timer_rid, k);
  };
  assert(timer_active_at(l2, timer_log_idx, n));
  assert(is_timer_wake_at(l2, n));

  if io_waker_validity::is_io_api_wake_at(l2, n) {
    let j = choose |j: int| 0 <= j < n &&
      io_api_registered_at(l2, j) &&
      get_io_api_register_rid(l2[j]) == timer_rid &&
      io_api_active_at(l2, j, n);
    assert(l2[j] == l[j]);
    assert(io_api_registered_at(l, j));
    assert forall |k: int| j < k < n implies !(
      io_api_deregistered_at(l, k) &&
      get_io_api_deregister_rid(l[k]) == get_io_api_register_rid(l[j])
    ) by {
      assert(l2[k] == l[k]);
      assert(l2[j] == l[j]);
      if io_api_deregistered_at(l2, k) {
        assert(io_api_deregistered_at(l, k));
      }
    };
    assert(io_api_active_at(l, j, n));
    assert(io_currently_active(l, timer_rid));
    timer_heap_no_io_rid(old_ts, l, timer_rid);
    assert(false);
  }

  reactor_safety_inv_preserved_by_wake_task(
    l, e, Some(timer_log_idx as int), None::<int>,
  );
  reactor_ext_inv_preserved_by_non_trigger(l, e);
  alloc_inv_preserved_by_non_registration(l, e, next_rid);

  timer_heap_entries_valid_after_wake_and_remove(old_ts, l, e, timer_rid, entry);
  active_timers_in_heap_after_wake_and_remove(old_tm, l, e, timer_rid);
  timer_wakers_match_preserved_by_append(old_tw, old_tm, l, e);
  timer_wakers_match_after_remove(old_tw, old_tm, timer_rid, l2);
  timer_heap_has_wakers_after_remove(old_tw, old_tm, timer_rid);
  read_wakers_valid_preserved_by_non_set_waker(rw, l, e);
  write_wakers_valid_preserved_by_non_set_waker(ww, l, e);
  read_wakers_complete_preserved_by_non_trigger(rw, l, e);
  write_wakers_complete_preserved_by_non_trigger(ww, l, e);

  assert forall |r: nat| #![auto] new_tm.contains_key(r) implies
    new_ts.contains(new_tm[r]) && new_tm[r].1 == r &&
    1 <= r && r < next_rid
  by {
    assert(old_tm.contains_key(r));
    assert(new_tm[r] == old_tm[r]);
    assert(old_ts.contains(old_tm[r]));
    assert(old_tm[r].1 == r);
    assert(r != timer_rid);
    assert(entry.1 == timer_rid);
    assert(old_tm[r] != entry);
    assert(new_ts.contains(old_tm[r]));
  };

  assert forall |d: InstantView, r: ResourceIdView, i: int|
    #![auto] new_ts.contains((d, r, i)) implies
    new_tm.contains_key(r) && new_tm[r] == (d, r, i)
  by {
    assert(old_ts.contains((d, r, i)));
    assert(old_tm.contains_key(r));
    assert(old_tm[r] == (d, r, i));
    if r == timer_rid {
      assert(old_tm[timer_rid] == entry);
      assert((d, r, i) == entry);
      assert(false);
    }
  };
}

#[verifier::rlimit(80)]
pub proof fn timer_rid_has_no_io_registration(
  l: Log,
  rid: ResourceIdView,
  timer_log_idx: int,
)
  requires
    reactor_safety_inv(l),
    0 <= timer_log_idx < l.len() as int,
    is_succ_register_timer_at(l, timer_log_idx),
    get_register_timer_rid(l[timer_log_idx]) == rid,
    timer_active_at(l, timer_log_idx, l.len() as int),
  ensures
    no_prior_io_api_registration(l, rid, l.len() as int),
    no_io_api_with_rid_before(l, rid, l.len() as int),
{
  let n = l.len() as int;
  assert forall |k: int| 0 <= k < n && io_api_registered_at(l, k) && get_io_api_register_rid(l[k]) == rid implies
    exists |j: int| k < j < n && io_api_deregistered_at(l, j) && get_io_api_deregister_rid(l[j]) == rid
  by {
    if k < timer_log_idx {
      assert(no_io_api_with_rid_before(l, rid, timer_log_idx));
      let j = choose |j: int| k < j < timer_log_idx && io_api_deregistered_at(l, j) && get_io_api_deregister_rid(l[j]) == rid;
      assert(j < n);
    } else {
      assert(no_timer_with_rid_before(l, rid, k));
      assert(is_succ_register_timer_at(l, timer_log_idx));
      assert(get_register_timer_rid(l[timer_log_idx]) == rid);
      let j = choose |j: int| timer_log_idx < j < k && timer_retired_at(l, rid, j);
      assert(timer_active_at(l, timer_log_idx, n));
      assert(!timer_retired_at(l, rid, j));
      assert(false);
    }
  }
}

#[verifier::rlimit(80)]
pub proof fn io_rid_has_no_timer_registration(
  l: Log,
  rid: ResourceIdView,
  io_log_idx: int,
)
  requires
    reactor_safety_inv(l),
    0 <= io_log_idx < l.len() as int,
    io_api_registered_at(l, io_log_idx),
    get_io_api_register_rid(l[io_log_idx]) == rid,
    io_api_active_at(l, io_log_idx, l.len() as int),
  ensures
    no_prior_timer_registration(l, rid, l.len() as int),
    no_timer_with_rid_before(l, rid, l.len() as int),
{
  let n = l.len() as int;
  assert forall |k: int| 0 <= k < n && is_succ_register_timer_at(l, k) && get_register_timer_rid(l[k]) == rid implies
    exists |j: int| k < j < n && timer_retired_at(l, rid, j)
  by {
    if k < io_log_idx {
      assert(no_timer_with_rid_before(l, rid, io_log_idx));
      let j = choose |j: int| k < j < io_log_idx && timer_retired_at(l, rid, j);
      assert(j < n);
    } else {
      assert(no_io_api_with_rid_before(l, rid, k));
      assert(io_api_registered_at(l, io_log_idx));
      assert(get_io_api_register_rid(l[io_log_idx]) == rid);
      let j = choose |j: int| io_log_idx < j < k && io_api_deregistered_at(l, j) && get_io_api_deregister_rid(l[j]) == rid;
      assert(io_api_active_at(l, io_log_idx, n));
      assert(!(io_api_deregistered_at(l, j) && get_io_api_deregister_rid(l[j]) == rid));
      assert(false);
    }
  }
}

}
