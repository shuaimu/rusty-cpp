// Trusted glue: readiness of a raw fd for tasks, over the current reactor.
// Plain Rust that Verus never sees; it reaches the verified reactor only
// through ReactorHandle (register_io_resource / set_waker /
// deregister_io_resource) and the trusted readiness table.
#![cfg_attr(verus_keep_ghost, verus::trusted)]

//! [`AsyncFd`]: a task waits for a raw fd to become readable or writable.
//!
//! This is the one place the reactor's edge-triggered readiness protocol is
//! written down and implemented for general use. The state is a flag per io
//! resource and direction (`crate::readiness`) plus, in the reactor, one waker
//! per direction. The protocol keeps five invariants:
//!
//! 1. **Only a park sets a flag.** The reactor's park (`process_io_events`)
//!    sets a direction's flag, on this thread, when the backend reports an edge
//!    (or an error or hang-up) for it. Nothing sets a flag between two parks,
//!    so a task poll is atomic with respect to readiness.
//! 2. **Only an observed `WouldBlock` clears a flag.** [`AsyncFdReadyGuard::try_io`]
//!    clears the direction's flag right after the operation returned
//!    `WouldBlock`, in the same poll. By (1) no edge can have arrived between
//!    that `WouldBlock` and the clear, so the clear consumes exactly the
//!    readiness the `WouldBlock` disproved, and the next transition to ready is
//!    a new edge (the backend contract) that sets the flag at the next park.
//!    Clearing on any other evidence (a successful operation, a short read on a
//!    datagram fd, "not connected yet" before the connect edge was consumed) can
//!    consume the flag of the last edge that will ever come: the lost wakeup of
//!    HANG_FIXING_STORY.md story 1, where the connect path cleared the writable
//!    flag after the connection-complete edge and the first write then waited
//!    forever. This type has no other way to clear a flag.
//! 3. **Register the waker, then re-check, then return `Pending`.** A poll that
//!    finds the flag clear stores `cx.waker()` in the reactor for the
//!    direction, reads the flag again, and returns `Pending` only if it is
//!    still clear. By (1) the re-read cannot differ on this single-threaded
//!    reactor; it stays so that the protocol does not rest on that fact
//!    (register-then-check is the order that survives a flag set concurrently).
//! 4. **The caller's waker, not the task's.** The waker stored is
//!    `cx.waker()`, so combinators that give a sub-future its own waker (select,
//!    join, FuturesUnordered) are woken for the right sub-future, and a wait
//!    costs no allocation. The reactor keeps a direction's waker until the next
//!    registration for that direction and wakes it on every edge, so a stale
//!    waker costs only a spurious wake. There is one waiter per direction: a
//!    second task waiting on the same direction of the same `AsyncFd` replaces
//!    the first one's waker (a reader task and a writer task can share one
//!    `AsyncFd`, e.g. through an `Rc`).
//! 5. **Ready from the start.** Registration sets both flags (the fd is
//!    optimistically ready), so the first operation discovers the real state
//!    and the backend need not report an fd that was already ready when it was
//!    registered.
//!
//! A caller must act on a `Ready` result: if it gets a guard and returns
//! `Pending` without calling `try_io` (or `poll_*_io`) until `WouldBlock`, no
//! waker is registered and nothing will wake it.
//!
//! `AsyncFd` is `!Send`: the flags, the waker slots and the reactor are
//! per-thread, and (1) holds only on the reactor's own thread.

use crate::handle::ReactorHandle;
use crate::os::RawFd;
use crate::reactor::enter::current_reactor_epoch;
use crate::readiness;
use crate::types::{Interest, IoResult, ResourceId, Source, Waker};
use std::future::poll_fn;
use std::io;
use std::marker::PhantomData;
use std::task::{Context, Poll};

/// A raw file descriptor registered with the current thread's reactor for
/// read and write readiness.
///
/// The fd must be in non-blocking mode (`AsyncFd` never touches it: it neither
/// sets flags nor reads, writes or closes it) and must stay open until the
/// `AsyncFd` is dropped; dropping it deregisters the fd.
pub struct AsyncFd {
  fd: RawFd,
  rid: ResourceId,
  // The reactor this fd is registered with (current_reactor_epoch at `new`).
  reactor: u64,
  _not_send: PhantomData<*const ()>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Direction {
  Read,
  Write,
}

impl Direction {
  fn interest(self) -> Interest {
    match self {
      Direction::Read => Interest::READABLE,
      Direction::Write => Interest::WRITABLE,
    }
  }

