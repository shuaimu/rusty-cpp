use crate::types::ResourceId;
use std::cell::RefCell;
use std::collections::HashMap;

const READABLE: u8 = 0x01;
const WRITABLE: u8 = 0x02;

// Per-resource readiness flags, keyed by rid. An entry exists only between
// `init_readiness` (after a successful io registration) and `remove_readiness`
// (on deregistration) or `clear_all` (when the reactor leaves the thread), so
// the table holds one byte per LIVE io resource of the current reactor. The
// protocol these flags implement is written down in async_fd.rs. Rids
// are never reused, so an entry indexed by rid in a window would instead grow
// with every rid ever allocated. `mark_*` / `clear_*` touch existing entries
// only: a stale event for a deregistered rid is a no-op, and a rid that was
// never initialised (a timer) reads as not ready.
thread_local! {
  static IO_READINESS: RefCell<HashMap<u64, u8>> = RefCell::new(HashMap::new());
}

pub fn init_readiness(resource_id: ResourceId) {
  IO_READINESS.with(|r| {
    r.borrow_mut().insert(resource_id.0, READABLE | WRITABLE);
  });
}

// Drops every entry: the reactor that owned these rids has left the thread
// (ReactorGuard's drop).
pub(crate) fn clear_all() {
  let _ = IO_READINESS.try_with(|r| {
    if let Ok(mut r) = r.try_borrow_mut() {
      r.clear();
    }
  });
}

/// Number of live readiness entries on this thread (one per registered io
/// resource that initialised its readiness). For tests of resource cleanup.
pub fn live_entries() -> usize {
  IO_READINESS.with(|r| r.borrow().len())
}

pub fn remove_readiness(resource_id: ResourceId) {
  IO_READINESS.with(|r| {
    r.borrow_mut().remove(&resource_id.0);
  });
}

pub fn mark_readable(resource_id: ResourceId) {
  IO_READINESS.with(|r| {
    if let Some(f) = r.borrow_mut().get_mut(&resource_id.0) {
      *f |= READABLE;
    }
  });
}

pub fn mark_writable(resource_id: ResourceId) {
  IO_READINESS.with(|r| {
    if let Some(f) = r.borrow_mut().get_mut(&resource_id.0) {
      *f |= WRITABLE;
    }
  });
}

pub fn is_readable(resource_id: ResourceId) -> bool {
  IO_READINESS.with(|r| {
    r.borrow().get(&resource_id.0).is_some_and(|f| (f & READABLE) != 0)
  })
}

pub fn is_writable(resource_id: ResourceId) -> bool {
  IO_READINESS.with(|r| {
    r.borrow().get(&resource_id.0).is_some_and(|f| (f & WRITABLE) != 0)
  })
}

pub fn clear_readable(resource_id: ResourceId) {
  IO_READINESS.with(|r| {
    if let Some(f) = r.borrow_mut().get_mut(&resource_id.0) {
      *f &= !READABLE;
    }
  });
}

pub fn clear_writable(resource_id: ResourceId) {
  IO_READINESS.with(|r| {
    if let Some(f) = r.borrow_mut().get_mut(&resource_id.0) {
      *f &= !WRITABLE;
    }
  });
}
