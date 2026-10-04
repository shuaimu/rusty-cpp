// @rule imports
// @expect-lowered use std::cmp::max;
// @reject-lowered Ordering
// @reject-lowered use vstd
// @expect-cpp using rusty::cmp::max;
// @reject-cpp using rusty::cmp::Ordering;
// T2 rule 5: use trees rooted at vstd are removed, and so is an import whose
// every use the erasure removed (std::cmp::Ordering is only named by a spec
// fn). Imports still used stay.
use vstd::prelude::*;
use vstd::seq::Seq;
use std::cmp::Ordering;
use std::cmp::max;

verus! {

pub open spec fn ordered(o: Ordering) -> bool {
    o == Ordering::Less
}

pub fn larger(a: u64, b: u64) -> u64 {
    max(a, b)
}

} // verus!
