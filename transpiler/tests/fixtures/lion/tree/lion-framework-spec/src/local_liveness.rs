use vstd::prelude::*;

verus! {

// Paper reference: §4's progress-invariant discussion, which describes
// `progress_inv` as a conjunction of local liveness properties — each stating
// that once a certain event occurs in the log, a corresponding outcome follows
// within a bounded number of events. The paper carries that triple in prose
// rather than as a listing; this struct is it. The executor's instance is
// `tick_polls_if_runnable` (the paper's *cached => poll*), whose `timely`
// component is "no Tick(End) occurs in between", i.e. the poll lands before the
// enclosing tick ends. Sketch of the paper's shape:
//
// struct LocalLiveness<L> {
//   acceptance: spec fn(L, nat) -> bool,
//   fulfillment: spec fn(L, nat, nat) -> bool,
//   timely: spec fn(L, nat, nat) -> bool,
// }
// spec fn local_liveness_satisfied<L>(p: LocalLiveness<L>, l: L) -> bool {
//   forall |i: nat| (p.acceptance)(l, i) ==> exists |j: nat|
//     j > i && (p.fulfillment)(l, i, j) && (p.timely)(l, i, j)
// }
//
// Note: We use `int` instead of `nat` for flexibility with log indices.
#[verifier::reject_recursive_types(L)]
pub struct LocalLiveness<L> {
  pub acceptance: spec_fn(L, int) -> bool,
  pub fulfillment: spec_fn(L, int, int) -> bool,
  pub timely: spec_fn(L, int, int) -> bool,
}

pub open spec fn local_liveness_satisfied<L>(p: LocalLiveness<L>, l: L) -> bool {
  forall |i: int|
    #[trigger] (p.acceptance)(l, i) ==>
    exists |j: int|
      #![trigger (p.fulfillment)(l, i, j)]
      j > i &&
      (p.fulfillment)(l, i, j) &&
      (p.timely)(l, i, j)
}

}
