// @rule empty-statements
// @reject-lowered {}
// @expect-cpp const auto ab = rusty::detail::deref_if_pointer_like(a) + rusty::detail::deref_if_pointer_like(b);
// @expect-cpp return std::move(r);
// T2 rule 6: the empty `{}` / `{};` statements EraseAll leaves where proof
// blocks and assertions were are removed.
use vstd::prelude::*;

verus! {

pub fn sum3(a: u64, b: u64, c: u64) -> (r: u64)
    requires
        a < 100,
        b < 100,
        c < 100,
    ensures
        r == a + b + c,
{
    let ab = a + b;
    proof {
        assert(ab < 200);
    }
    assert(ab == a + b);
    let r = ab + c;
    proof {
        assert(r < 300);
    }
    r
}

} // verus!
