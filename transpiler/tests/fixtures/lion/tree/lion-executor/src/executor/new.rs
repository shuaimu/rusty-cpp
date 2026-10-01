use lion_reactor::Reactor as LionReactor;
use crate::collections::{VecDeque, MpscReceiver, TaskSlab, TidLedger};
use crate::config::RuntimeConfig;
use crate::proof::invariants::*;
use crate::types::{Reactor, Task};
use super::Executor;
use vstd::prelude::*;

verus! {

impl Executor {
  pub fn new(
  reactor: LionReactor,
  injection_queue: MpscReceiver<Task>,
  config: RuntimeConfig,
  ) -> (result: Self)
    ensures ledger_matches_log(result.ledger, result.log@),
  {
  let result = Executor {
    task_slab: TaskSlab::new(),
    local_queue: VecDeque::new(),
    injection_queue,
    reactor: Reactor::new(reactor),
    event_interval: config.event_interval,
    log: Ghost(Seq::empty()),
    ledger: TidLedger::new(),
  };
  proof {
    assert(result.log@.len() == 0);
  }
  result
  }
}

} // end verus!
