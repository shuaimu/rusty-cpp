// @error unparseable-source
// @expect-error --verus-exec could not parse the source
// A file with Verus constructs is parsed before erasure; one that does not
// parse is an error, not a pass-through.
use vstd::prelude::*;

verus! {
    pub fn f() {}
}

pub fn g( {}
