// @rule driver-cfg
// @expect-lowered pub fn kept_outside() -> u64 {
// @expect-lowered #[inline]
// @reject-lowered ghost_only
// @reject-lowered ghost_value
// @reject-lowered cfg
// @reject-lowered trusted
// @expect-cpp uint64_t kept_outside() {
// @expect-cpp uint64_t read(const Slot& s) {
// @reject-cpp ghost_only
// @reject-cpp ghost_value
// cfg(verus_keep_ghost) and cfg(verus_keep_ghost_body) are false, inside and
// outside verus!, on items, fields and in cfg_attr; what they gate is gone and
// `not(..)` of them is true. Lion's #![cfg_attr(verus_keep_ghost,
// verus::trusted)] file attribute disappears.
#![cfg_attr(verus_keep_ghost, verus::trusted)]
use vstd::prelude::*;

#[cfg(verus_keep_ghost)]
pub fn ghost_only_outside() -> u64 {
    1
}

#[cfg(not(verus_keep_ghost))]
pub fn kept_outside() -> u64 {
    2
}

verus! {

pub struct Slot {
    pub value: u64,
    #[cfg(verus_keep_ghost)]
    pub ghost_value: u64,
}

#[cfg(verus_keep_ghost_body)]
pub fn ghost_only_inside() -> u64 {
    3
}

#[cfg_attr(not(verus_keep_ghost), inline)]
pub fn read(s: &Slot) -> u64 {
    s.value
}

} // verus!
