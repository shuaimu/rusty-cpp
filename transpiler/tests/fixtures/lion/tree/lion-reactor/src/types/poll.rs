use crate::os::OsBackend;
use vstd::prelude::*;

verus! {

// The reactor's OS backend (see crate::os). Opaque to verification: the
// verified reactor reaches it only through the trusted leaves of reactor/ext.rs
// and reactor/new.rs.
#[verifier::external_body]
pub struct Poll {
  pub(crate) inner: Box<dyn OsBackend>,
}

impl View for Poll {
  type V = int;

  #[verifier::external_body]
  spec fn view(&self) -> int {
    unimplemented!()
  }
}

}

impl Poll {
  pub fn new(backend: Box<dyn OsBackend>) -> Poll {
    Poll { inner: backend }
  }
}
