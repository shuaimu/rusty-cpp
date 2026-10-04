mod new;
mod enter;
mod tick;
mod next_task;
mod poll_task;
mod park;
mod wake_deferred;
mod ext;

use crate::collections::{VecDeque, MpscReceiver, TaskSlab, TidLedger};
use crate::spec::log::Log;
use crate::types::{Reactor, Task, TaskId};
use vstd::prelude::*;

verus! {

pub struct Executor {
  pub task_slab: TaskSlab,
  pub local_queue: VecDeque<TaskId>,
  pub injection_queue: MpscReceiver<Task>,
  pub reactor: Reactor,
  pub event_interval: usize,
  pub log: Ghost<Log>,
  pub ledger: TidLedger,
}

}
