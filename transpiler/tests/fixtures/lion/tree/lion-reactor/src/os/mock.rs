// Trusted glue (test support): an in-process OS backend. Plain Rust; compiled
// only with the `test-backend` feature.
#![cfg_attr(verus_keep_ghost, verus::trusted)]

//! An in-process [`OsBackend`] whose readiness the test injects.
//!
//! [`MockBackend::new`] returns the backend, to hand to a reactor
//! (`Reactor::with_backend`, `RuntimeBuilder::os_backend`), and a [`MockOs`]
//! handle that plays the operating system from any thread: it reports readiness
//! edges for registered fds and shows what the reactor registered. Fds are just
//! numbers here; nothing is ever opened, read or closed.
//!
//! It behaves as the edge-triggered contract in [`crate::os`] asks and no more
//! generously: an event exists only when the test reports one (registering an
//! fd reports nothing), `readable`/`writable` are masked by the registered
//! interest while error and hang-up flags are always delivered, events for one
//! fd pending at the same time merge into one (as in epoll's ready list),
//! deregistering drops the fd's pending event, and a signalled interrupt ends
//! the current or next wait and is consumed by that wait.

use super::{OsBackend, OsEvent, OsInterrupt, RawFd};
use crate::types::Interest;
use std::collections::{HashMap, VecDeque};
use std::io;
use std::sync::{Arc, Condvar, Mutex, MutexGuard, PoisonError};
use std::time::{Duration, Instant};

/// Counters of what the reactor asked of the mock.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MockStats {
  pub registers: u64,
  pub reregisters: u64,
  pub deregisters: u64,
  /// Calls of `wait`.
  pub waits: u64,
  /// Waits that returned because the interrupt was signalled.
  pub interrupted_waits: u64,
  /// Calls of `OsInterrupt::signal`.
  pub signals: u64,
}

#[derive(Default)]
struct State {
  registered: HashMap<RawFd, (usize, Interest)>,
  // Events not yet returned by a wait, at most one per fd.
  pending: VecDeque<(RawFd, OsEvent)>,
  signalled: bool,
  stats: MockStats,
}

struct Shared {
  state: Mutex<State>,
  cond: Condvar,
}

impl Shared {
  fn lock(&self) -> MutexGuard<'_, State> {
    self.state.lock().unwrap_or_else(PoisonError::into_inner)
  }
}

/// The backend half: give it to the reactor.
pub struct MockBackend {
  shared: Arc<Shared>,
  interrupt: Arc<MockInterrupt>,
}

/// The test's half: plays the OS. Cloneable and usable from any thread.
#[derive(Clone)]
pub struct MockOs {
  shared: Arc<Shared>,
}

struct MockInterrupt {
  shared: Arc<Shared>,
}

impl MockBackend {
  pub fn new() -> (MockBackend, MockOs) {
    let shared = Arc::new(Shared { state: Mutex::new(State::default()), cond: Condvar::new() });
    let interrupt = Arc::new(MockInterrupt { shared: shared.clone() });
    (MockBackend { shared: shared.clone(), interrupt }, MockOs { shared })
  }
}

impl MockOs {
  /// Reports a readiness edge for `fd` in the given directions (masked by its
  /// registered interest). Returns whether an event was queued: false if `fd`
  /// is not registered or no requested direction is in its interest.
  pub fn ready(&self, fd: RawFd, readable: bool, writable: bool) -> bool {
    self.event(fd, OsEvent { readable, writable, ..OsEvent::default() })
  }

