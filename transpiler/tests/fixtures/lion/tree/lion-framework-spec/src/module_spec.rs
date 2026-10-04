use vstd::prelude::*;

verus! {

// Paper reference: the liveness-framework figure (§4, lst:liveness-framework),
// which presents this struct under pedagogical field names:
//
//   paper `progress_inv` = `well_formed` here (the inductive progress invariant)
//   paper `step`         = `progress` here (one driving cycle appended to the log)
//   paper `step_n`       = `progress_n` / `env_progress_n` below
#[verifier::reject_recursive_types(L)]
pub struct ModuleSpec<L> {
  pub well_formed: spec_fn(L) -> bool,
  pub progress: spec_fn(L, L) -> bool,
}

pub open spec fn is_valid_trace<L>(progress: spec_fn(L, L) -> bool, trace: Seq<L>) -> bool {
  trace.len() >= 1 &&
  forall |i: int| 0 <= i < trace.len() - 1 ==>
    progress(#[trigger] trace[i], trace[i + 1])
}

pub open spec fn progress_n<L>(progress: spec_fn(L, L) -> bool, l: L, l_prime: L, n: nat) -> bool {
  exists |trace: Seq<L>|
    #![trigger trace.len()]
    trace.len() == n + 1 &&
    trace.first() == l &&
    trace.last() == l_prime &&
    is_valid_trace(progress, trace)
}

// Every state of an n-step trace satisfies `env(·, t)`. The trace-level filter
// primitive for □env ⇒ ◇goal: unlike progress_n, this makes env available at
// EVERY intermediate state (not just the endpoint), which the response proof's
// intermediate-state uses require.
// NOTE (P7 hygiene): `progress` is unused in the body — env_holds_along only
// asserts env at every trace state. It is kept in the signature for symmetry with
// env_progress_n (which needs progress to constrain the trace) so the two read as
// a matched pair at call sites; removing it would churn ~28 callers for no
// semantic gain.
pub open spec fn env_holds_along<L, T>(
  progress: spec_fn(L, L) -> bool,
  trace: Seq<L>,
  env: spec_fn(L, T) -> bool,
  t: T,
) -> bool {
  forall |i: int| 0 <= i < trace.len() ==> #[trigger] env(trace[i], t)
}

pub open spec fn env_progress_n<L, T>(
  progress: spec_fn(L, L) -> bool,
  l: L,
  l_prime: L,
  n: nat,
  env: spec_fn(L, T) -> bool,
  t: T,
) -> bool {
  exists |trace: Seq<L>|
    #![trigger trace.len()]
    trace.len() == n + 1 &&
    trace.first() == l &&
    trace.last() == l_prime &&
    is_valid_trace(progress, trace) &&
    env_holds_along(progress, trace, env, t)
}

// An env-good (n1+n2)-trace splits at n1 into two env-good sub-traces sharing the
// midpoint l_mid. Used to thread the wake loop: reach l_mid by k rounds (wake
// fires) then continue to l' (queue drains), each segment still env-good.
pub proof fn env_progress_n_split<L, T>(
  progress: spec_fn(L, L) -> bool,
  l: L,
  l_prime: L,
  n1: nat,
  n2: nat,
  env: spec_fn(L, T) -> bool,
  t: T,
)
  requires
    env_progress_n(progress, l, l_prime, (n1 + n2) as nat, env, t),
  ensures
    exists |l_mid: L|
      #[trigger] env_progress_n(progress, l, l_mid, n1, env, t)
      && env_progress_n(progress, l_mid, l_prime, n2, env, t),
{
  let trace: Seq<L> = choose |trace: Seq<L>|
    trace.len() == (n1 + n2) + 1 && trace.first() == l && trace.last() == l_prime
    && is_valid_trace(progress, trace) && env_holds_along(progress, trace, env, t);
  let l_mid: L = trace[n1 as int];
  let t1: Seq<L> = trace.subrange(0, (n1 + 1) as int);
  let t2: Seq<L> = trace.subrange(n1 as int, (n1 + n2 + 1) as int);
  assert(is_valid_trace(progress, t1)) by {
    assert forall |i: int| 0 <= i < t1.len() - 1 implies progress(#[trigger] t1[i], t1[i + 1]) by {
      assert(t1[i] == trace[i]);
      assert(t1[i + 1] == trace[i + 1]);
    };
  };
  assert(env_holds_along(progress, t1, env, t)) by {
    assert forall |i: int| 0 <= i < t1.len() implies #[trigger] env(t1[i], t) by {
      assert(t1[i] == trace[i]);
    };
  };
  assert(env_progress_n(progress, l, l_mid, n1, env, t)) by {
    assert(t1.len() == n1 + 1 && t1.first() == l && t1.last() == l_mid);
  };
  assert(is_valid_trace(progress, t2)) by {
    assert forall |i: int| 0 <= i < t2.len() - 1 implies progress(#[trigger] t2[i], t2[i + 1]) by {
      assert(t2[i] == trace[n1 + i]);
      assert(t2[i + 1] == trace[n1 + i + 1]);
    };
  };
  assert(env_holds_along(progress, t2, env, t)) by {
    assert forall |i: int| 0 <= i < t2.len() implies #[trigger] env(t2[i], t) by {
      assert(t2[i] == trace[n1 + i]);
    };
  };
  assert(env_progress_n(progress, l_mid, l_prime, n2, env, t)) by {
    assert(t2.len() == n2 + 1 && t2.first() == l_mid && t2.last() == l_prime);
  };
}

