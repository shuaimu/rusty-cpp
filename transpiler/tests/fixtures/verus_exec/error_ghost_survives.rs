// @error ghost-survives
// @expect-error `Ghost::new` survives lowering
// Only Ghost<T> types and Ghost::assume_new* constructors lower.
use vstd::prelude::*;

verus! {

pub fn f() -> u64 {
    let _g: Ghost<u64> = Ghost::new(1);
    0
}

} // verus!
