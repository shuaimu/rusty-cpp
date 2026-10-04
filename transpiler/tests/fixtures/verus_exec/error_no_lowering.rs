// @error no-lowering
// @expect-error vstd's `StrSliceExecFns::unicode_len` has no C++ lowering
use vstd::prelude::*;

verus! {

pub fn f(s: &str) -> usize {
    s.unicode_len()
}

} // verus!
