use crate::types::ReactorGuard;
use super::Executor;
use vstd::prelude::*;

verus! {

impl Executor {
  #[verifier::external_body]
  pub fn enter(&mut self) -> (result: ReactorGuard)
  {
  self.reactor.enter()
  }
}

} // end verus!
