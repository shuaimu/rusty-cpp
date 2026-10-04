// @rule prune-datatypes
// @reject-lowered TaskView
// @reject-lowered Event
// @reject-lowered Tid
// @reject-lowered Log
// @expect-lowered pub enum Source {
// @expect-cpp enum class Source {
// @expect-cpp [[no_unique_address]] rusty::Ghost log;
// @reject-cpp TaskView
// @reject-cpp Event
// @reject-cpp Tid
// T2 rule 4: every datatype from which a vstd spec type is reachable is
// removed with its impls: here a type alias to nat, a struct holding a Seq, an
// enum holding that struct, and an alias to the enum. Datatypes that reach no
// spec type stay.
use vstd::prelude::*;

verus! {

pub type Tid = nat;

pub struct TaskView {
    pub id: Tid,
    pub history: Seq<u64>,
}

impl TaskView {
    pub fn zero(&self) -> u64 {
        0
    }
}

pub enum Event {
    Spawned(TaskView),
    Idle,
}

pub type Log = Seq<Event>;

pub enum Source {
    Timer,
    Io,
}

pub struct Counter {
    pub value: u64,
    pub log: Ghost<Log>,
}

pub fn bump(c: &mut Counter) -> u64 {
    c.value = c.value + 1;
    c.value
}

} // verus!
