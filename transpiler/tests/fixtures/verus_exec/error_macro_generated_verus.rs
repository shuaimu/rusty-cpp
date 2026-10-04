// @error macro-generated-verus
// @expect-error `macro_rules!` carries a `verus!` invocation in its tokens
// Lion's former *_log_action shape: the erasure never sees what the
// expansion produces, so the macro itself fails closed.
use vstd::prelude::*;

macro_rules! log_action {
    ($name:ident) => {
        verus! {
            pub fn $name() {}
        }
    };
}

log_action!(tick);
