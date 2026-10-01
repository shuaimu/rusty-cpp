// The pure-log reactor predicates (event/log predicates, extractors, timer
// retirement, activity tracking, no-prior-registration families, timeout
// points, set-waker searches) now live in the shared crate lion-reactor-spec.
// This module re-exports them and keeps only the impl-linking material:
// alloc_inv, the slab/wheel coupling predicates, wake_covered_by_deregister,
// and the free_rids_wf apparatus.
//
// IO ANCHOR (F-K): this crate's io registration predicates are the API-anchor
// family (io_api_registered_at etc.); the liveness proof uses the
// syscall-anchor family (io_syscall_*). See lion-reactor-spec::bridge.
#[allow(unused_imports)]
pub use lion_reactor_spec::events::*;
#[allow(unused_imports)]
pub use lion_reactor_spec::log::*;

use vstd::prelude::*;
use super::types::*;
use super::log::Log;

verus! {

// ============================================================================
// Allocation invariant (links concrete state to ghost log)
// ============================================================================

pub open spec fn timer_impl_inv(
  timers_view: Set<(InstantView, ResourceIdView, int)>,
  by_rid_view: Map<ResourceIdView, (InstantView, ResourceIdView, int)>,
  next_rid: nat,
) -> bool {
  timers_view.finite() &&
  (forall |r: nat| #![auto] by_rid_view.contains_key(r) ==>
    timers_view.contains(by_rid_view[r]) && by_rid_view[r].1 == r &&
    1 <= r && r < next_rid)
  &&
  (forall |d: InstantView, r: ResourceIdView, i: int|
    #![auto] timers_view.contains((d, r, i)) ==>
    by_rid_view.contains_key(r) && by_rid_view[r] == (d, r, i))
}

pub open spec fn slab_alloc_inv(
  slab_view: Map<nat, ResourceSlotView>,
  next_rid: nat,
) -> bool {
  forall |k: nat| #![auto] slab_view.contains_key(k) ==> 1 <= k && k < next_rid
}

pub open spec fn wheel_slab_consistent(
  wheel_view: Map<nat, int>,
  timer_map: Map<ResourceIdView, (InstantView, ResourceIdView, int)>,
) -> bool {
  &&& forall |rid: nat| #![auto] wheel_view.contains_key(rid) <==> timer_map.contains_key(rid)
  &&& forall |rid: nat| #![auto] wheel_view.contains_key(rid) ==> wheel_view[rid] == timer_map[rid].0
}

pub open spec fn alloc_inv(l: Log, next_rid: nat) -> bool {
  &&& next_rid >= 1
  &&& forall |i: int| #![auto] is_succ_register_timer_at(l, i) ==>
        get_register_timer_rid(l[i]) < next_rid
  &&& forall |i: int| #![auto] io_api_registered_at(l, i) ==>
        get_io_api_register_rid(l[i]) < next_rid
  &&& forall |i: int| #![auto] is_succ_register_timer_at(l, i) ==>
        get_register_timer_rid(l[i]) >= 1
  &&& forall |i: int| #![auto] io_api_registered_at(l, i) ==>
        get_io_api_register_rid(l[i]) >= 1
}

pub proof fn alloc_inv_preserved_by_non_registration(l: Log, e: ReactorEvent, next_rid: nat)
  requires
    alloc_inv(l, next_rid),
    !is_succ_register_timer_at(l.push(e), l.len() as int),
    !io_api_registered_at(l.push(e), l.len() as int),
  ensures
    alloc_inv(l.push(e), next_rid),
{
  let new_l = l.push(e);
  assert forall |i: int| #![auto] is_succ_register_timer_at(new_l, i) implies
    get_register_timer_rid(new_l[i]) < next_rid
  by {
    if i < l.len() {
      assert(new_l[i] == l[i]);
      assert(is_succ_register_timer_at(l, i));
    }
  }
  assert forall |i: int| #![auto] io_api_registered_at(new_l, i) implies
    get_io_api_register_rid(new_l[i]) < next_rid
  by {
    if i < l.len() {
      assert(new_l[i] == l[i]);
      assert(io_api_registered_at(l, i));
    }
  }
  assert forall |i: int| #![auto] is_succ_register_timer_at(new_l, i) implies
    get_register_timer_rid(new_l[i]) >= 1
  by {
    if i < l.len() {
      assert(new_l[i] == l[i]);
      assert(is_succ_register_timer_at(l, i));
    }
  }
  assert forall |i: int| #![auto] io_api_registered_at(new_l, i) implies
    get_io_api_register_rid(new_l[i]) >= 1
  by {
    if i < l.len() {
      assert(new_l[i] == l[i]);
      assert(io_api_registered_at(l, i));
    }
  }
}

pub proof fn alloc_inv_preserved_by_registration(l: Log, e: ReactorEvent, next_rid: nat)
  requires
    alloc_inv(l, next_rid),
    is_succ_register_timer_at(l.push(e), l.len() as int) ==>
      get_register_timer_rid(l.push(e)[l.len() as int]) < next_rid &&
      get_register_timer_rid(l.push(e)[l.len() as int]) >= 1,
    io_api_registered_at(l.push(e), l.len() as int) ==>
      get_io_api_register_rid(l.push(e)[l.len() as int]) < next_rid &&
      get_io_api_register_rid(l.push(e)[l.len() as int]) >= 1,
  ensures
    alloc_inv(l.push(e), next_rid),
{
  let new_l = l.push(e);
  let n = l.len() as int;
  assert forall |i: int| #![auto] is_succ_register_timer_at(new_l, i) implies
    get_register_timer_rid(new_l[i]) < next_rid
  by {
    if i < l.len() {
      assert(new_l[i] == l[i]);
      assert(is_succ_register_timer_at(l, i));
    }
  }
  assert forall |i: int| #![auto] io_api_registered_at(new_l, i) implies
    get_io_api_register_rid(new_l[i]) < next_rid
  by {
    if i < l.len() {
      assert(new_l[i] == l[i]);
      assert(io_api_registered_at(l, i));
    }
  }
  assert forall |i: int| #![auto] is_succ_register_timer_at(new_l, i) implies
    get_register_timer_rid(new_l[i]) >= 1
  by {
    if i < l.len() {
      assert(new_l[i] == l[i]);
      assert(is_succ_register_timer_at(l, i));
    }
  }
  assert forall |i: int| #![auto] io_api_registered_at(new_l, i) implies
    get_io_api_register_rid(new_l[i]) >= 1
  by {
    if i < l.len() {
      assert(new_l[i] == l[i]);
      assert(io_api_registered_at(l, i));
    }
  }
}

pub closed spec fn wake_covered_by_deregister(log: Log, rid: ResourceIdView) -> bool {
  forall |j: int| 0 <= j < log.len() &&
    is_wake_task_at(log, j) && get_wake_task_source_rid(log[j]) == rid ==>
    exists |d: int| j < d < log.len() &&
      is_deregister_timer_at(log, d) && get_deregister_timer_rid(log[d]) == rid
}

pub proof fn wake_covered_by_deregister_vacuous(log: Log, rid: ResourceIdView, next_rid: nat)
  requires
    alloc_inv(log, next_rid),
    rid >= next_rid,
    forall |j: int| 0 <= j < log.len() && is_wake_task_at(log, j) ==>
      get_wake_task_source_rid(log[j]) < next_rid,
  ensures
    wake_covered_by_deregister(log, rid),
{
  reveal(wake_covered_by_deregister);
}

pub closed spec fn free_rids_wf(
  free_rids: Seq<u64>,
  log: Log,
  resources_view: Map<nat, super::types::ResourceSlotView>,
  next_rid: nat,
) -> bool {
  &&& forall |i: int| #![trigger free_rids[i]] 0 <= i < free_rids.len() ==> {
    let rid = free_rids[i] as nat;
    &&& rid >= 1
    &&& rid < next_rid
    &&& !resources_view.contains_key(rid)
    &&& no_prior_timer_registration(log, rid, log.len() as int)
    &&& no_prior_io_api_registration(log, rid, log.len() as int)
    &&& no_timer_with_rid_before(log, rid, log.len() as int)
    &&& no_io_api_with_rid_before(log, rid, log.len() as int)
  }
  &&& forall |i: int, j: int|
    0 <= i < j < free_rids.len() ==> free_rids[i] != free_rids[j]
}

