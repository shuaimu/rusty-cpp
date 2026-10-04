// @error pruned-datatype
// @expect-error the spec-only datatype `Log` (pruned because it reaches vstd spec types) is used in executable code
use vstd::prelude::*;

verus! {

pub type Log = Seq<u64>;

pub fn f(_log: &Log) -> u64 {
    0
}

} // verus!
