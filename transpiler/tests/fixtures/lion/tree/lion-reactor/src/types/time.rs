use vstd::prelude::*;
use crate::spec::types::{InstantView, DurationView};

verus! {

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Duration {
  pub inner: u64,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Instant {
  pub inner: u64,
}

impl View for Duration {
  type V = DurationView;

  open spec fn view(&self) -> DurationView {
    self.inner as int
  }
}

impl DeepView for Duration {
  type V = DurationView;

  open spec fn deep_view(&self) -> DurationView {
    self.inner as int
  }
}

impl View for Instant {
  type V = InstantView;

  open spec fn view(&self) -> InstantView {
    self.inner as int
  }
}

impl DeepView for Instant {
  type V = InstantView;

  open spec fn deep_view(&self) -> InstantView {
    self.inner as int
  }
}

impl Duration {
  pub fn from_millis(millis: u64) -> Self {
    Duration { inner: millis }
  }

  // Saturating: secs beyond u64::MAX/1000 clamp to u64::MAX ms (same saturating
  // style as Instant::add/sub). Verified — the multiplication cannot overflow.
  pub fn from_secs(secs: u64) -> Self {
    if secs > u64::MAX / 1000 {
      Duration { inner: u64::MAX }
    } else {
      Duration { inner: secs * 1000 }
    }
  }

  pub fn as_millis(&self) -> u64 {
    self.inner
  }

  #[verifier::external_body]
  pub fn from_std(std_duration: std::time::Duration) -> Self {
    Duration { inner: std_duration.as_millis() as u64 }
  }
}

impl Instant {
  #[verifier::external_body]
  pub fn now() -> Self {
    use std::sync::OnceLock;
    static START: OnceLock<std::time::Instant> = OnceLock::new();
    let start = *START.get_or_init(std::time::Instant::now);
    Instant { inner: start.elapsed().as_millis() as u64 }
  }

  #[verifier::external_body]
  pub fn elapsed(&self) -> Duration {
    let now = Self::now();
    Duration { inner: now.inner.saturating_sub(self.inner) }
  }
}

impl std::ops::Add<Duration> for Instant {
  type Output = Instant;

  #[verifier::external_body]
  fn add(self, duration: Duration) -> Instant {
    Instant { inner: self.inner.saturating_add(duration.inner) }
  }
}

impl std::ops::Sub<Duration> for Instant {
  type Output = Instant;

  #[verifier::external_body]
  fn sub(self, duration: Duration) -> Instant {
    Instant { inner: self.inner.saturating_sub(duration.inner) }
  }
}

}

impl From<std::time::Duration> for Duration {
  fn from(d: std::time::Duration) -> Self {
    Duration { inner: d.as_millis() as u64 }
  }
}

impl From<Duration> for std::time::Duration {
  fn from(d: Duration) -> Self {
    std::time::Duration::from_millis(d.inner)
  }
}

impl std::ops::Add<std::time::Duration> for Instant {
  type Output = Instant;

  fn add(self, duration: std::time::Duration) -> Instant {
    Instant { inner: self.inner.saturating_add(duration.as_millis() as u64) }
  }
}

impl std::ops::Sub<std::time::Duration> for Instant {
  type Output = Instant;

  fn sub(self, duration: std::time::Duration) -> Instant {
    Instant { inner: self.inner.saturating_sub(duration.as_millis() as u64) }
  }
}

impl std::ops::AddAssign<std::time::Duration> for Instant {
  fn add_assign(&mut self, duration: std::time::Duration) {
    self.inner = self.inner.saturating_add(duration.as_millis() as u64);
  }
}
