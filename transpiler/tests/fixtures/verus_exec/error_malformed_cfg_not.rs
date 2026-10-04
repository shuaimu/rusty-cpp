// @error malformed-driver-cfg
// @expect-error malformed cfg predicate `not (verus_keep_ghost , test)`: `not` takes one argument
#[cfg(not(verus_keep_ghost, test))]
pub fn f() {}