  fn is_ready(self, rid: ResourceId) -> bool {
    match self {
      Direction::Read => readiness::is_readable(rid),
      Direction::Write => readiness::is_writable(rid),
    }
  }

  // Invariant 2: called only from `try_io`, right after a WouldBlock.
  fn clear(self, rid: ResourceId) {
    match self {
      Direction::Read => readiness::clear_readable(rid),
      Direction::Write => readiness::clear_writable(rid),
    }
  }
}

/// Proof that the fd was ready in one direction at the last poll. Its only
/// operation, [`try_io`](AsyncFdReadyGuard::try_io), runs one non-blocking
/// operation and consumes the readiness if and only if that operation returned
/// `WouldBlock`.
#[must_use = "a ready guard that is dropped without try_io leaves the task with no waker registered"]
pub struct AsyncFdReadyGuard<'a> {
  fd: &'a AsyncFd,
  dir: Direction,
}

impl<'a> AsyncFdReadyGuard<'a> {
  /// Runs `f` on the fd. If it fails with `WouldBlock`, the readiness for this
  /// direction is cleared and `WouldBlock` is returned: poll for readiness
  /// again to wait for the next edge. Any other outcome, success or error,
  /// leaves the readiness set.
  pub fn try_io<R>(self, f: impl FnOnce(RawFd) -> io::Result<R>) -> io::Result<R> {
    let result = f(self.fd.fd);
    if let Err(e) = &result {
      if e.kind() == io::ErrorKind::WouldBlock {
        self.dir.clear(self.fd.rid);
      }
    }
    result
  }

  pub fn fd(&self) -> RawFd {
    self.fd.fd
  }
}

impl AsyncFd {
  /// Registers `fd` with the current thread's reactor for read and write
  /// readiness. Fails if no reactor is entered on this thread (no Lion runtime)
  /// or if the backend refuses the registration.
  pub fn new(fd: RawFd) -> io::Result<AsyncFd> {
    let reactor = current_reactor_epoch().ok_or_else(|| {
      io::Error::new(io::ErrorKind::Other, "AsyncFd::new called on a thread with no Lion reactor")
    })?;
    let mut source = Source::new(fd);
    match ReactorHandle::new().register_io_resource(&mut source, Interest::READABLE_WRITABLE) {
      IoResult::Ok(rid) => {
        // Invariant 5.
        readiness::init_readiness(rid);
        Ok(AsyncFd { fd, rid, reactor, _not_send: PhantomData })
      }
      IoResult::Err(e) => Err(e.into_io_error()),
    }
  }

  /// The registered fd.
  pub fn as_raw_fd(&self) -> RawFd {
    self.fd
  }

  /// Ready when the fd may be readable; otherwise registers `cx.waker()` for
  /// the next read edge and returns `Pending`. Fails if polled on a thread
  /// whose current reactor is not the one the fd was registered with.
  pub fn poll_read_ready(&self, cx: &mut Context<'_>) -> Poll<io::Result<AsyncFdReadyGuard<'_>>> {
    self.poll_ready(Direction::Read, cx)
  }

  /// As [`poll_read_ready`](AsyncFd::poll_read_ready), for writability.
  pub fn poll_write_ready(&self, cx: &mut Context<'_>) -> Poll<io::Result<AsyncFdReadyGuard<'_>>> {
    self.poll_ready(Direction::Write, cx)
  }

  /// Runs the read operation `f` when the fd is readable, until it does not
  /// return `WouldBlock`; returns `Pending` (with `cx.waker()` registered) when
  /// it would block.
  pub fn poll_read_io<R>(
    &self,
    cx: &mut Context<'_>,
    f: impl FnMut(RawFd) -> io::Result<R>,
  ) -> Poll<io::Result<R>> {
    self.poll_io(Direction::Read, cx, f)
  }

  /// As [`poll_read_io`](AsyncFd::poll_read_io), for a write operation.
  pub fn poll_write_io<R>(
    &self,
    cx: &mut Context<'_>,
    f: impl FnMut(RawFd) -> io::Result<R>,
  ) -> Poll<io::Result<R>> {
    self.poll_io(Direction::Write, cx, f)
  }

  /// Waits until the fd may be readable.
  pub async fn readable(&self) -> io::Result<AsyncFdReadyGuard<'_>> {
    poll_fn(|cx| self.poll_read_ready(cx)).await
  }

  /// Waits until the fd may be writable.
  pub async fn writable(&self) -> io::Result<AsyncFdReadyGuard<'_>> {
    poll_fn(|cx| self.poll_write_ready(cx)).await
  }

