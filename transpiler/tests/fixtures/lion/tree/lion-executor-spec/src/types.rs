use std::task::Poll as StdPoll;
use vstd::prelude::*;

verus! {

pub type TID = nat;

pub struct TaskView {
  pub id: TID,
}

pub enum PollResult<T> {
  Ready(T),
  Pending,
  Invalid,
}

impl<T> PollResult<T> {
  pub exec fn is_ready(&self) -> (result: bool)
  {
  matches!(self, PollResult::Ready(_))
  }

  pub exec fn is_pending(&self) -> (result: bool)
  {
  matches!(self, PollResult::Pending)
  }

  pub exec fn is_invalid(&self) -> (result: bool)
  {
  matches!(self, PollResult::Invalid)
  }
}

impl<T: View> View for PollResult<T> {
  type V = PollResult<T::V>;

  open spec fn view(&self) -> PollResult<T::V> {
  match self {
    PollResult::Ready(v) => PollResult::Ready(v@),
    PollResult::Pending => PollResult::Pending,
    PollResult::Invalid => PollResult::Invalid,
  }
  }
}

} // end verus!

impl<T> From<StdPoll<T>> for PollResult<T> {
  fn from(poll: StdPoll<T>) -> Self {
  match poll {
    StdPoll::Ready(v) => PollResult::Ready(v),
    StdPoll::Pending => PollResult::Pending,
  }
  }
}

impl<T> From<PollResult<T>> for StdPoll<T> {
  fn from(poll: PollResult<T>) -> Self {
  match poll {
    PollResult::Ready(v) => StdPoll::Ready(v),
    PollResult::Pending => StdPoll::Pending,
    PollResult::Invalid => StdPoll::Pending,
  }
  }
}
