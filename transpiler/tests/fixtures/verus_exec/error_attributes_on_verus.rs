// @error attributes-on-verus
// @expect-error attributes on a `verus!` invocation in `crate` are unsupported
use vstd::prelude::*;

#[allow(unused)]
verus! {
    pub fn f() {}
}
