#![cfg_attr(verus_keep_ghost, verus::trusted)]

pub(crate) mod enter;
pub mod ext;
pub(crate) mod new;
mod park;
mod register;
mod timer;
mod waker;

use crate::resource_slab::ResourceSlab;
use crate::spec::log::Log;
use lion_timer_wheel::TimerWheel;
use crate::types::{IoEventQueue, Poll, ResourceId, Waker};
use vstd::prelude::*;

verus! {

pub struct Reactor {
  pub next_resource_id: u64,
  pub poll: Poll,
  pub events: IoEventQueue,
  pub wheel: TimerWheel,
  pub resources: ResourceSlab,
  pub log: Ghost<Log>,
  pub free_rids: Vec<u64>,
  pub pending_deregister: Option<(u64, u64)>,
}

pub struct ReactorGuard {
  // Address of the reactor this guard entered (identity only, never
  // dereferenced): drop clears the thread's entry only if it is still this one.
  pub(crate) reactor: usize,
}

}
