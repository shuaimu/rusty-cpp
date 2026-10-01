use super::IoResult;
use crate::os::OsEvent;
use vstd::prelude::*;

verus! {

// The buffer a backend's wait fills (see crate::os::OsBackend::wait), reused
// across parks.
#[verifier::external_body]
pub struct IoEventQueue {
  pub(crate) inner: Vec<OsEvent>,
}

impl View for IoEventQueue {
  type V = int;

  #[verifier::external_body]
  spec fn view(&self) -> int {
    unimplemented!()
  }
}

impl IoEventQueue {
  #[verifier::external_body]
  pub fn with_capacity(capacity: usize) -> IoResult<Self> {
    IoResult::Ok(IoEventQueue {
      inner: Vec::with_capacity(capacity),
    })
  }
}

}
