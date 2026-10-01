// Trusted glue (plain Rust, invisible to Verus): the join channel between a
// task and its JoinHandle, and the task's cancellation flag.
//
// Every lock here is entered even if poisoned. No critical section runs user
// code: wakers are cloned under the lock but always woken after releasing it.

use std::any::Any;
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::task::{Context, Poll, Waker};

/// Why a task finished without producing its output: it was cancelled
/// (`JoinHandle::abort`, or its runtime was dropped first), or it panicked.
pub struct JoinError {
  repr: Repr,
}

enum Repr {
  Cancelled,
  // The Mutex makes JoinError Sync (the payload is only Send), which
  // `std::io::Error::other` requires.
  Panic(Mutex<Box<dyn Any + Send + 'static>>),
}

impl JoinError {
  /// A cancellation error. Public so adapters can map another runtime's join
  /// errors onto Lion's.
  pub fn cancelled() -> Self {
    JoinError { repr: Repr::Cancelled }
  }

  /// A panic error carrying `payload`, as returned by `std::panic::catch_unwind`.
  pub fn panic(payload: Box<dyn Any + Send + 'static>) -> Self {
    JoinError { repr: Repr::Panic(Mutex::new(payload)) }
  }

  pub fn is_cancelled(&self) -> bool {
    matches!(self.repr, Repr::Cancelled)
  }

  pub fn is_panic(&self) -> bool {
    matches!(self.repr, Repr::Panic(_))
  }

  /// The panic payload. Panics if the task was cancelled rather than panicked.
  pub fn into_panic(self) -> Box<dyn Any + Send + 'static> {
    self.try_into_panic().expect("`JoinError` reason is not a panic.")
  }

  pub fn try_into_panic(self) -> Result<Box<dyn Any + Send + 'static>, JoinError> {
    match self.repr {
      Repr::Panic(p) => Ok(p.into_inner().unwrap_or_else(PoisonError::into_inner)),
      repr => Err(JoinError { repr }),
    }
  }

  fn panic_message(&self) -> Option<String> {
    match &self.repr {
      Repr::Cancelled => None,
      Repr::Panic(p) => {
        let p = p.lock().unwrap_or_else(PoisonError::into_inner);
        if let Some(s) = p.downcast_ref::<&'static str>() {
          Some((*s).to_string())
        } else {
          p.downcast_ref::<String>().cloned()
        }
      }
    }
  }
}

impl std::fmt::Display for JoinError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match (&self.repr, self.panic_message()) {
      (Repr::Cancelled, _) => write!(f, "task was cancelled"),
      (Repr::Panic(_), Some(msg)) => write!(f, "task panicked with message {:?}", msg),
      (Repr::Panic(_), None) => write!(f, "task panicked"),
    }
  }
}

impl std::fmt::Debug for JoinError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match (&self.repr, self.panic_message()) {
      (Repr::Cancelled, _) => write!(f, "JoinError::Cancelled"),
      (Repr::Panic(_), Some(msg)) => write!(f, "JoinError::Panic({:?}, ...)", msg),
      (Repr::Panic(_), None) => write!(f, "JoinError::Panic(...)"),
    }
  }
}

impl std::error::Error for JoinError {}

impl From<JoinError> for std::io::Error {
  fn from(e: JoinError) -> Self {
    std::io::Error::other(e)
  }
}

// State shared by a task (through its JoinSender) and its JoinHandle.
struct JoinShared<T> {
  // Set by JoinHandle::abort, read by the task wrapper before every poll.
  cancelled: AtomicBool,
  state: Mutex<JoinState<T>>,
}

struct JoinState<T> {
  result: Option<Result<T, JoinError>>,
  // The waker of whoever awaits the JoinHandle.
  join_waker: Option<Waker>,
  // The task's own waker, registered at its first poll, so that abort can get
  // the task polled again. Cleared when the task finishes.
  task_waker: Option<Waker>,
}

fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
  m.lock().unwrap_or_else(PoisonError::into_inner)
}

pub struct JoinHandle<T> {
  shared: Arc<JoinShared<T>>,
}

impl<T> JoinHandle<T> {
  /// Cancels the task. Its future is dropped on the runtime's thread the next
  /// time the executor reaches the task (abort wakes it), and awaiting this
  /// handle then yields a cancelled `JoinError`. A task that has already
  /// finished keeps its result. A `spawn_blocking` closure that has already
  /// started runs to completion; one that has not started never runs.
  pub fn abort(&self) {
    self.shared.cancelled.store(true, Ordering::SeqCst);
    let task_waker = lock(&self.shared.state).task_waker.take();
    if let Some(w) = task_waker {
      w.wake();
    }
  }

  pub fn new() -> (Self, JoinSender<T>) {
    let shared = Arc::new(JoinShared {
      cancelled: AtomicBool::new(false),
      state: Mutex::new(JoinState { result: None, join_waker: None, task_waker: None }),
    });
    let handle = JoinHandle { shared: shared.clone() };
    let sender = JoinSender { shared, done: false };
    (handle, sender)
  }
}

impl<T> Future for JoinHandle<T> {
  type Output = Result<T, JoinError>;

  fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<T, JoinError>> {
    let mut state = lock(&self.shared.state);
    if let Some(result) = state.result.take() {
      Poll::Ready(result)
    } else {
      match &state.join_waker {
        Some(w) if w.will_wake(cx.waker()) => {}
        _ => state.join_waker = Some(cx.waker().clone()),
      }
      Poll::Pending
    }
  }
}

/// The task side of a JoinHandle. Dropping it without completing resolves the
/// handle as cancelled, so an awaiting handle never hangs on a task that is
/// gone (its runtime was dropped, or its future was discarded).
pub struct JoinSender<T> {
  shared: Arc<JoinShared<T>>,
  done: bool,
}

impl<T> JoinSender<T> {
  pub fn complete(self, result: T) {
    self.finish(Ok(result));
  }

  pub(crate) fn finish(mut self, result: Result<T, JoinError>) {
    self.publish(result);
  }

  pub(crate) fn is_cancelled(&self) -> bool {
    self.shared.cancelled.load(Ordering::SeqCst)
  }

  pub(crate) fn register_task_waker(&self, waker: &Waker) {
    lock(&self.shared.state).task_waker = Some(waker.clone());
  }

  // `done` is set before the waker runs, so a panicking waker cannot make
  // Drop publish a second time.
  fn publish(&mut self, result: Result<T, JoinError>) {
    self.done = true;
    let (join_waker, task_waker) = {
      let mut state = lock(&self.shared.state);
      state.result = Some(result);
      (state.join_waker.take(), state.task_waker.take())
    };
    drop(task_waker);
    if let Some(w) = join_waker {
      w.wake();
    }
  }
}

impl<T> Drop for JoinSender<T> {
  fn drop(&mut self) {
    if !self.done {
      self.publish(Err(JoinError::cancelled()));
    }
  }
}
