// @error vstd-path
// @expect-error `vstd::pervasive::runtime_assert` is a vstd path this pass does not lower
use vstd::prelude::*;

verus! {

pub fn f(x: u64) -> u64 {
    vstd::pervasive::runtime_assert(x > 0);
    x
}

} // verus!
