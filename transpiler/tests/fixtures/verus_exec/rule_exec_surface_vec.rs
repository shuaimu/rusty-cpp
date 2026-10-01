// @rule exec-surface
// @compile no: rusty::Vec is the vec_port C++ module, which the header-only
// @expect-lowered v[0] = 7;
// @expect-lowered ::core::mem::swap(&mut v[1], x);
// @expect-cpp (*v_shadow1)[static_cast<size_t>(0)] = static_cast<uint64_t>(7);
// @expect-cpp rusty::mem::swap((*v_shadow1)[static_cast<size_t>(1)], rusty::detail::deref_if_pointer(x));
// @reject-cpp .set(
// @reject-cpp set_and_swap
// single-file output this driver compiles cannot import.
// T3: VecAdditionalExecFns::set lowers to an index assignment and
// set_and_swap to core::mem::swap of the slot and the argument (vstd's body).
use vstd::prelude::*;

verus! {

pub fn update(v: &mut Vec<u64>, x: &mut u64) {
    v.set(0, 7);
    v.set_and_swap(1, x);
}

} // verus!
