use vstd::prelude::*;

verus! {

pub type ResourceIdView = nat;
pub type WakerView = int;
pub type InstantView = int;
pub type DurationView = int;
pub type InterestView = (bool, bool);
pub type SourceView = int;
pub type TokenView = nat;

#[derive(PartialEq, Eq)]
pub struct IoEventView {
  pub resource_id: nat,
  pub readable: bool,
  pub writable: bool,
}

#[derive(PartialEq, Eq)]
pub enum IoResultView<T> {
  Ok(T),
  Err(int),
}

impl<T> IoResultView<T> {
  pub open spec fn to_unit(self) -> IoResultView<()> {
    match self {
      IoResultView::Ok(_) => IoResultView::Ok(()),
      IoResultView::Err(e) => IoResultView::Err(e),
    }
  }
}

}