// A trace that is env-good for a STRONGER filter is env-good for a weaker one.
// (Used to reuse all end_to_end_env machinery for the stronger env_N filter.)
pub proof fn env_progress_n_weaken<L, T>(
  progress: spec_fn(L, L) -> bool,
  l: L,
  l_prime: L,
  n: nat,
  env_strong: spec_fn(L, T) -> bool,
  env_weak: spec_fn(L, T) -> bool,
  t: T,
)
  requires
    env_progress_n(progress, l, l_prime, n, env_strong, t),
    forall |x: L| #[trigger] env_strong(x, t) ==> env_weak(x, t),
  ensures
    env_progress_n(progress, l, l_prime, n, env_weak, t),
{
  let trace: Seq<L> = choose |trace: Seq<L>|
    trace.len() == n + 1 &&
    trace.first() == l &&
    trace.last() == l_prime &&
    is_valid_trace(progress, trace) &&
    env_holds_along(progress, trace, env_strong, t);
  assert(env_holds_along(progress, trace, env_weak, t)) by {
    assert forall |i: int| 0 <= i < trace.len() implies env_weak(trace[i], t) by {
      assert(env_holds_along(progress, trace, env_strong, t));
      assert(env_strong(trace[i], t));
    };
  };
}

// Base case for building env-good traces (used by non-emptiness of the filter):
// the length-0 trace [s] is env-good iff env(s, t).
pub proof fn env_progress_n_zero<L, T>(
  progress: spec_fn(L, L) -> bool,
  s: L,
  env: spec_fn(L, T) -> bool,
  t: T,
)
  requires
    env(s, t),
  ensures
    env_progress_n(progress, s, s, 0, env, t),
{
  let trace: Seq<L> = seq![s];
  assert(trace.len() == 1);
  assert(trace.first() == s);
  assert(trace.last() == s);
  assert(is_valid_trace(progress, trace));
  assert(env_holds_along(progress, trace, env, t)) by {
    assert forall |i: int| 0 <= i < trace.len() implies env(trace[i], t) by {
      assert(trace[i] == s);
    };
  };
}

// Inductive step for building env-good traces: extend an env-good n-step trace
// ending at l by one env-good progress step to l'.
pub proof fn env_progress_n_step<L, T>(
  progress: spec_fn(L, L) -> bool,
  s: L,
  l: L,
  l_prime: L,
  n: nat,
  env: spec_fn(L, T) -> bool,
  t: T,
)
  requires
    env_progress_n(progress, s, l, n, env, t),
    progress(l, l_prime),
    env(l_prime, t),
  ensures
    env_progress_n(progress, s, l_prime, (n + 1) as nat, env, t),
{
  let trace: Seq<L> = choose |trace: Seq<L>|
    trace.len() == n + 1 &&
    trace.first() == s &&
    trace.last() == l &&
    is_valid_trace(progress, trace) &&
    env_holds_along(progress, trace, env, t);
  let trace2: Seq<L> = trace.push(l_prime);

  assert(trace2.len() == n + 2);
  assert(trace2.first() == s) by { assert(trace2[0] == trace[0]); };
  assert(trace2.last() == l_prime);
  assert(trace[trace.len() - 1] == l);

  assert(is_valid_trace(progress, trace2)) by {
    assert forall |i: int| 0 <= i < trace2.len() - 1 implies
      progress(#[trigger] trace2[i], trace2[i + 1]) by {
      if i < trace.len() - 1 {
        assert(trace2[i] == trace[i]);
        assert(trace2[i + 1] == trace[i + 1]);
        assert(is_valid_trace(progress, trace));
      } else {
        assert(i == trace.len() - 1);
        assert(trace2[i] == l);
        assert(trace2[i + 1] == l_prime);
      }
    };
  };

  assert(env_holds_along(progress, trace2, env, t)) by {
    assert forall |i: int| 0 <= i < trace2.len() implies env(trace2[i], t) by {
      if i < trace.len() {
        assert(trace2[i] == trace[i]);
        assert(env_holds_along(progress, trace, env, t));
      } else {
        assert(trace2[i] == l_prime);
      }
    };
  };
}

