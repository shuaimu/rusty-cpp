use crate::types::{Instant, ResourceId};
use crate::spec::types::InstantView;
use std::cmp::Ordering;
use vstd::prelude::*;

verus! {

#[derive(Copy, Clone)]
pub struct TimerEntry {
  pub deadline: Instant,
  pub resource_id: ResourceId,
  pub log_index: Ghost<int>,
}

impl View for TimerEntry {
  type V = (InstantView, nat, int);

  open spec fn view(&self) -> (InstantView, nat, int) {
    (self.deadline@, self.resource_id@, self.log_index@)
  }
}

}

impl PartialEq for TimerEntry {
  fn eq(&self, other: &Self) -> bool {
    self.deadline == other.deadline && self.resource_id == other.resource_id
  }
}

impl Eq for TimerEntry {}

impl PartialOrd for TimerEntry {
  fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
    Some(self.cmp(other))
  }
}

impl Ord for TimerEntry {
  fn cmp(&self, other: &Self) -> Ordering {
    match self.deadline.cmp(&other.deadline) {
      Ordering::Equal => self.resource_id.cmp(&other.resource_id),
      ord => ord,
    }
  }
}
