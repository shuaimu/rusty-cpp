#![allow(unused_imports)]
#![allow(unused_braces)]
#![allow(unused_variables)]
#![allow(unused_assignments)]
#![allow(dead_code)]
#![allow(unused_mut)]

use vstd::prelude::*;

mod collections;
mod config;
mod executor;
mod framework;
mod spec;
mod proof;
pub mod types;
mod handle;
pub mod tls;
pub mod blocking;

use std::future::Future;
use std::pin::pin;
use std::task::{Context, Poll};
use std::cell::RefCell;
use std::sync::Arc;
use std::marker::PhantomData;
use lion_reactor::{InterruptHandle, OsBackend, Reactor};
use config::RuntimeConfig;
use handle::InnerHandle;
use collections::mpsc_queue;
use types::{BoxedFuture, ReactorGuard, Task, TaskId, WakeSource};
use executor::Executor;

#[derive(Clone)]
pub struct ExecutorHandle {
  inner: InnerHandle,
}

impl ExecutorHandle {
  pub fn spawn<T: Send + 'static>(&self, future: impl Future<Output = T> + Send + 'static) -> types::JoinHandle<T> {
  self.inner.spawn(future)
  }

  /// Spawns a future that need not be `Send` onto this handle's runtime. The
  /// task is polled and dropped only on the runtime's own thread (the thread
  /// that created the `Runtime`).
  ///
  /// # Panics
  ///
  /// If called on any other thread. Use [`ExecutorHandle::spawn`] from there.
  pub fn spawn_local<F>(&self, future: F) -> types::JoinHandle<F::Output>
  where
    F: Future + 'static,
    F::Output: 'static,
  {
  self.inner.spawn_local(future)
  }
}

thread_local! {
  static CURRENT_HANDLE: RefCell<Option<ExecutorHandle>> = RefCell::new(None);
}

/// A single-threaded Lion runtime. It is bound to the thread that creates it
/// (it is neither `Send` nor `Sync`): that thread drives it, and `spawn_local`
/// tasks live only there. A thread runs at most one Lion runtime at a time:
/// `Runtime::new` on a thread whose runtime is still alive fails with
/// `ErrorKind::AlreadyExists` (use another thread, e.g. `block_in_place` in the
/// `lion` facade). Once the runtime is dropped, the thread may create another.
///
/// ```compile_fail
/// fn assert_send<T: Send>() {}
/// assert_send::<lion_executor::Runtime>();
/// ```
pub struct Runtime {
  // Field order is drop order: the executor (its tasks and the injection
  // receiver) goes first, while the reactor guard is still in place.
  executor: RefCell<Box<Executor>>,
  handle: ExecutorHandle,
  _guard: ReactorGuard,
  // Last: releases this runtime's thread-local registrations after everything
  // above is gone, so task destructors that spawn or wake still find it.
  _thread: ThreadBinding,
  // !Send: the executor may hold spawn_local tasks, which must be dropped on
  // this thread (see types/boxed_future.rs, OwnerThreadOnly).
  _not_send: PhantomData<*const ()>,
}

// What a Runtime installed in its thread's TLS, released on drop. Each entry is
// cleared only if it is still this runtime's; the per-thread wake queues are
// reset only if the cross-thread context was, i.e. if this was the thread's
// runtime. Uses try_with throughout: a Runtime kept in a thread_local may drop
// while the thread's other TLS is being destroyed.
struct ThreadBinding {
  runtime_id: usize,
  cross_thread_queue: Arc<tls::CrossThreadQueue>,
}

impl Drop for ThreadBinding {
  fn drop(&mut self) {
    let _ = CURRENT_HANDLE.try_with(|h| {
      if let Ok(mut h) = h.try_borrow_mut() {
        if h.as_ref().is_some_and(|x| x.inner.runtime_id() == self.runtime_id) {
          *h = None;
        }
      }
    });
    tls::release_runtime_thread_state(&self.cross_thread_queue);
  }
}

// Whether this thread already has a live Lion runtime (or a bare lion-reactor
// Reactor entered). A thread whose TLS is being torn down counts as busy.
fn runtime_is_live_on_this_thread() -> bool {
  CURRENT_HANDLE.try_with(|h| h.borrow().is_some()).unwrap_or(true)
    || tls::has_cross_thread_ctx()
    || Reactor::is_entered_on_current_thread()
}

