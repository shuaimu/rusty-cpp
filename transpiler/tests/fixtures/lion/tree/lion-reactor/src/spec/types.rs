// The reactor-side view types now live in the shared spec crates
// (lion-reactor-spec re-exports lion-utility-spec's view_types).
#[allow(unused_imports)]
pub use lion_reactor_spec::types::*;

use vstd::prelude::*;

verus! {

// ResourceSlotView links concrete slab state to the ghost log; it is
// impl-linking material and stays local to lion-reactor.
#[derive(PartialEq, Eq)]
pub enum ResourceSlotView {
  Timer { entry: (InstantView, nat, int), waker: WakerView },
  Io { read_waker: Option<WakerView>, write_waker: Option<WakerView> },
}

impl ResourceSlotView {
  pub open spec fn is_timer(&self) -> bool {
    matches!(self, ResourceSlotView::Timer { .. })
  }

  pub open spec fn is_io(&self) -> bool {
    matches!(self, ResourceSlotView::Io { .. })
  }

  pub open spec fn get_timer_entry(&self) -> (InstantView, nat, int) {
    match self {
      ResourceSlotView::Timer { entry, .. } => *entry,
      _ => arbitrary(),
    }
  }

  pub open spec fn get_timer_waker(&self) -> WakerView {
    match self {
      ResourceSlotView::Timer { waker, .. } => *waker,
      _ => arbitrary(),
    }
  }

  pub open spec fn get_read_waker(&self) -> Option<WakerView> {
    match self {
      ResourceSlotView::Io { read_waker, .. } => *read_waker,
      _ => arbitrary(),
    }
  }

  pub open spec fn get_write_waker(&self) -> Option<WakerView> {
    match self {
      ResourceSlotView::Io { write_waker, .. } => *write_waker,
      _ => arbitrary(),
    }
  }
}

}
