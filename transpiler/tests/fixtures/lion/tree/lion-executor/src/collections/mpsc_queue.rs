#![cfg_attr(verus_keep_ghost, verus::trusted)]

use std::sync::mpsc::{channel, Receiver, Sender};
use vstd::prelude::*;

verus! {

#[verifier::external_body]
#[verifier::reject_recursive_types(T)]
pub struct MpscSender<T> {
  sender: Sender<T>,
}

#[verifier::external_body]
#[verifier::reject_recursive_types(T)]
pub struct MpscReceiver<T> {
  receiver: Receiver<T>,
}

} // end verus!

pub fn mpsc_queue<T>() -> (MpscSender<T>, MpscReceiver<T>) {
  let (sender, receiver) = channel();
  (
  MpscSender { sender },
  MpscReceiver { receiver },
  )
}

impl<T> MpscSender<T> {
  pub fn send(&self, item: T) -> bool {
  self.sender.send(item).is_ok()
  }
}

impl<T> Clone for MpscSender<T> {
  fn clone(&self) -> Self {
  Self {
    sender: self.sender.clone(),
  }
  }
}

impl<T> MpscReceiver<T> {
  pub fn try_recv(&self) -> Option<T> {
  self.receiver.try_recv().ok()
  }
}
