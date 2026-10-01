use std::task::Waker as StdWaker;
use vstd::prelude::*;
use crate::spec::types::WakerView;

verus! {

#[verifier::external_body]
pub struct Waker {
  pub(crate) inner: StdWaker,
}

impl View for Waker {
  type V = WakerView;

  #[verifier::external_body]
  spec fn view(&self) -> WakerView {
    unimplemented!()
  }
}

impl Clone for Waker {
  #[verifier::external_body]
  fn clone(&self) -> (result: Self)
    ensures
      result@ == self@
  {
    Waker {
      inner: self.inner.clone(),
    }
  }
}

} // end verus!

impl Waker {
  pub fn from_std(waker: StdWaker) -> Self {
    Waker { inner: waker }
  }
}
