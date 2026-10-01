// @error other-verus-macro
// @expect-error `verus!` is a Verus macro this pass does not erase
// A verus! in statement position is not item-level.
use vstd::prelude::*;

pub fn f() {
    verus! {
        fn g() {}
    }
}
