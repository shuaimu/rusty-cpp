use vstd::prelude::*;
use crate::spec::types::ResourceIdView;

verus! {

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct ResourceId(pub u64);

impl View for ResourceId {
  type V = ResourceIdView;

  open spec fn view(&self) -> ResourceIdView {
    self.0 as nat
  }
}

}
