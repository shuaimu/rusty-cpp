// @error other-verus-macro
// @expect-error `proof!` is a Verus macro this pass does not erase
use vstd::prelude::*;

pub fn f() -> u64 {
    proof! {
        assert(true);
    }
    1
}
