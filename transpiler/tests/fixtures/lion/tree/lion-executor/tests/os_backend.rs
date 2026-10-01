// A whole runtime over the in-process mock backend (lion_reactor::os::mock):
// timers, fd readiness and cross-thread wakes all reach the executor through
// `OsBackend`. These tests also run with `--no-default-features`, where no mio
// is linked at all and `Runtime::new` has no backend to use.

use lion_executor::os::mock::{MockBackend, MockOs};
use lion_executor::{Runtime, RuntimeBuilder};
use lion_reactor::{readiness, Duration, Instant, Interest, IoResult, ReactorHandle, ResourceId, Source, Waker};
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::task::{Context, Poll};
use std::time::Instant as StdInstant;

fn mock_runtime() -> (Runtime, MockOs) {
  let (backend, os) = MockBackend::new();
  let rt = RuntimeBuilder::new().os_backend(Box::new(backend)).build().expect("runtime over the mock backend");
  (rt, os)
}

// A one-shot reactor timer: registers `cx.waker()` until the deadline passes.
struct Sleep {
  deadline: Instant,
  rid: Option<ResourceId>,
}

fn sleep_ms(ms: u64) -> Sleep {
  Sleep { deadline: Instant::now() + Duration::from_millis(ms), rid: None }
}

impl Future for Sleep {
  type Output = ();
  fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
    let handle = ReactorHandle::new();
    if let Some(rid) = self.rid.take() {
      handle.deregister_timer(rid);
    }
    let now = ReactorHandle::cached_now().unwrap_or_else(Instant::now);
    if now.inner >= self.deadline.inner {
      return Poll::Ready(());
    }
    match handle.register_timer(self.deadline, Waker::from_std(cx.waker().clone())) {
      IoResult::Ok(rid) => self.rid = Some(rid),
      IoResult::Err(e) => panic!("register_timer: {e:?}"),
    }
    Poll::Pending
  }
}

// Fails the test instead of hanging when `fut` misses a wakeup: the reactor
// timer of `limit` is a wake source of its own.
struct Deadline<F> {
  fut: Pin<Box<F>>,
  limit: Sleep,
}

fn deadline<F: Future>(ms: u64, fut: F) -> Deadline<F> {
  Deadline { fut: Box::pin(fut), limit: sleep_ms(ms) }
}

impl<F: Future> Future for Deadline<F> {
  type Output = F::Output;
  fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<F::Output> {
    if let Poll::Ready(v) = self.fut.as_mut().poll(cx) {
      return Poll::Ready(v);
    }
    if Pin::new(&mut self.limit).poll(cx).is_ready() {
      panic!("no wakeup within the deadline");
    }
    Poll::Pending
  }
}

// Waits until fd `fd`'s read flag is set, the raw way (U8's AsyncFd wraps it).
struct Readable {
  rid: ResourceId,
}

impl Future for Readable {
  type Output = ();
  fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
    if readiness::is_readable(self.rid) {
      return Poll::Ready(());
    }
    ReactorHandle::new().set_waker(self.rid, Interest::READABLE, Waker::from_std(cx.waker().clone()));
    Poll::Pending
  }
}

#[test]
fn timer_fires_on_a_custom_backend() {
  let (rt, os) = mock_runtime();
  let start = StdInstant::now();
  rt.block_on(async {
    let j = lion_executor::spawn(async { sleep_ms(30).await; 7 });
    sleep_ms(10).await;
    assert_eq!(j.await.unwrap(), 7);
  });
  assert!(start.elapsed() >= std::time::Duration::from_millis(25));
  assert!(os.stats().waits >= 1);
}

#[test]
fn readiness_from_another_thread_wakes_a_task() {
  let (rt, os) = mock_runtime();
  let fd = 42;
  let done = Arc::new(AtomicBool::new(false));
  let done2 = done.clone();
  let os2 = os.clone();
  rt.block_on(async move {
    let mut source = Source::new(fd);
    let rid = match ReactorHandle::new().register_io_resource(&mut source, Interest::READABLE_WRITABLE) {
      IoResult::Ok(rid) => rid,
      IoResult::Err(e) => panic!("register: {e:?}"),
    };
    readiness::init_readiness(rid);
    readiness::clear_readable(rid);
    let t = std::thread::spawn(move || {
      std::thread::sleep(std::time::Duration::from_millis(30));
      assert!(os2.ready(fd, true, false));
    });
    deadline(5_000, Readable { rid }).await;
    done2.store(true, Ordering::SeqCst);
    let _ = ReactorHandle::new().deregister_io_resource(rid, &mut source);
    t.join().unwrap();
  });
  assert!(done.load(Ordering::SeqCst));
  assert_eq!(os.registered(), 0);
}

#[test]
fn foreign_spawn_cuts_an_idle_park_short() {
  let (rt, os) = mock_runtime();
  let handle = rt.handle().clone();
  let ran = Arc::new(AtomicBool::new(false));
  let ran2 = ran.clone();
  let t = std::thread::spawn(move || {
    std::thread::sleep(std::time::Duration::from_millis(50));
    handle.spawn(async move { ran2.store(true, Ordering::SeqCst) });
  });
  let start = StdInstant::now();
  while !ran.load(Ordering::SeqCst) {
    rt.tick_with_timeout(std::time::Duration::from_secs(10));
    assert!(start.elapsed() < std::time::Duration::from_secs(5), "foreign spawn did not interrupt the park");
  }
  t.join().unwrap();
  assert!(os.stats().interrupted_waits >= 1);
}

#[cfg(not(feature = "mio"))]
#[test]
fn runtime_new_without_mio_is_unsupported() {
  match Runtime::new() {
    Ok(_) => panic!("Runtime::new succeeded without a backend"),
    Err(e) => assert_eq!(e.kind(), std::io::ErrorKind::Unsupported),
  }
}

#[cfg(feature = "mio")]
#[test]
fn runtime_new_uses_the_mio_backend() {
  let rt = Runtime::new().expect("default runtime");
  assert_eq!(rt.block_on(async { sleep_ms(5).await; 1 }), 1);
}
