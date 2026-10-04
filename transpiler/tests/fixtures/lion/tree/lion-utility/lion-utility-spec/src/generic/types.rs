use vstd::prelude::*;
// Reuse the shared view types from the reactor spec — utilities and reactor
// agree on the data carried across the call boundary.
pub use crate::view_types::{
  ResourceIdView, WakerView, InstantView, DurationView, InterestView,
  SourceView, IoResultView,
};

verus! {

// `TickResult<R>` is utility-level — distinct from `executor::PollResult`
// (Ready / Pending / Invalid). Utility code never produces `Invalid` (an
// executor-internal slab-miss outcome) and adds `Ongoing` for sync
// state-mutating method calls that don't yield.
//
// `R` is the utility-specific return value type — usually a sum over the
// return values of all the utility's methods. `Pending` carries no payload;
// `Finished` / `Ongoing` carry a value of type `R`.
//
// Alignment-boundary mapping:
//   executor::PollResult::Ready    <-> utility::TickResult::Finished(_)
//   executor::PollResult::Pending  <-> utility::TickResult::Pending
//   executor::PollResult::Invalid  has no utility-side counterpart
//   utility::TickResult::Ongoing   has no executor-side counterpart (sync)

pub enum TickResult<R> {
  Pending,
  Finished(R),
  Ongoing(R),
}

}