#[verifier::rlimit(50)]
// [RID-REUSE DISABLED — retained for gen-id restoration; uncalled under no-reuse]
pub proof fn free_rids_wf_pop(
  free_rids: Seq<u64>,
  log: Log,
  resources_view: Map<nat, super::types::ResourceSlotView>,
  next_rid: nat,
)
  requires
    free_rids.len() > 0,
    free_rids_wf(free_rids, log, resources_view, next_rid),
  ensures ({
    let rid = free_rids.last() as nat;
    let remaining = free_rids.drop_last();
    &&& rid >= 1
    &&& rid < next_rid
    &&& !resources_view.contains_key(rid)
    &&& no_prior_timer_registration(log, rid, log.len() as int)
    &&& no_prior_io_api_registration(log, rid, log.len() as int)
    &&& no_timer_with_rid_before(log, rid, log.len() as int)
    &&& no_io_api_with_rid_before(log, rid, log.len() as int)
    &&& free_rids_wf(remaining, log, resources_view, next_rid)
    &&& forall |i: int| #![trigger remaining[i]] 0 <= i < remaining.len() ==> remaining[i] as nat != rid
  }),
{
  reveal(free_rids_wf);
  let last = (free_rids.len() - 1) as int;
  let remaining = free_rids.drop_last();
  assert forall |i: int| #![trigger remaining[i]] 0 <= i < remaining.len() implies {
    let rid = remaining[i] as nat;
    &&& rid >= 1
    &&& rid < next_rid
    &&& !resources_view.contains_key(rid)
    &&& no_prior_timer_registration(log, rid, log.len() as int)
    &&& no_prior_io_api_registration(log, rid, log.len() as int)
    &&& no_timer_with_rid_before(log, rid, log.len() as int)
    &&& no_io_api_with_rid_before(log, rid, log.len() as int)
  } by {
    assert(remaining[i] == free_rids[i]);
  }
  assert forall |i: int, j: int| 0 <= i < j < remaining.len() implies remaining[i] != remaining[j]
  by {
    assert(remaining[i] == free_rids[i]);
    assert(remaining[j] == free_rids[j]);
  }
}

