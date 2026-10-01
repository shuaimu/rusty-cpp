use lion_reactor::{Reactor as LionReactor, ReactorGuard as LionReactorGuard};
use super::{Duration, Instant};
use vstd::prelude::*;

verus! {

#[verifier::external_body]
pub struct Reactor {
  inner: LionReactor,
  // Upper bound, in ms, on the reactor park when the executor has no ready
  // work (read only by `Executor::park_action`). `Runtime` sets it before every
  // tick: IDLE_PARK_MS for block_on and tick, the caller's bound for
  // tick_with_timeout.
  idle_park_ms: u64,
}

#[verifier::external_body]
pub struct ReactorGuard {
  inner: LionReactorGuard,
}

impl Reactor {
  #[verifier::external_body]
  pub fn new(reactor: LionReactor) -> (result: Self)
  {
  Reactor { inner: reactor, idle_park_ms: IDLE_PARK_MS }
  }
}

} // end verus!

// The executor's idle park: long enough to cost nothing while idle, short
// enough to bound what an unsignalled wake would wait.
pub(crate) const IDLE_PARK_MS: u64 = 100;

impl Reactor {
  pub(crate) fn idle_park_ms(&self) -> u64 {
    self.idle_park_ms
  }

  pub(crate) fn set_idle_park_ms(&mut self, ms: u64) {
    self.idle_park_ms = ms;
  }

  pub(crate) fn enter(&mut self) -> ReactorGuard {
  ReactorGuard {
    inner: self.inner.enter(),
  }
  }

  pub(crate) fn park(&mut self, timeout: Option<Duration>) {
  self.inner.park(timeout.map(|d| d.into_reactor()));
  }

  pub(crate) fn next_deadline(&mut self) -> Option<Instant> {
  self.inner.next_deadline().map(Instant::from)
  }

  #[inline]
  pub(crate) fn flush_pending_deregister(&mut self) {
  self.inner.flush_pending_deregister();
  }
}
