// @error spec-type
// @expect-error the vstd spec type `nat` is used in executable code
use vstd::prelude::*;

verus! {

pub fn f(x: nat) -> u64 {
    0
}

} // verus!