#[verifier::rlimit(100)]
pub proof fn free_rids_wf_preserved_by_append(
  free_rids: Seq<u64>,
  l: Log,
  e: ReactorEvent,
  resources_view: Map<nat, super::types::ResourceSlotView>,
  next_rid: nat,
)
  requires
    free_rids_wf(free_rids, l, resources_view, next_rid),
    !is_succ_register_timer_at(l.push(e), l.len() as int),
    !io_api_registered_at(l.push(e), l.len() as int),
  ensures
    free_rids_wf(free_rids, l.push(e), resources_view, next_rid),
{
  reveal(free_rids_wf);
  let l2 = l.push(e);
  let n = l.len() as int;
  assert forall |i: int| #![trigger free_rids[i]] 0 <= i < free_rids.len() implies {
    let rid = free_rids[i] as nat;
    &&& rid >= 1
    &&& rid < next_rid
    &&& !resources_view.contains_key(rid)
    &&& no_prior_timer_registration(l2, rid, l2.len() as int)
    &&& no_prior_io_api_registration(l2, rid, l2.len() as int)
    &&& no_timer_with_rid_before(l2, rid, l2.len() as int)
    &&& no_io_api_with_rid_before(l2, rid, l2.len() as int)
  } by {
    let rid = free_rids[i] as nat;
    assert(no_prior_timer_registration(l, rid, n));
    assert(no_prior_io_api_registration(l, rid, n));
    assert(no_timer_with_rid_before(l, rid, n));
    assert(no_io_api_with_rid_before(l, rid, n));
    // Extend from l to l2 with end = n+1
    // For timer: new event is not register_timer, and for any k < n with witness j < n, j < n+1
    assert forall |k: int| 0 <= k < n + 1 && is_succ_register_timer_at(l2, k) && get_register_timer_rid(l2[k]) == rid implies
      exists |j: int| k < j < n + 1 && #[trigger] timer_retired_at(l2, rid, j)
    by {
      assert(k < n);
      assert(l2[k] == l[k]);
      assert(is_succ_register_timer_at(l, k));
      assert(get_register_timer_rid(l[k]) == rid);
      assert(no_prior_timer_registration(l, rid, n));
      let j = choose |j: int| k < j < n && #[trigger] timer_retired_at(l, rid, j);
      timer_retired_preserved(l, e, rid, j);
    }
    no_prior_io_api_reg_preserved(l, e, rid, n);
    no_io_api_with_rid_preserved(l, e, rid, n);
    assert forall |k: int| 0 <= k < n + 1 && io_api_registered_at(l2, k) && get_io_api_register_rid(l2[k]) == rid implies
      exists |j: int| k < j < n + 1 && io_api_deregistered_at(l2, j) && get_io_api_deregister_rid(l2[j]) == rid
    by {
      if k < n {
        assert(l2[k] == l[k]);
        assert(io_api_registered_at(l, k));
        assert(get_io_api_register_rid(l[k]) == rid);
        let j = choose |j: int| k < j < n && io_api_deregistered_at(l, j) && get_io_api_deregister_rid(l[j]) == rid;
        assert(l2[j] == l[j]);
        assert(io_api_deregistered_at(l2, j));
        assert(get_io_api_deregister_rid(l2[j]) == rid);
      }
    }
  }
}

