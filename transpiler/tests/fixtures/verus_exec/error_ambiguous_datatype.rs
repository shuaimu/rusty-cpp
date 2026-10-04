// @error ambiguous-datatype
// @expect-error spec-only datatype name(s) also name executable datatypes
use vstd::prelude::*;

pub mod spec {
    use vstd::prelude::*;

    verus! {
    pub type Id = nat;
    } // verus!
}

pub mod exec {
    pub struct Id(pub u64);
}
