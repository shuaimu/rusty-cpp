use vstd::prelude::*;

verus! {

pub struct Widget {
    pub v: u32,
    pub log: Ghost<Seq<u32>>,
}

impl Widget {
    pub fn new(v: u32) -> Self {
        Widget { v, log: Ghost::assume_new() }
    }

    pub fn bump(&mut self)
        requires old(self).v < 100,
    {
        self.v = self.v + 1;
    }

    pub fn value(&self) -> u32 {
        self.v
    }
}

} // verus!