#[verifier::rlimit(50)]
pub proof fn no_prior_timer_reg_vacuous(l: Log, rid: ResourceIdView, next_rid: nat)
  requires
    alloc_inv(l, next_rid),
    rid >= next_rid,
  ensures
    no_prior_timer_registration(l, rid, l.len() as int),
{
  assert forall |k: int| 0 <= k < l.len() && is_succ_register_timer_at(l, k) && get_register_timer_rid(l[k]) == rid implies
    exists |j: int| k < j < l.len() && #[trigger] timer_retired_at(l, rid, j)
  by {
    assert(get_register_timer_rid(l[k]) < next_rid);
  }
}

#[verifier::rlimit(50)]
pub proof fn no_prior_io_api_reg_vacuous(l: Log, rid: ResourceIdView, next_rid: nat)
  requires
    alloc_inv(l, next_rid),
    rid >= next_rid,
  ensures
    no_prior_io_api_registration(l, rid, l.len() as int),
{
  assert forall |k: int| 0 <= k < l.len() && io_api_registered_at(l, k) && get_io_api_register_rid(l[k]) == rid implies
    exists |j: int| k < j < l.len() && io_api_deregistered_at(l, j) && get_io_api_deregister_rid(l[j]) == rid
  by {
    assert(get_io_api_register_rid(l[k]) < next_rid);
  }
}

#[verifier::rlimit(50)]
pub proof fn free_rids_wf_empty(
  log: Log,
  resources_view: Map<nat, super::types::ResourceSlotView>,
  next_rid: nat,
)
  ensures
    free_rids_wf(Seq::<u64>::empty(), log, resources_view, next_rid),
{
  reveal(free_rids_wf);
}

#[verifier::rlimit(50)]
pub proof fn free_rids_wf_preserved_by_next_rid_increase(
  free_rids: Seq<u64>,
  log: Log,
  resources_view: Map<nat, super::types::ResourceSlotView>,
  next_rid: nat,
  new_next_rid: nat,
)
  requires
    free_rids_wf(free_rids, log, resources_view, next_rid),
    new_next_rid >= next_rid,
  ensures
    free_rids_wf(free_rids, log, resources_view, new_next_rid),
    forall |i: int| #![trigger free_rids[i]] 0 <= i < free_rids.len() ==> (free_rids[i] as nat) < next_rid,
{
  reveal(free_rids_wf);
}

