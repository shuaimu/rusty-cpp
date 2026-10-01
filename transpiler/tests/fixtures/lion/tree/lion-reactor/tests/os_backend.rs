// The reactor alone (no executor) over the in-process mock backend
// (lion_reactor::os::mock): timers, fd readiness, error events and the
// cross-thread interrupt all go through `OsBackend`, with no OS involved.
// Run with `cargo test --features test-backend`.

use lion_reactor::os::mock::{MockBackend, MockOs};
use lion_reactor::os::OsEvent;
use lion_reactor::{readiness, Duration, Instant, Interest, InterruptHandle, Reactor, ReactorHandle, Source, Waker};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::task::Wake;
use std::time::Instant as StdInstant;

struct Counter(AtomicUsize);

impl Wake for Counter {
  fn wake(self: Arc<Self>) {
    self.0.fetch_add(1, Ordering::SeqCst);
  }
  fn wake_by_ref(self: &Arc<Self>) {
    self.0.fetch_add(1, Ordering::SeqCst);
  }
}

fn counter() -> (Arc<Counter>, Waker) {
  let c = Arc::new(Counter(AtomicUsize::new(0)));
  let w = Waker::from_std(std::task::Waker::from(c.clone()));
  (c, w)
}

fn count(c: &Arc<Counter>) -> usize {
  c.0.load(Ordering::SeqCst)
}

fn mock_reactor() -> (Box<Reactor>, InterruptHandle, MockOs) {
  let (backend, os) = MockBackend::new();
  let (reactor, interrupt) = match Reactor::with_backend(Box::new(backend)) {
    lion_reactor::IoResult::Ok(r) => r,
    lion_reactor::IoResult::Err(e) => panic!("reactor over the mock backend: {e:?}"),
  };
  (Box::new(reactor), interrupt, os)
}

fn park(reactor: &mut Reactor, ms: u64) {
  let _ = reactor.park(Some(Duration::from_millis(ms)));
}

#[test]
fn timer_fires_through_the_backend_wait() {
  let (mut reactor, _interrupt, os) = mock_reactor();
  let _guard = reactor.enter();
  let (fired, waker) = counter();
  let deadline = Instant::now() + Duration::from_millis(30);
  let start = StdInstant::now();
  match ReactorHandle::new().register_timer(deadline, waker) {
    lion_reactor::IoResult::Ok(_) => {}
    lion_reactor::IoResult::Err(e) => panic!("register_timer: {e:?}"),
  }
  // The park bound is 10 s; the reactor cuts the wait at the timer deadline.
  while count(&fired) == 0 {
    park(&mut reactor, 10_000);
    assert!(start.elapsed() < std::time::Duration::from_secs(5), "timer did not fire");
  }
  assert!(start.elapsed() >= std::time::Duration::from_millis(25), "timer fired early");
  assert_eq!(count(&fired), 1);
  assert!(os.stats().waits >= 1);
}

#[test]
fn readiness_edge_wakes_the_registered_waker() {
  let (mut reactor, _interrupt, os) = mock_reactor();
  let _guard = reactor.enter();
  let handle = ReactorHandle::new();
  let fd = 100;
  let mut source = Source::new(fd);
  let rid = match handle.register_io_resource(&mut source, Interest::READABLE_WRITABLE) {
    lion_reactor::IoResult::Ok(rid) => rid,
    lion_reactor::IoResult::Err(e) => panic!("register: {e:?}"),
  };
  // The backend sees the fd under the rid's token.
  let (token, interest) = os.registration(fd).expect("fd registered with the backend");
  assert_eq!(token, rid.0 as usize);
  assert!(interest.readable && interest.writable);
  readiness::init_readiness(rid);
  // As after an observed WouldBlock: not readable, waiter armed.
  readiness::clear_readable(rid);
  let (woken, waker) = counter();
  handle.set_waker(rid, Interest::READABLE, waker);

  let os2 = os.clone();
  let t = std::thread::spawn(move || {
    std::thread::sleep(std::time::Duration::from_millis(30));
    assert!(os2.ready(fd, true, false));
  });
  let start = StdInstant::now();
  while count(&woken) == 0 {
    park(&mut reactor, 10_000);
    assert!(start.elapsed() < std::time::Duration::from_secs(5), "readiness did not wake");
  }
  t.join().unwrap();
  assert!(readiness::is_readable(rid));
  assert_eq!(count(&woken), 1);

  let _ = handle.deregister_io_resource(rid, &mut source);
  assert!(!os.is_registered(fd));
  assert!(!os.ready(fd, true, true), "a deregistered fd reports nothing");
}

// An event carrying only an error (EPOLLERR alone: a pipe whose reader closed
// while the writer waits for space) must wake both directions, or the writer
// waits forever.
#[test]
fn error_only_event_wakes_both_directions() {
  let (mut reactor, _interrupt, os) = mock_reactor();
  let _guard = reactor.enter();
  let handle = ReactorHandle::new();
  let fd = 7;
  let mut source = Source::new(fd);
  let rid = match handle.register_io_resource(&mut source, Interest::READABLE_WRITABLE) {
    lion_reactor::IoResult::Ok(rid) => rid,
    lion_reactor::IoResult::Err(e) => panic!("register: {e:?}"),
  };
  readiness::init_readiness(rid);
  readiness::clear_readable(rid);
  readiness::clear_writable(rid);
  let (reader, rw) = counter();
  let (writer, ww) = counter();
  handle.set_waker(rid, Interest::READABLE, rw);
  handle.set_waker(rid, Interest::WRITABLE, ww);
  assert!(os.event(fd, OsEvent { error: true, write_closed: true, ..OsEvent::default() }));
  park(&mut reactor, 1_000);
  assert_eq!(count(&reader), 1, "reader not woken by an error event");
  assert_eq!(count(&writer), 1, "writer not woken by an error event");
  assert!(readiness::is_readable(rid) && readiness::is_writable(rid));
  let _ = handle.deregister_io_resource(rid, &mut source);
}

#[test]
fn foreign_interrupt_ends_the_park_and_is_consumed() {
  let (mut reactor, interrupt, os) = mock_reactor();
  let _guard = reactor.enter();
  let remote = interrupt.clone();
  let t = std::thread::spawn(move || {
    std::thread::sleep(std::time::Duration::from_millis(50));
    remote.wake();
  });
  let start = StdInstant::now();
  park(&mut reactor, 10_000);
  let first = start.elapsed();
  t.join().unwrap();
  assert!(first < std::time::Duration::from_secs(5), "interrupt did not end the park ({first:?})");
  assert_eq!(os.stats().signals, 1);
  assert_eq!(os.stats().interrupted_waits, 1);

  // Consumed: after the owner's reset, a park with nothing pending waits out
  // its bound instead of returning at once.
  interrupt.reset();
  let start = StdInstant::now();
  park(&mut reactor, 60);
  assert!(start.elapsed() >= std::time::Duration::from_millis(50), "interrupt not consumed");

  // Coalescing: two wakes before a reset signal the backend once.
  interrupt.wake();
  interrupt.wake();
  assert_eq!(os.stats().signals, 2);
  park(&mut reactor, 10_000);
  interrupt.reset();
  interrupt.wake();
  assert_eq!(os.stats().signals, 3);
}
