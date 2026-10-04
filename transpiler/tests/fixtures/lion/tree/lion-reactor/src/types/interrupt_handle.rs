use crate::os::OsInterrupt;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use vstd::prelude::*;

// The reactor's cross-thread wake: the backend's interrupt plus a coalescing
// flag. `wake` (any thread) signals the backend only on the first wake since
// the owner's last `reset`; the owner resets before it drains its cross-thread
// queue and parks, so a wake that finds the flag already set is covered by the
// signal that set it (see crate::os).
pub struct InterruptHandleShared {
  os: Arc<dyn OsInterrupt>,
  notified: AtomicBool,
}

#[derive(Clone)]
pub struct InterruptHandleInner(pub Arc<InterruptHandleShared>);

impl InterruptHandleInner {
  pub fn new(os: Arc<dyn OsInterrupt>) -> Self {
    InterruptHandleInner(Arc::new(InterruptHandleShared {
      os,
      notified: AtomicBool::new(false),
    }))
  }

  pub fn wake(&self) {
    if !self.0.notified.swap(true, Ordering::AcqRel) {
      self.0.os.signal().expect("failed to wake reactor");
    }
  }

  pub fn reset(&self) {
    self.0.notified.store(false, Ordering::Release);
  }
}

verus! {

#[verifier::external_body]
pub struct InterruptHandle {
  pub(crate) inner: InterruptHandleInner,
}

impl View for InterruptHandle {
  type V = int;

  #[verifier::external_body]
  spec fn view(&self) -> int {
    unimplemented!()
  }
}

impl Clone for InterruptHandle {
  #[verifier::external_body]
  fn clone(&self) -> Self {
    InterruptHandle {
      inner: self.inner.clone(),
    }
  }
}

impl InterruptHandle {
  #[verifier::external_body]
  pub fn wake(&self) {
    self.inner.wake()
  }

  #[verifier::external_body]
  pub fn reset(&self) {
    self.inner.reset()
  }
}

}