#[verifier::rlimit(100)]
pub proof fn free_rids_wf_preserved_by_register_timer(
  free_rids: Seq<u64>,
  l: Log,
  e: ReactorEvent,
  resources_view: Map<nat, super::types::ResourceSlotView>,
  next_rid: nat,
  new_rid: ResourceIdView,
)
  requires
    free_rids_wf(free_rids, l, resources_view, next_rid),
    is_succ_register_timer_at(l.push(e), l.len() as int),
    get_register_timer_rid(l.push(e)[l.len() as int]) == new_rid,
    forall |i: int| #![trigger free_rids[i]] 0 <= i < free_rids.len() ==> free_rids[i] as nat != new_rid,
  ensures
    free_rids_wf(free_rids, l.push(e), resources_view, next_rid),
{
  reveal(free_rids_wf);
  let l2 = l.push(e);
  let n = l.len() as int;
  assert forall |i: int| #![trigger free_rids[i]] 0 <= i < free_rids.len() implies {
    let rid = free_rids[i] as nat;
    &&& rid >= 1
    &&& rid < next_rid
    &&& !resources_view.contains_key(rid)
    &&& no_prior_timer_registration(l2, rid, l2.len() as int)
    &&& no_prior_io_api_registration(l2, rid, l2.len() as int)
    &&& no_timer_with_rid_before(l2, rid, l2.len() as int)
    &&& no_io_api_with_rid_before(l2, rid, l2.len() as int)
  } by {
    let rid = free_rids[i] as nat;
    assert(rid != new_rid);
    assert forall |k: int| 0 <= k < n + 1 && is_succ_register_timer_at(l2, k) && get_register_timer_rid(l2[k]) == rid implies
      exists |j: int| k < j < n + 1 && #[trigger] timer_retired_at(l2, rid, j)
    by {
      if k < n {
        assert(l2[k] == l[k]);
        assert(is_succ_register_timer_at(l, k));
        assert(get_register_timer_rid(l[k]) == rid);
        let j = choose |j: int| k < j < n && #[trigger] timer_retired_at(l, rid, j);
        timer_retired_preserved(l, e, rid, j);
      } else {
        assert(get_register_timer_rid(l2[k]) == new_rid);
        assert(rid != new_rid);
      }
    }
    no_prior_io_api_reg_preserved(l, e, rid, n);
    assert forall |k: int| 0 <= k < n + 1 && io_api_registered_at(l2, k) && get_io_api_register_rid(l2[k]) == rid implies
      exists |j: int| k < j < n + 1 && io_api_deregistered_at(l2, j) && get_io_api_deregister_rid(l2[j]) == rid
    by {
      if k < n {
        assert(l2[k] == l[k]);
        assert(io_api_registered_at(l, k));
        let j = choose |j: int| k < j < n && io_api_deregistered_at(l, j) && get_io_api_deregister_rid(l[j]) == rid;
        assert(l2[j] == l[j]);
      }
    }
    no_timer_with_rid_preserved(l, e, rid, n);
    no_io_api_with_rid_preserved(l, e, rid, n);
  }
}

#[verifier::rlimit(100)]
pub proof fn free_rids_wf_preserved_by_register_io(
  free_rids: Seq<u64>,
  l: Log,
  e: ReactorEvent,
  resources_view: Map<nat, super::types::ResourceSlotView>,
  next_rid: nat,
  new_rid: ResourceIdView,
)
  requires
    free_rids_wf(free_rids, l, resources_view, next_rid),
    io_api_registered_at(l.push(e), l.len() as int),
    get_io_api_register_rid(l.push(e)[l.len() as int]) == new_rid,
    forall |i: int| #![trigger free_rids[i]] 0 <= i < free_rids.len() ==> free_rids[i] as nat != new_rid,
  ensures
    free_rids_wf(free_rids, l.push(e), resources_view, next_rid),
{
  reveal(free_rids_wf);
  let l2 = l.push(e);
  let n = l.len() as int;
  assert forall |i: int| #![trigger free_rids[i]] 0 <= i < free_rids.len() implies {
    let rid = free_rids[i] as nat;
    &&& rid >= 1
    &&& rid < next_rid
    &&& !resources_view.contains_key(rid)
    &&& no_prior_timer_registration(l2, rid, l2.len() as int)
    &&& no_prior_io_api_registration(l2, rid, l2.len() as int)
    &&& no_timer_with_rid_before(l2, rid, l2.len() as int)
    &&& no_io_api_with_rid_before(l2, rid, l2.len() as int)
  } by {
    let rid = free_rids[i] as nat;
    assert(rid != new_rid);
    no_prior_timer_reg_preserved(l, e, rid, n);
    no_prior_io_api_reg_preserved(l, e, rid, n);
    assert forall |k: int| 0 <= k < n + 1 && io_api_registered_at(l2, k) && get_io_api_register_rid(l2[k]) == rid implies
      exists |j: int| k < j < n + 1 && io_api_deregistered_at(l2, j) && get_io_api_deregister_rid(l2[j]) == rid
    by {
      if k < n {
        assert(l2[k] == l[k]);
        assert(io_api_registered_at(l, k));
        let j = choose |j: int| k < j < n && io_api_deregistered_at(l, j) && get_io_api_deregister_rid(l[j]) == rid;
        assert(l2[j] == l[j]);
      } else {
        assert(get_io_api_register_rid(l2[k]) == new_rid);
        assert(rid != new_rid);
      }
    }
    no_timer_with_rid_preserved(l, e, rid, n);
    no_io_api_with_rid_preserved(l, e, rid, n);
  }
}

