// @rule view
// @expect-lowered impl<V: Copy> Pair<V> {
// @expect-lowered pub trait Keyed {
// @expect-lowered pub fn count_some<V: Copy>(pair: &Pair<V>) -> u64 {
// @expect-cpp std::array<rusty::Option<V>, 2> items;
// @expect-cpp class Keyed {
// @expect-cpp uint64_t count_some(const Pair<V>& pair) {
// T2 rule 1: View and DeepView impls are dropped, and so is every View or
// DeepView bound (type parameters, where-clauses, supertraits).
use vstd::prelude::*;

verus! {

pub struct Pair<V> {
    pub items: [Option<V>; 2],
}

impl<V: View> View for Pair<V> {
    type V = Seq<Option<V::V>>;

    open spec fn view(&self) -> Self::V {
        Seq::empty()
    }
}

impl<V: DeepView> DeepView for Pair<V> {
    type V = Seq<Option<V::V>>;

    open spec fn deep_view(&self) -> Self::V {
        Seq::empty()
    }
}

impl<V: View + Copy> Pair<V> where V: View {
    pub fn first(&self) -> Option<V> {
        self.items[0]
    }
}

pub trait Keyed: View {
    fn key(&self) -> u64;
}

pub fn count_some<V: View + Copy>(pair: &Pair<V>) -> u64 {
    let mut n: u64 = 0;
    if pair.items[0].is_some() {
        n = n + 1;
    }
    if pair.items[1].is_some() {
        n = n + 1;
    }
    n
}

} // verus!