impl Runtime {
  /// A runtime over Lion's default OS backend (mio). Without the `mio`
  /// feature this fails with `ErrorKind::Unsupported`: give the runtime a
  /// backend with [`RuntimeBuilder::os_backend`].
  pub fn new() -> std::io::Result<Self> {
  RuntimeBuilder::new().build()
  }

  fn with_config(config: RuntimeConfig, backend: Option<Box<dyn OsBackend>>) -> std::io::Result<Self> {
  // Before anything is created or installed: a refused Runtime::new leaves
  // this thread's runtime, and its TLS, untouched.
  if runtime_is_live_on_this_thread() {
    return Err(std::io::Error::new(
      std::io::ErrorKind::AlreadyExists,
      "a Lion runtime is already running on this thread; runtimes do not nest (run the new one on another thread)",
    ));
  }
  let (reactor, interrupt_handle) = new_reactor(backend)?;

  let cross_thread_queue = tls::CrossThreadQueue::new();
  tls::set_cross_thread_ctx(
    cross_thread_queue.clone(),
    interrupt_handle.clone(),
    std::thread::current().id(),
  );

  let (injection_sender, injection_receiver) = mpsc_queue();
  let mut executor = Box::new(Executor::new(reactor, injection_receiver, config));
  let guard = executor.enter();
  let inner_handle = InnerHandle::new(injection_sender, interrupt_handle);
  let handle = ExecutorHandle { inner: inner_handle };

  CURRENT_HANDLE.with(|h| {
    *h.borrow_mut() = Some(handle.clone());
  });
  let thread = ThreadBinding { runtime_id: handle.inner.runtime_id(), cross_thread_queue };

  Ok(Runtime {
    executor: RefCell::new(executor),
    handle,
    _guard: guard,
    _thread: thread,
    _not_send: PhantomData,
  })
  }

  pub fn handle(&self) -> &ExecutorHandle {
  &self.handle
  }

  pub fn block_on<F: Future>(&self, future: F) -> F::Output {
    let mut future = pin!(future);
    let block_on_task_id = TaskId(u64::MAX);
    let waker = types::create_waker(block_on_task_id, WakeSource::Task, false);
    let mut context = Context::from_waker(&waker);

    loop {
      let poll_result = {
        let _current = tls::CurrentTaskGuard::enter(block_on_task_id);
        future.as_mut().poll(&mut context)
      };
      if let Poll::Ready(result) = poll_result {
        return result;
      }
      self.run_tick(types::IDLE_PARK_MS);
    }
  }

  /// Runs one iteration of the loop that `block_on` drives, for an embedder
  /// that owns its own loop: wake deferred tasks, admit newly spawned ones,
  /// poll up to `event_interval` ready tasks, park the reactor, collect the
  /// wakes it delivered (io, timers, other threads), and poll up to
  /// `event_interval` ready tasks again. Tasks come from `spawn`,
  /// `spawn_local` and the runtime's handle; `tick` never polls a future of
  /// its own.
  ///
  /// The park blocks only when no task is ready, and then for at most 100 ms,
  /// cut short by the next timer deadline, an io event, a wake from another
  /// thread or a spawn. So an idle `tick` can take up to 100 ms. A loop with
  /// work of its own must use [`Runtime::tick_with_timeout`] with the time it
  /// can afford to wait (`Duration::ZERO` for a non-blocking step): the
  /// executor cannot see the caller's work, and a plain `tick` would sleep
  /// through it.
  ///
  /// # Panics
  ///
  /// If called from inside a task or a `block_on` future on this thread: the
  /// loop is not re-entrant.
  pub fn tick(&self) {
    self.run_tick(types::IDLE_PARK_MS);
  }

  /// [`Runtime::tick`] with the idle park bounded by `max_park` instead of
  /// 100 ms (rounded up to whole milliseconds; the reactor's clock is in ms).
  /// `Duration::ZERO` makes the step non-blocking; a longer bound is allowed,
  /// since every wake source (io, timers, spawns, other threads) interrupts
  /// the park. When a task is ready the park never blocks, whatever the bound.
  ///
  /// # Panics
  ///
  /// As [`Runtime::tick`].
  pub fn tick_with_timeout(&self, max_park: std::time::Duration) {
    let ms = max_park.as_nanos().div_ceil(1_000_000).min(u64::MAX as u128) as u64;
    self.run_tick(ms);
  }

