// @rule ghost
// @expect-cpp [[no_unique_address]] rusty::Ghost log_index;
// @expect-cpp [[no_unique_address]] rusty::Ghost perm;
// @expect-cpp .log_index = rusty::Ghost{}, .perm = rusty::Ghost{}
// @expect-cpp void keep(rusty::Ghost _g)
// @expect-cpp rusty::Option<std::tuple<uint64_t, rusty::Ghost, rusty::Ghost>> pop(const Entry& entry)
// @expect-cpp rusty::Ghost when = rusty::Ghost{};
// @expect-lowered Some((d, _index, _when)) => d,
// T2 rule 2: Ghost<T> and Tracked<T> become the empty tag rusty::Ghost (with
// [[no_unique_address]] on fields), their constructors a rusty::Ghost{}
// value, and T is never emitted. Tuple arity and patterns stay as written, a
// ghost value may be stored, passed to a ghost parameter and returned.
use vstd::prelude::*;

verus! {

pub struct Entry {
    pub deadline: u64,
    pub log_index: Ghost<int>,
    pub perm: Tracked<u8>,
}

pub fn make(deadline: u64) -> Entry {
    Entry {
        deadline,
        log_index: Ghost::assume_new_fallback(|| unreachable!()),
        perm: Tracked::assume_new(),
    }
}

fn keep(_g: Ghost<nat>) {}

pub fn pop(entry: &Entry) -> Option<(u64, Ghost<int>, Ghost<nat>)> {
    let index: Ghost<int> = Ghost::assume_new_fallback(|| unreachable!());
    let when: Ghost<nat> = Ghost::<nat>::assume_new();
    keep(when);
    Some((entry.deadline, index, when))
}

pub fn deadline_of(entry: &Entry) -> u64 {
    match pop(entry) {
        Some((d, _index, _when)) => d,
        None => 0,
    }
}

} // verus!
