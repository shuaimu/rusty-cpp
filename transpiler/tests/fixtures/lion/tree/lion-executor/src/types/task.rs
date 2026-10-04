use std::task::{Context, Poll};
use super::{BoxedFuture, TaskId};
use lion_executor_spec::types::TaskView;
use vstd::prelude::*;

verus! {

pub struct Task {
  pub id: TaskId,
  pub future: BoxedFuture,
}

impl View for Task {
  type V = TaskView;

  open spec fn view(&self) -> TaskView {
  TaskView {
    id: self.id@,
  }
  }
}

impl Task {
  pub exec fn new(id: TaskId, future: BoxedFuture) -> (result: Self)
  {
  Self { id, future }
  }

  pub exec fn id(&self) -> (result: TaskId)
  ensures result@ == self@.id,
  {
  self.id
  }
}

} // end verus!

impl Task {
  pub(crate) fn poll(&mut self, cx: &mut Context) -> Poll<()> {
  self.future.poll(cx)
  }
}
