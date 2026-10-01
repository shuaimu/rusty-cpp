// @rule verus-erasure
// @expect-lowered pub const LIMIT: u64 = 10;
// @expect-lowered pub fn count_to(n: u64) -> u64 {
// @reject-lowered spec_limit
// @reject-lowered limit_positive
// @reject-lowered invariant
// @reject-lowered decreases
// @reject-lowered start
// @expect-cpp constexpr uint64_t LIMIT = static_cast<uint64_t>(10);
// @expect-cpp return rusty::wrapping_add(a, static_cast<std::remove_cvref_t<decltype(a)>>(std::move(b)));
// @expect-cpp while (rusty::detail::deref_if_pointer_like(i) < rusty::detail::deref_if_pointer_like(n)) {
// @reject-cpp spec_limit
// @reject-cpp limit_positive
// T1: what plain rustc compiles from a verus! block. Spec and proof fns,
// requires/ensures/invariant/decreases, ghost lets and proof blocks vanish;
// exec fns, consts, #[verifier::external_body] bodies and loops stay.
use vstd::prelude::*;

verus! {

pub const LIMIT: u64 = 10;

pub open spec fn spec_limit() -> nat {
    10
}

proof fn limit_positive()
    ensures
        spec_limit() > 0,
{
}

#[verifier::external_body]
pub fn opaque_add(a: u64, b: u64) -> u64 {
    a.wrapping_add(b)
}

pub fn count_to(n: u64) -> (r: u64)
    requires
        n <= LIMIT,
    ensures
        r == n,
{
    let ghost start = 0int;
    let mut i: u64 = 0;
    while i < n
        invariant
            i <= n,
        decreases n - i,
    {
        i = i + 1;
    }
    proof {
        limit_positive();
    }
    opaque_add(i, 0)
}

} // verus!
