#![cfg_attr(verus_keep_ghost, verus::trusted)]

use crate::types::{TimerEntry, Waker};

pub enum ResourceSlot {
  Timer {
    entry: TimerEntry,
    waker: Waker,
  },
  Io {
    read_waker: Option<Waker>,
    write_waker: Option<Waker>,
  },
}
