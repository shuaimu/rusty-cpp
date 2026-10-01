use vstd::prelude::*;
use crate::os::RawFd;
use crate::spec::types::SourceView;

verus! {

// An io resource as the OS knows it: a raw file descriptor, which the caller
// keeps open while it is registered.
#[verifier::external_body]
pub struct Source {
  pub(crate) fd: RawFd,
}

impl View for Source {
  type V = SourceView;

  #[verifier::external_body]
  spec fn view(&self) -> SourceView {
    unimplemented!()
  }
}

} // end verus!

impl Source {
  pub fn new(fd: RawFd) -> Self {
    Source { fd }
  }

  pub fn fd(&self) -> RawFd {
    self.fd
  }
}
