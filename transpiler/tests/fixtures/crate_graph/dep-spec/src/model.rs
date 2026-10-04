use vstd::prelude::*;

verus! {

pub type ModelView = Seq<u32>;

pub open spec fn model_len(m: ModelView) -> nat {
    m.len()
}

} // verus!
