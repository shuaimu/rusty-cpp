// @error ghost-flow-field_base
// @expect-error the ghost value `h . ghost_slot` is used as a field base
// A ghost value has no runtime value; using it where its value matters fails
// closed instead of reading the empty tag.
use vstd::prelude::*;

verus! {

pub struct Holder {
    pub value: u64,
    pub ghost_slot: Ghost<u64>,
    pub ghost_items: Ghost<[u64; 2]>,
}

fn takes_u64(_x: u64) {}

pub fn f(h: &Holder, items: [u64; 2]) -> u64 {
    let g: Ghost<u64> = Ghost::assume_new();
    let x = h.ghost_slot.0;
    0
}

} // verus!