  /// Runs the read operation `f` once the fd is readable, retrying after each
  /// `WouldBlock`.
  pub async fn read_io<R>(&self, mut f: impl FnMut(RawFd) -> io::Result<R>) -> io::Result<R> {
    poll_fn(|cx| self.poll_read_io(cx, &mut f)).await
  }

  /// Runs the write operation `f` once the fd is writable, retrying after each
  /// `WouldBlock`.
  pub async fn write_io<R>(&self, mut f: impl FnMut(RawFd) -> io::Result<R>) -> io::Result<R> {
    poll_fn(|cx| self.poll_write_io(cx, &mut f)).await
  }

  fn check_reactor(&self) -> io::Result<()> {
    if current_reactor_epoch() == Some(self.reactor) {
      Ok(())
    } else {
      Err(io::Error::new(
        io::ErrorKind::Other,
        "AsyncFd polled outside the Lion reactor it was registered with",
      ))
    }
  }

  fn poll_ready(&self, dir: Direction, cx: &mut Context<'_>) -> Poll<io::Result<AsyncFdReadyGuard<'_>>> {
    if let Err(e) = self.check_reactor() {
      return Poll::Ready(Err(e));
    }
    if dir.is_ready(self.rid) {
      return Poll::Ready(Ok(AsyncFdReadyGuard { fd: self, dir }));
    }
    // Invariants 3 and 4: the caller's waker first, then the re-check.
    ReactorHandle::new().set_waker(self.rid, dir.interest(), Waker::from_std(cx.waker().clone()));
    if dir.is_ready(self.rid) {
      return Poll::Ready(Ok(AsyncFdReadyGuard { fd: self, dir }));
    }
    Poll::Pending
  }

  fn poll_io<R>(
    &self,
    dir: Direction,
    cx: &mut Context<'_>,
    mut f: impl FnMut(RawFd) -> io::Result<R>,
  ) -> Poll<io::Result<R>> {
    loop {
      let guard = match self.poll_ready(dir, cx) {
        Poll::Ready(Ok(guard)) => guard,
        Poll::Ready(Err(e)) => return Poll::Ready(Err(e)),
        Poll::Pending => return Poll::Pending,
      };
      match guard.try_io(&mut f) {
        // Readiness consumed (invariant 2): the next round registers the
        // waker and returns Pending, since nothing can set the flag within
        // this poll (invariant 1).
        Err(e) if e.kind() == io::ErrorKind::WouldBlock => continue,
        result => return Poll::Ready(result),
      }
    }
  }
}

impl std::os::fd::AsRawFd for AsyncFd {
  fn as_raw_fd(&self) -> RawFd {
    self.fd
  }
}

impl std::fmt::Debug for AsyncFd {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("AsyncFd").field("fd", &self.fd).field("rid", &self.rid.0).finish()
  }
}

// Deregisters from the reactor the fd was registered with, which also drops
// its readiness entry. If that reactor has left the thread (its runtime was
// dropped first), there is nothing to undo: its readiness table was cleared
// with it, and a successor reactor, which numbers its rids from 1 again, must
// not be touched.
impl Drop for AsyncFd {
  fn drop(&mut self) {
    if current_reactor_epoch() == Some(self.reactor) {
      let mut source = Source::new(self.fd);
      let _ = ReactorHandle::new().deregister_io_resource(self.rid, &mut source);
    }
  }
}
