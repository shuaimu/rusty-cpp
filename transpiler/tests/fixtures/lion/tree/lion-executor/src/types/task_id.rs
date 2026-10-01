use vstd::prelude::*;

verus! {

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TaskId(pub u64);

impl View for TaskId {
  type V = nat;

  open spec fn view(&self) -> nat {
  self.0 as nat
  }
}

impl DeepView for TaskId {
  type V = nat;

  open spec fn deep_view(&self) -> nat {
  self.0 as nat
  }
}

}
