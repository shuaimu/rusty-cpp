// @error malformed-driver-cfg
// @expect-error malformed `#[cfg(..)]`
#[cfg(verus_keep_ghost, test)]
pub fn f() {}
