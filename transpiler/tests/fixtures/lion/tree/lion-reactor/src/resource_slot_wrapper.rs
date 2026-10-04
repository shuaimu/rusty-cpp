#![cfg_attr(verus_keep_ghost, verus::trusted)]

use crate::resource_slot::ResourceSlot;
use crate::spec::types::ResourceSlotView;
use crate::types::{TimerEntry, Waker};
use vstd::prelude::*;

verus! {

#[verifier::external_body]
pub struct ResourceSlotWrapper {
  pub(crate) inner: ResourceSlot,
}

impl View for ResourceSlotWrapper {
  type V = ResourceSlotView;

  #[verifier::external_body]
  open spec fn view(&self) -> ResourceSlotView {
    unimplemented!()
  }
}

impl ResourceSlotWrapper {
  #[verifier::external_body]
  pub exec fn new_timer(entry: TimerEntry, waker: Waker) -> (result: Self)
    ensures result@ == (ResourceSlotView::Timer { entry: entry@, waker: waker@ }),
  {
    ResourceSlotWrapper {
      inner: ResourceSlot::Timer {
        entry,
        waker,
      },
    }
  }

  #[verifier::external_body]
  pub exec fn new_io() -> (result: Self)
    ensures result@ == (ResourceSlotView::Io { read_waker: None, write_waker: None }),
  {
    ResourceSlotWrapper {
      inner: ResourceSlot::Io {
        read_waker: None,
        write_waker: None,
      },
    }
  }

  #[verifier::external_body]
  pub exec fn is_timer(&self) -> (result: bool)
    ensures result == self@.is_timer(),
  {
    matches!(self.inner, ResourceSlot::Timer { .. })
  }

  #[verifier::external_body]
  pub exec fn is_io(&self) -> (result: bool)
    ensures result == self@.is_io(),
  {
    matches!(self.inner, ResourceSlot::Io { .. })
  }

  #[verifier::external_body]
  pub exec fn with_read_waker(self, waker: Waker) -> (result: Self)
    requires self@.is_io(),
    ensures result@ == (ResourceSlotView::Io {
      read_waker: Some(waker@),
      write_waker: self@.get_write_waker(),
    }),
  {
    match self.inner {
      ResourceSlot::Io { write_waker, .. } => ResourceSlotWrapper {
        inner: ResourceSlot::Io { read_waker: Some(waker), write_waker },
      },
      _ => unreachable!(),
    }
  }

  #[verifier::external_body]
  pub exec fn with_write_waker(self, waker: Waker) -> (result: Self)
    requires self@.is_io(),
    ensures result@ == (ResourceSlotView::Io {
      read_waker: self@.get_read_waker(),
      write_waker: Some(waker@),
    }),
  {
    match self.inner {
      ResourceSlot::Io { read_waker, .. } => ResourceSlotWrapper {
        inner: ResourceSlot::Io { read_waker, write_waker: Some(waker) },
      },
      _ => unreachable!(),
    }
  }

  #[verifier::external_body]
  pub exec fn clone_timer_waker(&self) -> (result: Waker)
    requires self@.is_timer(),
    ensures result@ == self@.get_timer_waker(),
  {
    match &self.inner {
      ResourceSlot::Timer { waker, .. } => waker.clone(),
      _ => unreachable!(),
    }
  }

  #[verifier::external_body]
  pub exec fn clone_read_waker(&self) -> (result: Option<Waker>)
    requires self@.is_io(),
    ensures
      result.is_some() <==> self@.get_read_waker().is_some(),
      result.is_some() ==> result.unwrap()@ == self@.get_read_waker().unwrap(),
  {
    match &self.inner {
      ResourceSlot::Io { read_waker, .. } => read_waker.as_ref().map(|w| w.clone()),
      _ => unreachable!(),
    }
  }

  #[verifier::external_body]
  pub exec fn clone_write_waker(&self) -> (result: Option<Waker>)
    requires self@.is_io(),
    ensures
      result.is_some() <==> self@.get_write_waker().is_some(),
      result.is_some() ==> result.unwrap()@ == self@.get_write_waker().unwrap(),
  {
    match &self.inner {
      ResourceSlot::Io { write_waker, .. } => write_waker.as_ref().map(|w| w.clone()),
      _ => unreachable!(),
    }
  }
}

}