#[verifier::rlimit(100)]
// [RID-REUSE DISABLED — retained for gen-id restoration; uncalled under no-reuse]
pub proof fn free_rids_wf_push_deregistered_timer(
  free_rids: Seq<u64>,
  log: Log,
  resources_view: Map<nat, super::types::ResourceSlotView>,
  next_rid: nat,
  rid_val: u64,
)
  requires
    free_rids_wf(free_rids, log, resources_view, next_rid),
    rid_val as nat >= 1,
    (rid_val as nat) < next_rid,
    !resources_view.contains_key(rid_val as nat),
    forall |i: int| #![trigger free_rids[i]] 0 <= i < free_rids.len() ==> free_rids[i] != rid_val,
    no_prior_timer_registration(log, rid_val as nat, log.len() as int),
    no_prior_io_api_registration(log, rid_val as nat, log.len() as int),
    no_timer_with_rid_before(log, rid_val as nat, log.len() as int),
    no_io_api_with_rid_before(log, rid_val as nat, log.len() as int),
  ensures
    free_rids_wf(free_rids.push(rid_val), log, resources_view, next_rid),
{
  reveal(free_rids_wf);
  let new_free_rids = free_rids.push(rid_val);
  assert forall |i: int| #![trigger new_free_rids[i]] 0 <= i < new_free_rids.len() implies {
    let rid = new_free_rids[i] as nat;
    &&& rid >= 1
    &&& rid < next_rid
    &&& !resources_view.contains_key(rid)
    &&& no_prior_timer_registration(log, rid, log.len() as int)
    &&& no_prior_io_api_registration(log, rid, log.len() as int)
    &&& no_timer_with_rid_before(log, rid, log.len() as int)
    &&& no_io_api_with_rid_before(log, rid, log.len() as int)
  } by {
    if i < free_rids.len() {
      assert(new_free_rids[i] == free_rids[i]);
    } else {
      assert(new_free_rids[i] == rid_val);
    }
  }
  assert forall |i: int, j: int| 0 <= i < j < new_free_rids.len() implies new_free_rids[i] != new_free_rids[j]
  by {
    if j < free_rids.len() as int {
      assert(new_free_rids[i] == free_rids[i]);
      assert(new_free_rids[j] == free_rids[j]);
    } else {
      assert(new_free_rids[j] == rid_val);
      assert(new_free_rids[i] == free_rids[i]);
      assert(free_rids[i] != rid_val);
    }
  }
}

#[verifier::rlimit(50)]
pub proof fn free_rids_wf_preserved_by_resource_change(
  free_rids: Seq<u64>,
  log: Log,
  old_resources: Map<nat, super::types::ResourceSlotView>,
  new_resources: Map<nat, super::types::ResourceSlotView>,
  next_rid: nat,
)
  requires
    free_rids_wf(free_rids, log, old_resources, next_rid),
    forall |i: int| #![trigger free_rids[i]] 0 <= i < free_rids.len() ==>
      !new_resources.contains_key(free_rids[i] as nat),
  ensures
    free_rids_wf(free_rids, log, new_resources, next_rid),
{
  reveal(free_rids_wf);
}

#[verifier::rlimit(50)]
pub proof fn free_rids_wf_preserved_by_resource_insert(
  free_rids: Seq<u64>,
  log: Log,
  resources: Map<nat, super::types::ResourceSlotView>,
  next_rid: nat,
  key: nat,
  val: super::types::ResourceSlotView,
)
  requires
    free_rids_wf(free_rids, log, resources, next_rid),
    forall |i: int| #![trigger free_rids[i]] 0 <= i < free_rids.len() ==> free_rids[i] as nat != key,
  ensures
    free_rids_wf(free_rids, log, resources.insert(key, val), next_rid),
{
  reveal(free_rids_wf);
}

