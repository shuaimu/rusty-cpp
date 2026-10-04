// @error spec-trait
// @expect-error the vstd spec trait `View` is used in executable code
// A View bound is dropped, but View named as a type is not a bound.
use vstd::prelude::*;

verus! {

pub fn f(v: &dyn View<V = u64>) -> u64 {
    0
}

} // verus!
