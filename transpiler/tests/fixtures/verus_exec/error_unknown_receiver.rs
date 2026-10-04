// @error unknown-receiver
// @expect-error cannot tell whether `.set(..)` on `w . get ()` is vstd's `VecAdditionalExecFns::set`
use vstd::prelude::*;

pub struct Wrapper {
    pub v: [u64; 2],
}

verus! {

pub fn f(w: &Wrapper) {
    w.get().set(0, 1);
}

} // verus!
