use vstd::prelude::*;
use crate::framework::action_safety::*;
use crate::view_types::*;
use crate::generic::events::*;
use crate::generic::log::*;
use crate::generic::invariants::*;

verus! {

// Indexing into a prefix is stable under appending a suffix.
pub proof fn lemma_index_prefix<M, R>(pre: Log<M, R>, suf: Log<M, R>, k: int)
  requires 0 <= k < pre.len(),
  ensures (pre + suf)[k] == pre[k],
{
}

// Indexing into the appended suffix.
pub proof fn lemma_index_suffix<M, R>(pre: Log<M, R>, suf: Log<M, R>, m: int)
  requires 0 <= m < suf.len(),
  ensures (pre + suf)[pre.len() + m] == suf[m],
{
}

// token_active_before only looks at indices < i, so it is monotone under append
// (the witnessing register and the absence of a later deregister both transfer).
pub proof fn lemma_token_active_before_monotone<M, R>(
  pre: Log<M, R>, suf: Log<M, R>, rt: ResourceIdView, i: int,
)
  requires 0 <= i <= pre.len(), token_active_before(pre, rt, i),
  ensures token_active_before(pre + suf, rt, i),
{
  let post = pre + suf;
  assert forall |k: int| 0 <= k < pre.len() implies post[k] == pre[k] by {
    lemma_index_prefix(pre, suf, k);
  }
  let j = choose |j: int|
    0 <= j < i && is_register_succ_of_token_at(pre, j, rt) &&
    !(exists |k: int| j < k < i && is_dereg_of_token_at(pre, k, rt));
  assert(is_register_succ_of_token_at(post, j, rt));
  assert forall |k: int| j < k < i implies !is_dereg_of_token_at(post, k, rt) by {
    assert(post[k] == pre[k]);
    assert(!is_dereg_of_token_at(pre, k, rt));
  }
  assert(token_active_before(post, rt, i));
}

// io_active_at only looks at indices < i ⇒ monotone under append.
pub proof fn lemma_io_active_at_monotone<M, R>(
  pre: Log<M, R>, suf: Log<M, R>, rid: ResourceIdView, i: int,
)
  requires 0 <= i <= pre.len(), io_active_at(pre, rid, i),
  ensures io_active_at(pre + suf, rid, i),
{
  let post = pre + suf;
  assert forall |k: int| 0 <= k < pre.len() implies post[k] == pre[k] by {
    lemma_index_prefix(pre, suf, k);
  }
  let j = choose |j: int|
    0 <= j < i && #[trigger] is_register_io_succ_at(pre, j) && get_register_io_token(pre[j]) == rid;
  assert(post[j] == pre[j]);
  assert(is_register_io_succ_at(post, j) && get_register_io_token(post[j]) == rid);
  assert forall |k: int| 0 <= k < i implies
    !(#[trigger] is_deregister_io_at(post, k) && get_resource_token(post[k]) == rid) by {
    assert(post[k] == pre[k]);
    assert(!(is_deregister_io_at(pre, k) && get_resource_token(pre[k]) == rid));
  }
  assert(io_active_at(post, rid, i));
}

// Extend an active token's window forward across a dereg-free suffix region.
// (Used by reactor-io utilities, where the Register sits in a *previous* segment
// and "still active" must be carried forward into the current poll's indices.)
pub proof fn lemma_token_active_before_extend<M, R>(
  pre: Log<M, R>, suf: Log<M, R>, rt: ResourceIdView, k: int,
)
  requires
    token_active_before(pre, rt, pre.len() as int),
    pre.len() <= k <= (pre + suf).len(),
    forall |j: int| pre.len() <= j < k ==> !#[trigger] is_dereg_of_token_at(pre + suf, j, rt),
  ensures token_active_before(pre + suf, rt, k),
{
  let post = pre + suf;
  assert forall |m: int| 0 <= m < pre.len() implies post[m] == pre[m] by {
    lemma_index_prefix(pre, suf, m);
  }
  let j = choose |j: int|
    0 <= j < pre.len() as int && is_register_succ_of_token_at(pre, j, rt) &&
    !(exists |kk: int| j < kk < pre.len() as int && is_dereg_of_token_at(pre, kk, rt));
  assert(is_register_succ_of_token_at(post, j, rt));
  assert forall |kk: int| j < kk < k implies !is_dereg_of_token_at(post, kk, rt) by {
    if kk < pre.len() {
      assert(post[kk] == pre[kk]);
      assert(!is_dereg_of_token_at(pre, kk, rt));
    }
  }
  assert(token_active_before(post, rt, k));
}

// Same forward extension for io_active_at (no DeregisterIo of `rid` in the
// dereg-free suffix region keeps the io resource active at the new index).
pub proof fn lemma_io_active_at_extend<M, R>(
  pre: Log<M, R>, suf: Log<M, R>, rid: ResourceIdView, k: int,
)
  requires
    io_active_at(pre, rid, pre.len() as int),
    pre.len() <= k <= (pre + suf).len(),
    forall |j: int| pre.len() <= j < k ==>
      !(#[trigger] is_deregister_io_at(pre + suf, j) && get_resource_token((pre + suf)[j]) == rid),
  ensures io_active_at(pre + suf, rid, k),
{
  let post = pre + suf;
  assert forall |m: int| 0 <= m < pre.len() implies post[m] == pre[m] by {
    lemma_index_prefix(pre, suf, m);
  }
  let j = choose |j: int|
    0 <= j < pre.len() as int && #[trigger] is_register_io_succ_at(pre, j) && get_register_io_token(pre[j]) == rid;
  assert(is_register_io_succ_at(post, j) && get_register_io_token(post[j]) == rid);
  assert forall |kk: int| 0 <= kk < k implies
    !(#[trigger] is_deregister_io_at(post, kk) && get_resource_token(post[kk]) == rid) by {
    if kk < pre.len() {
      assert(post[kk] == pre[kk]);
      assert(!(is_deregister_io_at(pre, kk) && get_resource_token(pre[kk]) == rid));
    }
  }
  assert(io_active_at(post, rid, k));
}

// complete_tick_cycle only references indices in [b, e] ⇒ monotone (e < pre.len).
pub proof fn lemma_complete_tick_cycle_monotone<M, R>(
  pre: Log<M, R>, suf: Log<M, R>, b: int, e: int,
)
  requires 0 <= b, e < pre.len(), complete_tick_cycle(pre, b, e),
  ensures complete_tick_cycle(pre + suf, b, e),
{
  let post = pre + suf;
  assert forall |k: int| 0 <= k < pre.len() implies post[k] == pre[k] by {
    lemma_index_prefix(pre, suf, k);
  }
  assert forall |k: int| b < k < e implies
    (!is_tick_begin_at(post, k) && !is_tick_end_at(post, k)) by {
    assert(post[k] == pre[k]);
    assert(!is_tick_begin_at(pre, k) && !is_tick_end_at(pre, k));
  }
  assert(complete_tick_cycle(post, b, e));
}

// ── The four wakeup-source disjuncts are each monotone under append ──

pub proof fn lemma_passwaker_armed_monotone<M, R>(
  pre: Log<M, R>, suf: Log<M, R>, w: WakerView, b: int, i: int,
)
  requires 0 <= i <= pre.len(), passwaker_armed_in_cycle(pre, w, b, i),
  ensures passwaker_armed_in_cycle(pre + suf, w, b, i),
{
  let post = pre + suf;
  assert forall |k: int| 0 <= k < pre.len() implies post[k] == pre[k] by {
    lemma_index_prefix(pre, suf, k);
  }
  let j = choose |j: int| b < j < i && #[trigger] is_pass_waker_at(pre, j) && get_pass_waker_waker(pre[j]) == w;
  assert(post[j] == pre[j]);
  assert(is_pass_waker_at(post, j) && get_pass_waker_waker(post[j]) == w);
  assert(passwaker_armed_in_cycle(post, w, b, i));
}

pub proof fn lemma_defer_monotone<M, R>(
  pre: Log<M, R>, suf: Log<M, R>, b: int, i: int,
)
  requires 0 <= i <= pre.len(), defer_in_cycle(pre, b, i),
  ensures defer_in_cycle(pre + suf, b, i),
{
  let post = pre + suf;
  assert forall |k: int| 0 <= k < pre.len() implies post[k] == pre[k] by {
    lemma_index_prefix(pre, suf, k);
  }
  let j = choose |j: int| b < j < i && is_defer_at(pre, j);
  assert(post[j] == pre[j]);
  assert(is_defer_at(post, j));
  assert(defer_in_cycle(post, b, i));
}

pub proof fn lemma_timer_armed_monotone<M, R>(
  pre: Log<M, R>, suf: Log<M, R>, w: WakerView, b: int, i: int,
)
  requires 0 <= i <= pre.len(), timer_armed_for_in_cycle(pre, w, b, i),
  ensures timer_armed_for_in_cycle(pre + suf, w, b, i),
{
  let post = pre + suf;
  assert forall |k: int| 0 <= k < pre.len() implies post[k] == pre[k] by {
    lemma_index_prefix(pre, suf, k);
  }
  let j = choose |j: int|
    b < j < i && #[trigger] is_register_timer_succ_at(pre, j) && get_register_timer_waker(pre[j]) == w &&
    !timer_deregistered_after_in_cycle(pre, get_register_timer_token(pre[j]), j, i);
  assert(post[j] == pre[j]);
  let rt = get_register_timer_token(pre[j]);
  assert forall |k: int| j < k < i implies
    !(#[trigger] is_deregister_timer_at(post, k) && get_resource_token(post[k]) == rt) by {
    assert(post[k] == pre[k]);
    assert(!(is_deregister_timer_at(pre, k) && get_resource_token(pre[k]) == rt));
  }
  assert(!timer_deregistered_after_in_cycle(post, rt, j, i));
  assert(is_register_timer_succ_at(post, j) && get_register_timer_waker(post[j]) == w);
  assert(timer_armed_for_in_cycle(post, w, b, i));
}

pub proof fn lemma_io_armed_monotone<M, R>(
  pre: Log<M, R>, suf: Log<M, R>, w: WakerView, b: int, i: int,
)
  requires 0 <= i <= pre.len(), io_armed_for_in_cycle(pre, w, b, i),
  ensures io_armed_for_in_cycle(pre + suf, w, b, i),
{
  let post = pre + suf;
  assert forall |k: int| 0 <= k < pre.len() implies post[k] == pre[k] by {
    lemma_index_prefix(pre, suf, k);
  }
  let rid = choose |rid: ResourceIdView|
    io_active_at(pre, rid, i) &&
    (exists |j: int| b < j < i && #[trigger] is_set_io_waker_at(pre, j) &&
       get_resource_token(pre[j]) == rid && get_set_io_waker_waker(pre[j]) == w);
  lemma_io_active_at_monotone(pre, suf, rid, i);
  let j = choose |j: int| b < j < i && #[trigger] is_set_io_waker_at(pre, j) &&
    get_resource_token(pre[j]) == rid && get_set_io_waker_waker(pre[j]) == w;
  assert(post[j] == pre[j]);
  assert(is_set_io_waker_at(post, j) && get_resource_token(post[j]) == rid &&
    get_set_io_waker_waker(post[j]) == w);
  assert(io_armed_for_in_cycle(post, w, b, i));
}

pub proof fn lemma_active_source_monotone<M, R>(
  pre: Log<M, R>, suf: Log<M, R>, w: WakerView, b: int, i: int,
)
  requires 0 <= i <= pre.len(), active_wakeup_source_for(pre, w, b, i),
  ensures active_wakeup_source_for(pre + suf, w, b, i),
{
  if passwaker_armed_in_cycle(pre, w, b, i) {
    lemma_passwaker_armed_monotone(pre, suf, w, b, i);
  } else if defer_in_cycle(pre, b, i) {
    lemma_defer_monotone(pre, suf, b, i);
  } else if timer_armed_for_in_cycle(pre, w, b, i) {
    lemma_timer_armed_monotone(pre, suf, w, b, i);
  } else {
    assert(io_armed_for_in_cycle(pre, w, b, i));
    lemma_io_armed_monotone(pre, suf, w, b, i);
  }
}

// wakeup_validity is monotone under append (transfer the tick cycle and source).
pub proof fn lemma_wakeup_validity_monotone<M, R>(
  pre: Log<M, R>, suf: Log<M, R>, i: int,
)
  requires 0 <= i < pre.len(), wakeup_validity(pre, i),
  ensures wakeup_validity(pre + suf, i),
{
  let post = pre + suf;
  assert forall |k: int| 0 <= k < pre.len() implies post[k] == pre[k] by {
    lemma_index_prefix(pre, suf, k);
  }
  assert(post[i] == pre[i]);
  let w = get_tick_waker(pre[i]);
  let b = choose |b: int| #[trigger] complete_tick_cycle(pre, b, i) && active_wakeup_source_for(pre, w, b, i);
  assert(complete_tick_cycle(pre, b, i));
  lemma_complete_tick_cycle_monotone(pre, suf, b, i);
  lemma_active_source_monotone(pre, suf, w, b, i);
  assert(get_tick_waker(post[i]) == w);
  assert(complete_tick_cycle(post, b, i) &&
    active_wakeup_source_for(post, get_tick_waker(post[i]), b, i));
  assert(wakeup_validity(post, i));
}

// General action-safety extension lemma. If an action-safety property holds on
// `pre`, and under the append: (h1) acceptance agrees on old indices, (h2)
// validity is monotone on old indices, and (h3) every index outside the old
// range that accepts is valid, then the property holds on `pre + suf`.
//
// (h3) covers both the genuinely new indices and out-of-range indices; for the
// atomic acceptance predicates the latter are vacuous.
pub proof fn lemma_action_safety_extend<M, R>(
  p: ActionSafety<Log<M, R>>, pre: Log<M, R>, suf: Log<M, R>,
)
  requires
    action_safety_satisfied(p, pre),
    forall |i: int| 0 <= i < pre.len() ==>
      ((#[trigger] (p.acceptance)(pre + suf, i)) == (p.acceptance)(pre, i)),
    forall |i: int| 0 <= i < pre.len() ==>
      ((p.validity)(pre, i) ==> #[trigger] (p.validity)(pre + suf, i)),
    forall |i: int| !(0 <= i < pre.len()) ==>
      ((#[trigger] (p.acceptance)(pre + suf, i)) ==> (p.validity)(pre + suf, i)),
  ensures action_safety_satisfied(p, pre + suf),
{
  assert forall |i: int| #[trigger] (p.acceptance)(pre + suf, i) implies (p.validity)(pre + suf, i) by {
    if 0 <= i < pre.len() {
      assert((p.acceptance)(pre, i));
      assert((p.validity)(pre, i));
      assert((p.validity)(pre + suf, i));
    } else {
      assert((p.validity)(pre + suf, i));
    }
  }
}

}
