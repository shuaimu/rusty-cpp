// @error driver-cfg-file
// @expect-error removes the whole module; unsupported
#![cfg(verus_keep_ghost)]

pub fn f() -> u64 {
    1
}
