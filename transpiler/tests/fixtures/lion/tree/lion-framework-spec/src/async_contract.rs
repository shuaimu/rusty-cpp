use vstd::prelude::*;
#[cfg(verus_keep_ghost)]
use crate::module_spec::{ModuleSpec, progress_n, env_progress_n, progress_preserves_wf};

verus! {

// Paper reference: the liveness-framework figure (§4, lst:liveness-framework).
// The three fields match the paper's AsyncContract one-to-one (acceptance,
// fulfillment, assumption); see bounded_liveness_env_without_arrival below for
// how the paper's single `liveness` definition maps onto the two forms here.
#[verifier::reject_recursive_types(L)]
#[verifier::reject_recursive_types(T)]
pub struct AsyncContract<L, T> {
  pub acceptance: spec_fn(L, T) -> bool,
  pub fulfillment: spec_fn(L, T) -> bool,
  pub assumption: spec_fn(L, T) -> bool,
}
pub open spec fn ensures_response_within<L, T>(
  l: L,
  t: T,
  ms: ModuleSpec<L>,
  ac: AsyncContract<L, T>,
  n: nat
) -> bool {
  forall |l_prime: L|
    #[trigger] progress_n(ms.progress, l, l_prime, n)
    ==> (ac.fulfillment)(l_prime, t)
}

pub open spec fn ensures_response<L, T>(
  l: L,
  t: T,
  ms: ModuleSpec<L>,
  ac: AsyncContract<L, T>
) -> bool {
  exists |n: nat| ensures_response_within(l, t, ms, ac, n)
}

// Contract liveness: once the trigger event (acceptance) has occurred and the
// assumption holds, the response is guaranteed within bounded progress. (There is
// no separate `arrival` gate — the trigger event IS the arrival, so acceptance is
// used positively rather than as a `!trigger && arrival` pair; the paper's
// `liveness` definition uses the same positive-acceptance shape.)
// Paper relation: this is the paper's `liveness` with the trace filter dropped
// (plain progress_n); the paper presents only the filtered env form below.
pub open spec fn bounded_liveness_without_arrival<L, T>(
  ms: ModuleSpec<L>,
  ac: AsyncContract<L, T>
) -> bool {
  progress_preserves_wf(ms) &&
  forall |t: T, l: L|
    (ms.well_formed)(l) &&
    (ac.acceptance)(l, t) &&
    (ac.assumption)(l, t) ==>
    ensures_response(l, t, ms, ac)
}

pub open spec fn env_response_within_trace<L, T>(
  l: L,
  t: T,
  ms: ModuleSpec<L>,
  ac: AsyncContract<L, T>,
  env: spec_fn(L, T) -> bool,
  n: nat,
) -> bool {
  // Response-filter form only (no filter non-emptiness — soft-vacuity closure is
  // out of scope for this work).
  forall |l_prime: L|
    #[trigger] env_progress_n(ms.progress, l, l_prime, n, env, t)
      ==> (ac.fulfillment)(l_prime, t)
}

// Env-form contract liveness: the response is only required along runs where the
// environmental assumption `env` holds at every state (the response-filter form the
// composed proof consumes for the timer, io, and drainage contracts).
//
// Paper relation: the paper's `liveness` (§4, lst:liveness-framework) is THIS form
// with `env` folded into the contract's `assumption` field — its
// `step_n(ms.step, l, l', n, ac.assumption)` is `env_progress_n(ms.progress, l,
// l', n, env, t)` with env := assumption. The code keeps `env` a separate
// parameter so a contract's own precondition and the trace-filtered environmental
// assumption can differ (env-form instances typically set `assumption` to true
// and pass e.g. a queue bound as `env`).
//
// Why two carriers here but a single field in the paper: the first formulation of
// `assumption` proved too weak to state the runs we needed to filter, and
// re-stating it in place would have invalidated a large body of finished proofs.
// We therefore introduced `env` alongside it and migrated the real assumptions
// into `env` one contract at a time; what remains is `assumption == true` in the
// env-form instances, with the substance carried by `env`. The split is
// historical, not semantic — `env` is expressible as `assumption`, which is the
// single-field template the paper presents.
pub open spec fn bounded_liveness_env_without_arrival<L, T>(
  ms: ModuleSpec<L>,
  ac: AsyncContract<L, T>,
  env: spec_fn(L, T) -> bool,
) -> bool {
  progress_preserves_wf(ms) &&
  forall |t: T, l: L|
    #![trigger env(l, t)]
    (ms.well_formed)(l) &&
    (ac.acceptance)(l, t) &&
    env(l, t) ==>
    exists |n: nat| #[trigger] env_response_within_trace(l, t, ms, ac, env, n)
}

}
