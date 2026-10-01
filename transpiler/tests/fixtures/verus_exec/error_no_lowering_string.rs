// @error no-lowering
// @expect-error vstd's `StringExecFns::append` has no C++ lowering
use vstd::prelude::*;

verus! {

pub fn app(s: &mut String, t: &str) {
    s.append(t);
}

} // verus!
