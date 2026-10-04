use vstd::prelude::*;

verus! {

// Paper reference: Listing 4 (Invariant templates over event logs)
// specification.tex lines 250-258
//
// struct ActionSafety<L> {
//   acceptance: spec fn(L, nat) -> bool,
//   validity: spec fn(L, nat) -> bool,
// }
// spec fn action_safety_satisfied<L>(p: ActionSafety<L>, l: L) -> bool {
//   forall |i: nat| (p.acceptance)(l, i) ==> (p.validity)(l, i)
// }
//
// Note: We use `int` instead of `nat` for flexibility with log indices.
#[verifier::reject_recursive_types(L)]
pub struct ActionSafety<L> {
  pub acceptance: spec_fn(L, int) -> bool,
  pub validity: spec_fn(L, int) -> bool,
}

pub open spec fn action_safety_satisfied<L>(p: ActionSafety<L>, l: L) -> bool {
  forall |i: int| #[trigger] (p.acceptance)(l, i) ==> (p.validity)(l, i)
}

}
