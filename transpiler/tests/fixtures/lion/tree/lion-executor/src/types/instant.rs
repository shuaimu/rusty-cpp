use lion_reactor::Instant as ReactorInstant;
use super::Duration;
use vstd::prelude::*;

verus! {

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Instant {
  inner: ReactorInstant,
}

impl View for Instant {
  type V = nat;

  #[verifier::external_body]
  spec fn view(&self) -> nat {
  unimplemented!()
  }
}

impl Instant {
  #[verifier::external_body]
  pub exec fn now() -> (result: Self)
  {
  Instant { inner: ReactorInstant::now() }
  }

  #[verifier::external_body]
  pub exec fn elapsed(&self) -> (result: Duration)
  {
  Duration::from(self.inner.elapsed())
  }

  #[verifier::external_body]
  pub exec fn less_than(&self, other: &Instant) -> (result: bool)
  {
  self.inner.inner < other.inner.inner
  }

  #[verifier::external_body]
  pub exec fn duration_since(&self, earlier: &Instant) -> (result: Duration)
  {
  Duration::from_millis(self.inner.inner.saturating_sub(earlier.inner.inner))
  }
}

impl From<ReactorInstant> for Instant {
  #[verifier::external_body]
  fn from(i: ReactorInstant) -> (result: Self)
  {
  Instant { inner: i }
  }
}

}
