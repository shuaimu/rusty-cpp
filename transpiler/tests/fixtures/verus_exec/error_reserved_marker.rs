// @error reserved-marker
// @expect-error `RustyVerusGhost` is reserved for lowered Verus ghost state
use vstd::prelude::*;

verus! {

pub struct RustyVerusGhost;

} // verus!
