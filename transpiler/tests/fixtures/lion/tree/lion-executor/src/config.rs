use vstd::prelude::*;

verus! {

#[derive(Debug, Clone, Copy)]
pub struct RuntimeConfig {
  pub event_interval: usize,
}

} // end verus!

impl Default for RuntimeConfig {
  fn default() -> Self {
  Self {
    event_interval: 128,
  }
  }
}