  /// Reports an arbitrary event for `fd`. `token` is ignored (the registered
  /// token is used); `readable`/`writable` are masked by the registered
  /// interest; `error`, `read_closed` and `write_closed` are always delivered.
  pub fn event(&self, fd: RawFd, event: OsEvent) -> bool {
    let mut st = self.shared.lock();
    let Some(&(token, interest)) = st.registered.get(&fd) else {
      return false;
    };
    let ev = OsEvent {
      token,
      readable: event.readable && interest.readable,
      writable: event.writable && interest.writable,
      error: event.error,
      read_closed: event.read_closed,
      write_closed: event.write_closed,
    };
    if !(ev.readable || ev.writable || ev.error || ev.read_closed || ev.write_closed) {
      return false;
    }
    if let Some((_, p)) = st.pending.iter_mut().find(|(f, _)| *f == fd) {
      p.readable |= ev.readable;
      p.writable |= ev.writable;
      p.error |= ev.error;
      p.read_closed |= ev.read_closed;
      p.write_closed |= ev.write_closed;
    } else {
      st.pending.push_back((fd, ev));
    }
    drop(st);
    self.shared.cond.notify_all();
    true
  }

  pub fn is_registered(&self, fd: RawFd) -> bool {
    self.shared.lock().registered.contains_key(&fd)
  }

  /// The token and interest `fd` is registered with.
  pub fn registration(&self, fd: RawFd) -> Option<(usize, Interest)> {
    self.shared.lock().registered.get(&fd).copied()
  }

  /// Number of registered fds.
  pub fn registered(&self) -> usize {
    self.shared.lock().registered.len()
  }

  pub fn stats(&self) -> MockStats {
    self.shared.lock().stats
  }
}

impl OsInterrupt for MockInterrupt {
  fn signal(&self) -> io::Result<()> {
    let mut st = self.shared.lock();
    st.signalled = true;
    st.stats.signals += 1;
    drop(st);
    self.shared.cond.notify_all();
    Ok(())
  }
}

impl OsBackend for MockBackend {
  fn register(&mut self, fd: RawFd, token: usize, interest: Interest) -> io::Result<()> {
    let mut st = self.shared.lock();
    if st.registered.contains_key(&fd) {
      return Err(io::Error::new(io::ErrorKind::AlreadyExists, "mock: fd already registered"));
    }
    st.registered.insert(fd, (token, interest));
    st.stats.registers += 1;
    Ok(())
  }

  fn reregister(&mut self, fd: RawFd, token: usize, interest: Interest) -> io::Result<()> {
    let mut st = self.shared.lock();
    match st.registered.get_mut(&fd) {
      Some(r) => *r = (token, interest),
      None => return Err(io::Error::new(io::ErrorKind::NotFound, "mock: fd not registered")),
    }
    st.stats.reregisters += 1;
    Ok(())
  }

  fn deregister(&mut self, fd: RawFd) -> io::Result<()> {
    let mut st = self.shared.lock();
    if st.registered.remove(&fd).is_none() {
      return Err(io::Error::new(io::ErrorKind::NotFound, "mock: fd not registered"));
    }
    st.pending.retain(|(f, _)| *f != fd);
    st.stats.deregisters += 1;
    Ok(())
  }

  fn wait(&mut self, events: &mut Vec<OsEvent>, timeout: Option<Duration>) -> io::Result<()> {
    let deadline = timeout.map(|t| Instant::now() + t);
    let mut st = self.shared.lock();
    st.stats.waits += 1;
    loop {
      if st.signalled || !st.pending.is_empty() {
        if st.signalled {
          st.signalled = false;
          st.stats.interrupted_waits += 1;
        }
        let cap = if events.capacity() == 0 { usize::MAX } else { events.capacity() };
        while events.len() < cap {
          match st.pending.pop_front() {
            Some((_, ev)) => events.push(ev),
            None => break,
          }
        }
        return Ok(());
      }
      match deadline {
        None => st = self.shared.cond.wait(st).unwrap_or_else(PoisonError::into_inner),
        Some(d) => {
          let now = Instant::now();
          if now >= d {
            return Ok(());
          }
          st = self.shared.cond.wait_timeout(st, d - now).unwrap_or_else(PoisonError::into_inner).0;
        }
      }
    }
  }

  fn interrupt(&self) -> Arc<dyn OsInterrupt> {
    self.interrupt.clone()
  }
}
