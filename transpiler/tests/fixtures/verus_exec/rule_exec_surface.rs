// @rule exec-surface
// @expect-lowered local[0] = v;
// @expect-lowered let __vstd_exec_index = next_index(1);
// @expect-lowered let __vstd_exec_value = v + 1;
// @reject-lowered .set(
// @expect-cpp local.at(static_cast<size_t>(0)) = std::move(v);
// @expect-cpp const auto __vstd_exec_index = ::next_index(static_cast<size_t>(1));
// @expect-cpp this->words.at(__vstd_exec_index) = std::move(__vstd_exec_value);
// @reject-cpp .set(
// T3: vstd's executable surface. ArrayAdditionalExecFns::set becomes an index
// assignment; with an argument that could have a side effect, the arguments
// are evaluated first, in order, as rustc does.
use vstd::prelude::*;

verus! {

pub struct Words {
    pub words: [u64; 4],
}

fn next_index(i: usize) -> usize {
    i + 1
}

impl Words {
    pub fn fill(&mut self, v: u64) {
        let mut local: [u64; 4] = [0; 4];
        local.set(0, v);
        self.words.set(1, local[0]);
        self.words.set(next_index(1), v + 1);
    }
}

} // verus!