#[verifier::rlimit(50)]
pub proof fn free_rids_wf_preserved_by_resource_remove(
  free_rids: Seq<u64>,
  log: Log,
  resources: Map<nat, super::types::ResourceSlotView>,
  next_rid: nat,
  key: nat,
)
  requires
    free_rids_wf(free_rids, log, resources, next_rid),
  ensures
    free_rids_wf(free_rids, log, resources.remove(key), next_rid),
{
  reveal(free_rids_wf);
}

#[verifier::rlimit(50)]
pub proof fn free_rids_wf_not_contains_key(
  free_rids: Seq<u64>,
  log: Log,
  resources: Map<nat, super::types::ResourceSlotView>,
  next_rid: nat,
  i: int,
)
  requires
    free_rids_wf(free_rids, log, resources, next_rid),
    0 <= i < free_rids.len(),
  ensures
    !resources.contains_key(free_rids[i] as nat),
{
  reveal(free_rids_wf);
}

#[verifier::rlimit(50)]
pub proof fn free_rids_wf_preserved_by_same_domain(
  free_rids: Seq<u64>,
  log: Log,
  old_resources: Map<nat, super::types::ResourceSlotView>,
  new_resources: Map<nat, super::types::ResourceSlotView>,
  next_rid: nat,
)
  requires
    free_rids_wf(free_rids, log, old_resources, next_rid),
    old_resources.dom() =~= new_resources.dom(),
  ensures
    free_rids_wf(free_rids, log, new_resources, next_rid),
{
  reveal(free_rids_wf);
}

#[verifier::rlimit(50)]
// [RID-REUSE DISABLED — retained for gen-id restoration; uncalled under no-reuse]
pub proof fn free_rids_wf_disjoint_from_key(
  free_rids: Seq<u64>,
  log: Log,
  resources: Map<nat, super::types::ResourceSlotView>,
  next_rid: nat,
  key: nat,
)
  requires
    free_rids_wf(free_rids, log, resources, next_rid),
    resources.contains_key(key),
  ensures
    forall |i: int| #![trigger free_rids[i]] 0 <= i < free_rids.len() ==> free_rids[i] as nat != key,
{
  reveal(free_rids_wf);
}

#[verifier::rlimit(100)]
// [RID-REUSE DISABLED — retained for gen-id restoration; uncalled under no-reuse]
pub proof fn free_rids_wf_after_deregister_timer_push(
  free_rids: Seq<u64>,
  l: Log,
  e: ReactorEvent,
  old_resources: Map<nat, super::types::ResourceSlotView>,
  new_resources: Map<nat, super::types::ResourceSlotView>,
  next_rid: nat,
  rid_val: u64,
)
  requires
    free_rids_wf(free_rids, l, old_resources, next_rid),
    is_succ_deregister_timer_at(l.push(e), l.len() as int),
    get_deregister_timer_rid(l.push(e)[l.len() as int]) == rid_val as nat,
    !is_succ_register_timer_at(l.push(e), l.len() as int),
    !io_api_registered_at(l.push(e), l.len() as int),
    new_resources == old_resources.remove(rid_val as nat),
    rid_val as nat >= 1,
    (rid_val as nat) < next_rid,
    old_resources.contains_key(rid_val as nat),
    no_prior_io_api_registration(l, rid_val as nat, l.len() as int),
    no_io_api_with_rid_before(l, rid_val as nat, l.len() as int),
  ensures
    free_rids_wf(free_rids.push(rid_val), l.push(e), new_resources, next_rid),
{
  let l2 = l.push(e);
  let rid = rid_val as nat;
  free_rids_wf_disjoint_from_key(free_rids, l, old_resources, next_rid, rid);
  free_rids_wf_preserved_by_append(free_rids, l, e, old_resources, next_rid);
  free_rids_wf_preserved_by_resource_remove(free_rids, l2, old_resources, next_rid, rid);
  no_prior_timer_reg_after_deregister_timer(l, e, rid);
  no_prior_io_api_reg_preserved(l, e, rid, l.len() as int);
  no_io_api_with_rid_preserved(l, e, rid, l.len() as int);
  free_rids_wf_push_deregistered_timer(free_rids, l2, new_resources, next_rid, rid_val);
}