// Filter non-emptiness, reduced to a single hypothesis: if an env-preserving
// progress step exists from EVERY env-good state, then from an env-good `s`
// there is an env-good n-step trace, for every n. Uses the base+step toolkit.
// The composed instance discharges the step-existence hypothesis (the one
// remaining hard construction of piece I).
pub proof fn env_good_trace_exists<L, T>(
  progress: spec_fn(L, L) -> bool,
  s: L,
  env: spec_fn(L, T) -> bool,
  t: T,
  n: nat,
)
  requires
    env(s, t),
    forall |x: L| #[trigger] env(x, t) ==> exists |y: L| #![trigger progress(x, y)] progress(x, y) && env(y, t),
  ensures
    exists |l2: L| env_progress_n(progress, s, l2, n, env, t),
  decreases n,
{
  if n == 0 {
    env_progress_n_zero(progress, s, env, t);
  } else {
    env_good_trace_exists(progress, s, env, t, (n - 1) as nat);
    let l_mid: L = choose |l_mid: L|
      #![trigger env_progress_n(progress, s, l_mid, (n - 1) as nat, env, t)]
      env_progress_n(progress, s, l_mid, (n - 1) as nat, env, t);
    env_progress_n_gives_env_at_end(progress, s, l_mid, (n - 1) as nat, env, t);
    assert(env(l_mid, t));
    let l_next: L = choose |y: L| #![trigger progress(l_mid, y)] progress(l_mid, y) && env(y, t);
    assert(progress(l_mid, l_next) && env(l_next, t));
    env_progress_n_step(progress, s, l_mid, l_next, (n - 1) as nat, env, t);
    assert(env_progress_n(progress, s, l_next, n, env, t));
  }
}

// An env-good n-step trace has env at its endpoint (and, by env_holds_along, at
// every state). The response proof concludes the goal at the endpoint.
pub proof fn env_progress_n_gives_env_at_start<L, T>(
  progress: spec_fn(L, L) -> bool,
  l: L,
  l_prime: L,
  n: nat,
  env: spec_fn(L, T) -> bool,
  t: T,
)
  requires
    env_progress_n(progress, l, l_prime, n, env, t),
  ensures
    env(l, t),
{
  let trace: Seq<L> = choose |trace: Seq<L>|
    trace.len() == n + 1 &&
    trace.first() == l &&
    trace.last() == l_prime &&
    is_valid_trace(progress, trace) &&
    env_holds_along(progress, trace, env, t);
  assert(env_holds_along(progress, trace, env, t));
  assert(0 <= 0 < trace.len());
  assert(env(trace[0], t));
  assert(trace[0] == l);
}

pub proof fn env_progress_n_gives_env_at_end<L, T>(
  progress: spec_fn(L, L) -> bool,
  l: L,
  l_prime: L,
  n: nat,
  env: spec_fn(L, T) -> bool,
  t: T,
)
  requires
    env_progress_n(progress, l, l_prime, n, env, t),
  ensures
    env(l_prime, t),
{
  let trace: Seq<L> = choose |trace: Seq<L>|
    trace.len() == n + 1 &&
    trace.first() == l &&
    trace.last() == l_prime &&
    is_valid_trace(progress, trace) &&
    env_holds_along(progress, trace, env, t);
  assert(env_holds_along(progress, trace, env, t));
  assert(0 <= (trace.len() - 1) < trace.len());
  assert(env(trace[trace.len() - 1], t));
  assert(trace[trace.len() - 1] == l_prime);
}

// An env-good n-step trace is in particular an n-step trace.
pub proof fn env_progress_n_implies_progress_n<L, T>(
  progress: spec_fn(L, L) -> bool,
  l: L,
  l_prime: L,
  n: nat,
  env: spec_fn(L, T) -> bool,
  t: T,
)
  requires
    env_progress_n(progress, l, l_prime, n, env, t),
  ensures
    progress_n(progress, l, l_prime, n),
{
  let trace: Seq<L> = choose |trace: Seq<L>|
    trace.len() == n + 1 &&
    trace.first() == l &&
    trace.last() == l_prime &&
    is_valid_trace(progress, trace) &&
    env_holds_along(progress, trace, env, t);
  assert(trace.len() == n + 1 && trace.first() == l && trace.last() == l_prime
    && is_valid_trace(progress, trace));
}

pub open spec fn progress_preserves_wf<L>(ms: ModuleSpec<L>) -> bool {
  forall |l: L, l_prime: L|
    (ms.well_formed)(l) && #[trigger] (ms.progress)(l, l_prime) ==> (ms.well_formed)(l_prime)
}

pub open spec fn progress_n_preserves_wf<L>(ms: ModuleSpec<L>, l: L, l_prime: L, n: nat) -> bool {
  (ms.well_formed)(l) && progress_n(ms.progress, l, l_prime, n) ==> (ms.well_formed)(l_prime)
}

#[verifier::opaque]
pub open spec fn progress_once<L>(progress: spec_fn(L, L) -> bool, l: L, l_prime: L) -> bool {
  progress(l, l_prime)
}

}
