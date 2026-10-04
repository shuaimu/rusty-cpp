// Trusted glue: Lion's default OS backend over mio (epoll on Linux). Plain
// Rust; compiled only with the `mio` feature.
#![cfg_attr(verus_keep_ghost, verus::trusted)]

use super::{OsBackend, OsEvent, OsInterrupt, RawFd};
use crate::types::Interest;
use mio::unix::SourceFd;
use std::io;
use std::sync::Arc;
use std::time::Duration;

// The token of mio's cross-thread waker. Resource ids start at 1 and are never
// reused, so the reactor never registers token 0.
const INTERRUPT_TOKEN: mio::Token = mio::Token(0);

// Capacity of one wait's event batch, as before the backend seam.
const EVENT_CAPACITY: usize = 1024;

/// The mio (epoll) backend: every fd is registered edge-triggered with
/// `EPOLLRDHUP`, as mio does, and the interrupt is mio's eventfd `Waker`, itself
/// registered edge-triggered, so every `signal` produces a fresh event and
/// there is nothing to drain.
pub struct MioBackend {
  poll: mio::Poll,
  events: mio::Events,
  interrupt: Arc<MioInterrupt>,
}

struct MioInterrupt(mio::Waker);

impl OsInterrupt for MioInterrupt {
  fn signal(&self) -> io::Result<()> {
    self.0.wake()
  }
}

impl MioBackend {
  pub fn new() -> io::Result<MioBackend> {
    let poll = mio::Poll::new()?;
    let waker = mio::Waker::new(poll.registry(), INTERRUPT_TOKEN)?;
    Ok(MioBackend {
      poll,
      events: mio::Events::with_capacity(EVENT_CAPACITY),
      interrupt: Arc::new(MioInterrupt(waker)),
    })
  }
}

fn interest_to_mio(interest: Interest) -> mio::Interest {
  match (interest.readable, interest.writable) {
    (true, true) => mio::Interest::READABLE.add(mio::Interest::WRITABLE),
    (true, false) => mio::Interest::READABLE,
    (false, true) => mio::Interest::WRITABLE,
    (false, false) => mio::Interest::READABLE,
  }
}

impl OsBackend for MioBackend {
  fn register(&mut self, fd: RawFd, token: usize, interest: Interest) -> io::Result<()> {
    self.poll.registry().register(&mut SourceFd(&fd), mio::Token(token), interest_to_mio(interest))
  }

  fn reregister(&mut self, fd: RawFd, token: usize, interest: Interest) -> io::Result<()> {
    self.poll.registry().reregister(&mut SourceFd(&fd), mio::Token(token), interest_to_mio(interest))
  }

  fn deregister(&mut self, fd: RawFd) -> io::Result<()> {
    self.poll.registry().deregister(&mut SourceFd(&fd))
  }

  fn wait(&mut self, events: &mut Vec<OsEvent>, timeout: Option<Duration>) -> io::Result<()> {
    match self.poll.poll(&mut self.events, timeout) {
      Ok(()) => {}
      // A signal cut the wait short: an empty wait, not a failed one.
      Err(e) if e.kind() == io::ErrorKind::Interrupted => return Ok(()),
      Err(e) => return Err(e),
    }
    for e in self.events.iter() {
      // The interrupt only ends the wait; it is not an io event.
      if e.token() == INTERRUPT_TOKEN {
        continue;
      }
      events.push(OsEvent {
        token: e.token().0,
        readable: e.is_readable(),
        writable: e.is_writable(),
        error: e.is_error(),
        read_closed: e.is_read_closed(),
        write_closed: e.is_write_closed(),
      });
    }
    Ok(())
  }

  fn interrupt(&self) -> Arc<dyn OsInterrupt> {
    self.interrupt.clone()
  }
}
