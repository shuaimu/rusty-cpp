// @error other-verus-macro
// @expect-error `verus_impl!` is a Verus macro this pass does not erase
// Only item-level verus! { } is erased; any other Verus macro fails closed.
use vstd::prelude::*;

verus_impl! {
    pub fn f() {}
}
