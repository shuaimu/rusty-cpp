// Trusted glue: the OS seam of the reactor. Plain Rust that Verus never sees;
// the verified reactor reaches it only through the trusted leaves in
// reactor/ext.rs (register_io_source_action, deregister_io_source_action,
// poll_events_action) and reactor/new.rs (mio_setup, backend_setup).
#![cfg_attr(verus_keep_ghost, verus::trusted)]

//! The operating-system seam of the reactor.
//!
//! Everything the reactor needs from the OS goes through one object-safe trait,
//! [`OsBackend`]: register, reregister and deregister a raw file descriptor
//! with an interest, wait for readiness events with a timeout, and hand out a
//! cross-thread [`OsInterrupt`] that cuts a wait short. The reactor owns its
//! backend as a `Box<dyn OsBackend>` (one dynamic call per park, one per
//! registration). [`MioBackend`] (feature `mio`, on by default) is Lion's own
//! backend; an embedder with its own event loop plugs in its own backend with
//! `Reactor::with_backend` (lion-executor: `RuntimeBuilder::os_backend`), and
//! then needs no mio at all.
//!
//! # The contract a backend must meet
//!
//! The reactor keeps a readiness flag per io resource and direction (read,
//! write). A flag is set when a wait reports an event for the resource and is
//! cleared only by the resource's owner after an operation returned
//! `WouldBlock` (see [`crate::async_fd`]). The reactor therefore needs
//! **edge-triggered** reports, exactly epoll's `EPOLLET` as mio uses it:
//!
//! * **Registration.** `register(fd, token, interest)` starts reporting `fd`
//!   under `token`, for readability if `interest.readable`, for writability if
//!   `interest.writable` (an interest with neither is treated as readable).
//!   Tokens are opaque to the backend and must be returned verbatim in events.
//!   The reactor never registers token 0 (a backend may reserve it for its
//!   interrupt) and never registers one fd twice without deregistering it.
//!   An fd that is already ready when registered need not be reported: the
//!   reactor starts every resource as ready in both directions. `reregister`
//!   replaces the token and interest of a registered fd (the reactor itself
//!   does not call it today). `deregister(fd)` stops all reports for `fd`,
//!   including events not yet returned by a wait; it is called before the fd
//!   is closed. Errors are returned as `io::Error` and fail the registration.
//! * **Edge-triggered events.** After an event for (token, direction) has been
//!   returned, the backend must report that direction again whenever the fd
//!   becomes ready anew: new data arrives (even if unread data remains), buffer
//!   space frees up, a connection completes, the peer closes, an error occurs.
//!   It must never swallow such a transition. Reporting more often (spurious
//!   events, or level-triggered reports) is safe but costs wakeups: a
//!   level-triggered backend re-reports a ready fd on every wait, so a task that
//!   leaves data unread keeps the loop spinning.
//! * **Event flags** ([`OsEvent`]), with mio's meaning on Linux: `readable` =
//!   `EPOLLIN|EPOLLPRI`; `writable` = `EPOLLOUT`; `error` = `EPOLLERR`;
//!   `read_closed` = `EPOLLHUP`, or `EPOLLIN` with `EPOLLRDHUP`; `write_closed` =
//!   `EPOLLHUP`, or `EPOLLOUT` with `EPOLLERR`, or `EPOLLERR` alone. The error
//!   and hang-up conditions must be reported whatever the interest (epoll does
//!   so), and a registration should ask for `EPOLLRDHUP` so that a peer's
//!   half-close is visible. The reactor wakes a resource's reader on
//!   `readable || read_closed || error` and its writer on
//!   `writable || write_closed || error`, so an error-only event (a pipe whose
//!   reader went away while the writer waits for space) wakes the writer.
//! * **Wait.** `wait(events, timeout)` appends the ready events to `events`
//!   (the reactor passes it empty, with capacity reserved; a backend should
//!   not exceed `events.capacity()`, and events it does not return stay pending
//!   for the next wait). `timeout` `None` blocks until an event or an
//!   interrupt; `Some(d)` blocks at most about `d` (the reactor's clock is in
//!   milliseconds); `Some(Duration::ZERO)` does not block. A wait cut short by a
//!   signal (`EINTR`) returns `Ok` with the events it has, possibly none. Any
//!   other error is returned; the reactor then treats the park as an empty one
//!   (expired timers still fire).
//! * **Interrupt.** [`OsBackend::interrupt`] returns the backend's
//!   [`OsInterrupt`]; the reactor calls it once, when it is built, and shares it
//!   with every thread that may need to wake the loop. `signal()` may be called
//!   from any thread at any time. A signal made before or during a wait makes
//!   that wait return promptly; a signal made while no wait is in progress makes
//!   the next wait return promptly. Signals may coalesce into one early return,
//!   and waits may return early without a signal. The backend consumes its own
//!   notification inside `wait`, on the owner thread (for example by reading an
//!   eventfd registered under a reserved token), and never reports it as an
//!   event. Callers coalesce: the reactor's `InterruptHandle` signals only on
//!   the first wake after its owner reset it, and the owner resets before it
//!   drains its cross-thread queue and then waits, so no wake is lost as long as
//!   a signal is never dropped between `signal()` and the next wait's return.
//!
//! All methods but `OsInterrupt::signal` run on the thread that owns the
//! reactor.

use crate::types::Interest;
use std::io;
use std::sync::Arc;
use std::time::Duration;

#[cfg(feature = "mio")]
mod mio_backend;
#[cfg(feature = "mio")]
pub use mio_backend::MioBackend;

#[cfg(feature = "test-backend")]
pub mod mock;

/// A raw file descriptor, as the OS knows it (`std::os::fd::RawFd` on Unix).
pub type RawFd = i32;

/// One readiness report for a registered fd. See the module documentation for
/// the meaning of each flag.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct OsEvent {
  /// The token the fd was registered under.
  pub token: usize,
  pub readable: bool,
  pub writable: bool,
  pub error: bool,
  pub read_closed: bool,
  pub write_closed: bool,
}

/// What the reactor needs from the operating system. Object safe: the reactor
/// holds a `Box<dyn OsBackend>`. See the module documentation for the
/// edge-triggered contract every implementation must meet.
pub trait OsBackend: Send {
  /// Starts reporting `fd` under `token` for the directions in `interest`.
  fn register(&mut self, fd: RawFd, token: usize, interest: Interest) -> io::Result<()>;

  /// Replaces the token and interest of an fd registered earlier.
  fn reregister(&mut self, fd: RawFd, token: usize, interest: Interest) -> io::Result<()>;

  /// Stops all reports for `fd`, including pending ones.
  fn deregister(&mut self, fd: RawFd) -> io::Result<()>;

  /// Blocks for at most `timeout` (`None`: no bound) until at least one event
  /// is ready or the interrupt is signalled, and appends the ready events to
  /// `events`.
  fn wait(&mut self, events: &mut Vec<OsEvent>, timeout: Option<Duration>) -> io::Result<()>;

  /// The backend's cross-thread interrupt. Called once, when the reactor is
  /// built.
  fn interrupt(&self) -> Arc<dyn OsInterrupt>;
}

/// The cross-thread half of a backend: makes the owner's current or next
/// [`OsBackend::wait`] return promptly. Callable from any thread.
pub trait OsInterrupt: Send + Sync {
  fn signal(&self) -> io::Result<()>;
}
