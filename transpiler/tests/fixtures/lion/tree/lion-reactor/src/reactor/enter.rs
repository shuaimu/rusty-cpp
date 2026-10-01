#![cfg_attr(verus_keep_ghost, verus::trusted)]

use crate::reactor::{Reactor, ReactorGuard};
use std::cell::Cell;

thread_local! {
  static CURRENT_REACTOR: Cell<Option<*mut Reactor>> = const { Cell::new(None) };
  // Identity of the entered reactor that survives address reuse: a fresh
  // number per reactor entered on this thread (0: none entered). An io
  // resource records it at registration, so it never acts on a successor
  // reactor that happens to live at the same address with its own rids.
  static CURRENT_EPOCH: Cell<u64> = const { Cell::new(0) };
  static NEXT_EPOCH: Cell<u64> = const { Cell::new(1) };
}

impl Reactor {
  // Installs this reactor as the thread's current one. Entering while another
  // reactor is current replaces it; callers that must not (lion-executor's
  // Runtime) check `is_entered_on_current_thread` first.
  pub fn enter(&mut self) -> ReactorGuard {
    let ptr = self as *mut Reactor;
    let already_current = CURRENT_REACTOR.with(|r| r.get() == Some(ptr));
    if !already_current {
      let epoch = NEXT_EPOCH.with(|n| {
        let e = n.get();
        n.set(e + 1);
        e
      });
      CURRENT_EPOCH.with(|c| c.set(epoch));
    }
    CURRENT_REACTOR.with(|r| {
      r.set(Some(ptr));
    });
    ReactorGuard { reactor: ptr as usize }
  }

  /// Whether a reactor is entered on this thread.
  pub fn is_entered_on_current_thread() -> bool {
    CURRENT_REACTOR.try_with(|r| r.get().is_some()).unwrap_or(false)
  }
}

// Clears only what this guard installed: if a later `enter` replaced this
// reactor, the newer entry stays. With the reactor go its thread-local
// companions: the cached clock and the io readiness flags, whose rids belong to
// this reactor alone (the next one numbers its rids from 1 again).
impl Drop for ReactorGuard {
  fn drop(&mut self) {
    let _ = CURRENT_REACTOR.try_with(|r| {
      if r.get().map(|p| p as usize) == Some(self.reactor) {
        r.set(None);
        let _ = CURRENT_EPOCH.try_with(|c| c.set(0));
        crate::handle::clear_cached_now();
        crate::readiness::clear_all();
      }
    });
  }
}

// The identity (see CURRENT_EPOCH) of the reactor entered on this thread, if
// any.
pub(crate) fn current_reactor_epoch() -> Option<u64> {
  let entered = CURRENT_REACTOR.try_with(|r| r.get().is_some()).unwrap_or(false);
  if !entered {
    return None;
  }
  CURRENT_EPOCH.try_with(|c| c.get()).ok().filter(|e| *e != 0)
}

pub(crate) fn with_current_reactor<F, R>(f: F) -> Option<R>
where
  F: FnOnce(&mut Reactor) -> R,
{
  CURRENT_REACTOR.with(|r| {
    let ptr = r.get();
    ptr.and_then(|ptr| unsafe { ptr.as_mut().map(f) })
  })
}
