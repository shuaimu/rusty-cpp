#![cfg_attr(verus_keep_ghost, verus::trusted)]

use crate::reactor::Reactor;
use crate::reactor::enter::with_current_reactor;
use crate::readiness;
use crate::types::{Instant, Interest, IoResult, ResourceId, Source, Waker};
use std::cell::Cell;

// Park-cycle clock cache. `get_current_time_action` (the reactor's one real
// clock read per park cycle) stores its observation here; `cached_now()` lets
// the trusted utility glue (Sleep::poll etc.) reuse it instead of issuing a
// clock_gettime per task poll. The cached value equals `wheel.elapsed` and the
// maximum logged timestamp, so a caller guarding `deadline > cached` satisfies
// `register_timer`'s preconditions exactly; a completion decided against the
// cached (older) clock can only be later than one decided against the real
// clock — never early. Thread-local: one reactor per thread.
thread_local! {
  static CACHED_NOW: Cell<Option<u64>> = const { Cell::new(None) };
}

#[inline]
pub fn store_cached_now(now_ms: u64) {
  CACHED_NOW.with(|c| c.set(Some(now_ms)));
}

// Forgets the cached clock when the reactor that published it leaves this
// thread (its ReactorGuard drops), so a later reactor starts from the real
// clock.
pub(crate) fn clear_cached_now() {
  let _ = CACHED_NOW.try_with(|c| c.set(None));
}

#[derive(Copy, Clone)]
pub struct ReactorHandle;

impl ReactorHandle {
  #[inline]
  pub fn new() -> Self {
    ReactorHandle
  }

  /// The reactor's park-cycle clock observation (== `wheel.elapsed`), if the
  /// current thread's reactor has parked at least once. `None` before the
  /// first park (callers fall back to `Instant::now()`).
  #[inline]
  pub fn cached_now() -> Option<Instant> {
    CACHED_NOW.with(|c| c.get()).map(|ms| Instant { inner: ms })
  }

  pub fn register_io_resource(
    &self,
    source: &mut Source,
    interest: Interest,
  ) -> IoResult<ResourceId> {
    with_current_reactor(|reactor| reactor.register_io_resource(source, interest))
      .expect("ReactorHandle used outside reactor context")
  }

  // Also drops the rid's readiness entry: every io resource that called
  // `readiness::init_readiness` deregisters through here exactly once (the
  // TcpStream / TcpListener / UdpSocket Drop impls), and rids are never reused,
  // so the entry could never be read again. Removing it whatever the result
  // keeps the readiness table proportional to live io resources.
  pub fn deregister_io_resource(
    &self,
    resource_id: ResourceId,
    source: &mut Source,
  ) -> IoResult<()> {
    let result = with_current_reactor(|reactor| reactor.deregister_io_resource(resource_id, source))
      .expect("ReactorHandle used outside reactor context");
    readiness::remove_readiness(resource_id);
    result
  }

  pub fn set_waker(&self, resource_id: ResourceId, interest: Interest, waker: Waker) {
    with_current_reactor(|reactor| reactor.set_waker(resource_id, interest, waker))
      .expect("ReactorHandle used outside reactor context")
  }

  #[inline]
  pub fn register_timer(&self, deadline: Instant, waker: Waker) -> IoResult<ResourceId> {
    with_current_reactor(|reactor| {
      if let Some((rid, old_deadline)) = reactor.pending_deregister.take() {
        if old_deadline == deadline.inner && reactor.resources.contains(rid) {
          reactor.resources.replace_timer_waker(rid, waker);
          return IoResult::Ok(ResourceId(rid));
        }
        reactor.wheel.remove(rid);
        reactor.resources.v_remove_timer_slot(rid);
        // [RID-REUSE DISABLED — restore when generational ids are added]
        // reactor.free_rids.push(rid);
      }
      reactor.register_timer(deadline, waker)
    })
      .expect("ReactorHandle used outside reactor context")
  }

  #[inline]
  pub fn deregister_timer(&self, resource_id: ResourceId) {
    with_current_reactor(|reactor| {
      if let Some((old_rid, _)) = reactor.pending_deregister.take() {
        reactor.wheel.remove(old_rid);
        reactor.resources.v_remove_timer_slot(old_rid);
        // [RID-REUSE DISABLED — restore when generational ids are added]
        // reactor.free_rids.push(old_rid);
      }
      if let Some(deadline) = reactor.wheel.get_deadline(resource_id.0) {
        reactor.pending_deregister = Some((resource_id.0, deadline));
      } else {
        reactor.deregister_timer(resource_id);
      }
    })
      .expect("ReactorHandle used outside reactor context")
  }
}

