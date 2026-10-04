use super::ResourceId;
use vstd::prelude::*;
use crate::spec::types::IoEventView;

verus! {

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum IoMode {
  Readable,
  Writable,
}

impl IoMode {
  pub open spec fn spec_is_readable(&self) -> bool {
    matches!(self, IoMode::Readable)
  }

  pub open spec fn spec_is_writable(&self) -> bool {
    matches!(self, IoMode::Writable)
  }

  pub fn is_readable(&self) -> (result: bool)
    ensures result == self.spec_is_readable()
  {
    match self {
      IoMode::Readable => true,
      IoMode::Writable => false,
    }
  }

  pub fn is_writable(&self) -> (result: bool)
    ensures result == self.spec_is_writable()
  {
    match self {
      IoMode::Readable => false,
      IoMode::Writable => true,
    }
  }
}

pub struct IoEvent {
  pub resource_id: ResourceId,
  pub mode: IoMode,
  pub error: bool,
  pub read_closed: bool,
  pub write_closed: bool,
}

impl View for IoEvent {
  type V = IoEventView;

  open spec fn view(&self) -> IoEventView {
    IoEventView {
      resource_id: self.resource_id@,
      readable: self.mode.spec_is_readable(),
      writable: self.mode.spec_is_writable(),
    }
  }
}

impl DeepView for IoEvent {
  type V = IoEventView;

  open spec fn deep_view(&self) -> IoEventView {
    IoEventView {
      resource_id: self.resource_id@,
      readable: self.mode.spec_is_readable(),
      writable: self.mode.spec_is_writable(),
    }
  }
}

}
