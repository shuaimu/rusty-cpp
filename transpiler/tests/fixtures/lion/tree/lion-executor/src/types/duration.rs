use lion_reactor::Duration as ReactorDuration;
use vstd::prelude::*;

verus! {

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Duration {
  inner: ReactorDuration,
}

impl View for Duration {
  type V = nat;

  #[verifier::external_body]
  spec fn view(&self) -> nat {
  unimplemented!()
  }
}

impl Duration {
  #[verifier::external_body]
  pub exec fn zero() -> (result: Self)
  {
  Duration { inner: ReactorDuration::from_millis(0) }
  }

  #[verifier::external_body]
  pub exec fn from_millis(millis: u64) -> (result: Self)
  {
  Duration { inner: ReactorDuration::from_millis(millis) }
  }

  #[verifier::external_body]
  pub exec fn from_secs(secs: u64) -> (result: Self)
  {
  Duration { inner: ReactorDuration::from_secs(secs) }
  }

  #[verifier::external_body]
  pub exec fn as_millis(&self) -> (result: u64)
  {
  self.inner.as_millis()
  }

  #[verifier::external_body]
  pub exec fn into_reactor(self) -> (result: ReactorDuration)
  {
  self.inner
  }
}

impl From<ReactorDuration> for Duration {
  #[verifier::external_body]
  fn from(d: ReactorDuration) -> (result: Self)
  {
  Duration { inner: d }
  }
}

impl From<Duration> for ReactorDuration {
  #[verifier::external_body]
  fn from(d: Duration) -> (result: Self)
  {
  d.inner
  }
}

}
