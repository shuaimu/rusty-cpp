// @error verus-rejects-block
// @expect-error Verus rejected a `verus!` block
use vstd::prelude::*;

verus! {
    fn f() -> {}
}