  // Every tick sets the idle park bound it wants, so no caller inherits
  // another's.
  fn run_tick(&self, idle_park_ms: u64) {
    assert!(
      tls::get_current_task().is_none(),
      "Runtime::tick called from inside a task or a block_on future: the runtime loop is not re-entrant"
    );
    let mut exec = self
      .executor
      .try_borrow_mut()
      .expect("the Lion runtime loop is not re-entrant");
    exec.reactor.set_idle_park_ms(idle_park_ms);
    exec.tick();
  }
}

pub fn spawn<T: Send + 'static>(future: impl Future<Output = T> + Send + 'static) -> JoinHandle<T> {
  CURRENT_HANDLE.with(|h| {
  h.borrow()
    .as_ref()
    .expect("lion::spawn() called outside Lion runtime context")
    .spawn(future)
  })
}

/// Spawns a future that need not be `Send` onto the current thread's runtime.
/// It is polled and dropped only on this thread.
///
/// # Panics
///
/// If this thread has no running Lion runtime.
pub fn spawn_local<F>(future: F) -> JoinHandle<F::Output>
where
  F: Future + 'static,
  F::Output: 'static,
{
  CURRENT_HANDLE.with(|h| {
  h.borrow()
    .as_ref()
    .expect("lion::spawn_local() called outside Lion runtime context")
    .spawn_local(future)
  })
}

pub use types::JoinHandle;
pub use types::JoinSender;
pub use types::join_handle::JoinError;
pub use types::waker::create_reactor_waker_for_current;
pub use blocking::spawn_blocking;
pub use lion_reactor::os;

// The reactor of a new runtime: over the embedder's backend if one was given,
// else over Lion's mio backend (feature `mio`).
fn new_reactor(backend: Option<Box<dyn OsBackend>>) -> std::io::Result<(Reactor, InterruptHandle)> {
  let created = match backend {
    Some(backend) => Reactor::with_backend(backend),
    None => default_reactor()?,
  };
  match created {
    lion_reactor::IoResult::Ok(r) => Ok(r),
    lion_reactor::IoResult::Err(e) => {
      let e = e.into_io_error();
      Err(std::io::Error::new(e.kind(), format!("Failed to create reactor: {e}")))
    }
  }
}

#[cfg(feature = "mio")]
fn default_reactor() -> std::io::Result<lion_reactor::IoResult<(Reactor, InterruptHandle)>> {
  Ok(Reactor::new())
}

#[cfg(not(feature = "mio"))]
fn default_reactor() -> std::io::Result<lion_reactor::IoResult<(Reactor, InterruptHandle)>> {
  Err(std::io::Error::new(
    std::io::ErrorKind::Unsupported,
    "lion-executor was built without its `mio` feature, so it has no default OS backend: \
     build the runtime with RuntimeBuilder::os_backend",
  ))
}

pub struct RuntimeBuilder {
  config: RuntimeConfig,
  backend: Option<Box<dyn OsBackend>>,
}

impl RuntimeBuilder {
  pub fn new() -> Self {
  Self {
    config: RuntimeConfig::default(),
    backend: None,
  }
  }

  /// Drives the runtime's reactor with this OS backend instead of Lion's mio
  /// backend. The backend must meet the edge-triggered contract documented in
  /// `lion_reactor::os`.
  pub fn os_backend(mut self, backend: Box<dyn OsBackend>) -> Self {
  self.backend = Some(backend);
  self
  }

  pub fn event_interval(mut self, interval: usize) -> Self {
  assert!(interval > 0, "event_interval must be > 0");
  self.config.event_interval = interval;
  self
  }

  pub fn build(self) -> std::io::Result<Runtime> {
  Runtime::with_config(self.config, self.backend)
  }
}

impl Default for RuntimeBuilder {
  fn default() -> Self {
  Self::new()
  }
}

verus! {

fn main() {
}

}