#[verifier::rlimit(100)]
// [RID-REUSE DISABLED — retained for gen-id restoration; uncalled under no-reuse]
pub proof fn free_rids_wf_after_wake_timer_push(
  free_rids: Seq<u64>,
  l: Log,
  e: ReactorEvent,
  old_resources: Map<nat, super::types::ResourceSlotView>,
  new_resources: Map<nat, super::types::ResourceSlotView>,
  next_rid: nat,
  rid_val: u64,
)
  requires
    free_rids_wf(free_rids, l, old_resources, next_rid),
    is_wake_task_at(l.push(e), l.len() as int),
    get_wake_task_source_rid(l.push(e)[l.len() as int]) == rid_val as nat,
    !is_succ_register_timer_at(l.push(e), l.len() as int),
    !io_api_registered_at(l.push(e), l.len() as int),
    new_resources == old_resources.remove(rid_val as nat),
    rid_val as nat >= 1,
    (rid_val as nat) < next_rid,
    old_resources.contains_key(rid_val as nat),
    no_prior_io_api_registration(l, rid_val as nat, l.len() as int),
    no_io_api_with_rid_before(l, rid_val as nat, l.len() as int),
  ensures
    free_rids_wf(free_rids.push(rid_val), l.push(e), new_resources, next_rid),
{
  let l2 = l.push(e);
  let rid = rid_val as nat;
  free_rids_wf_disjoint_from_key(free_rids, l, old_resources, next_rid, rid);
  free_rids_wf_preserved_by_append(free_rids, l, e, old_resources, next_rid);
  free_rids_wf_preserved_by_resource_remove(free_rids, l2, old_resources, next_rid, rid);
  no_prior_timer_reg_after_wake(l, e, rid);
  no_prior_io_api_reg_preserved(l, e, rid, l.len() as int);
  no_io_api_with_rid_preserved(l, e, rid, l.len() as int);
  free_rids_wf_push_deregistered_timer(free_rids, l2, new_resources, next_rid, rid_val);
}

#[verifier::rlimit(100)]
// [RID-REUSE DISABLED — retained for gen-id restoration; uncalled under no-reuse]
pub proof fn free_rids_wf_after_deregister_io_push(
  free_rids: Seq<u64>,
  l: Log,
  e: ReactorEvent,
  old_resources: Map<nat, super::types::ResourceSlotView>,
  new_resources: Map<nat, super::types::ResourceSlotView>,
  next_rid: nat,
  rid_val: u64,
)
  requires
    free_rids_wf(free_rids, l, old_resources, next_rid),
    io_api_deregistered_at(l.push(e), l.len() as int),
    get_io_api_deregister_rid(l.push(e)[l.len() as int]) == rid_val as nat,
    !is_succ_register_timer_at(l.push(e), l.len() as int),
    !io_api_registered_at(l.push(e), l.len() as int),
    new_resources == old_resources.remove(rid_val as nat),
    rid_val as nat >= 1,
    (rid_val as nat) < next_rid,
    old_resources.contains_key(rid_val as nat),
    no_prior_timer_registration(l, rid_val as nat, l.len() as int),
    no_timer_with_rid_before(l, rid_val as nat, l.len() as int),
  ensures
    free_rids_wf(free_rids.push(rid_val), l.push(e), new_resources, next_rid),
{
  let l2 = l.push(e);
  let rid = rid_val as nat;
  free_rids_wf_disjoint_from_key(free_rids, l, old_resources, next_rid, rid);
  free_rids_wf_preserved_by_append(free_rids, l, e, old_resources, next_rid);
  free_rids_wf_preserved_by_resource_remove(free_rids, l2, old_resources, next_rid, rid);
  no_prior_io_api_reg_after_deregister_io(l, e, rid);
  no_prior_timer_reg_preserved(l, e, rid, l.len() as int);
  no_timer_with_rid_preserved(l, e, rid, l.len() as int);
  free_rids_wf_push_deregistered_timer(free_rids, l2, new_resources, next_rid, rid_val);
}

}
