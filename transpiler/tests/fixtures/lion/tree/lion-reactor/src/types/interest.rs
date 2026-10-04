use vstd::prelude::*;
use crate::spec::types::InterestView;

verus! {

#[derive(Copy, Clone, PartialEq, Eq)]
pub struct Interest {
  pub readable: bool,
  pub writable: bool,
}

impl View for Interest {
  type V = InterestView;

  open spec fn view(&self) -> InterestView {
    (self.readable, self.writable)
  }
}

impl Interest {
  pub const READABLE: Interest = Interest {
    readable: true,
    writable: false,
  };

  pub const WRITABLE: Interest = Interest {
    readable: false,
    writable: true,
  };

  pub const READABLE_WRITABLE: Interest = Interest {
    readable: true,
    writable: true,
  };

  pub fn is_readable(&self) -> bool {
    self.readable
  }

  pub fn is_writable(&self) -> bool {
    self.writable
  }
}

}
