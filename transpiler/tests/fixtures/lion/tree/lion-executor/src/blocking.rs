use std::collections::VecDeque;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::thread;
use crate::types::JoinHandle;
use crate::types::join_handle::JoinError;

struct Task {
  f: Box<dyn FnOnce() + Send>,
}

struct Worker {
  queue: Arc<Mutex<VecDeque<Task>>>,
  thread: thread::Thread,
}

// Shared by every thread that calls spawn_blocking (it lives in a static), so
// the round-robin cursor is atomic and the pool is Sync without an unsafe impl.
pub struct BlockingPool {
  workers: Vec<Worker>,
  next: AtomicUsize,
}

impl BlockingPool {
  pub fn new(num_threads: usize) -> Self {
    let mut workers = Vec::with_capacity(num_threads);

    for _ in 0..num_threads {
      let queue = Arc::new(Mutex::new(VecDeque::<Task>::new()));
      let queue_clone = queue.clone();

      let h = thread::spawn(move || {
        loop {
          loop {
            let task = queue_clone.lock().unwrap().pop_front();
            match task {
              // The closure contains its own panic (below); this catch keeps
              // the worker alive even if resolving the handle panics.
              Some(t) => {
                let _ = catch_unwind(AssertUnwindSafe(t.f));
              }
              None => break,
            }
          }
          thread::park();
        }
      });

      workers.push(Worker {
        queue,
        thread: h.thread().clone(),
      });

      std::mem::forget(h);
    }

    BlockingPool {
      workers,
      next: AtomicUsize::new(0),
    }
  }

  pub fn spawn<F, R>(&self, f: F) -> JoinHandle<R>
  where
    F: FnOnce() -> R + Send + 'static,
    R: Send + 'static,
  {
    let (handle, sender) = JoinHandle::new();
    // A handle aborted before the closure starts cancels it; a panic in the
    // closure resolves the handle with the payload instead of killing the
    // worker thread.
    let task = Task {
      f: Box::new(move || {
        if sender.is_cancelled() {
          sender.finish(Err(JoinError::cancelled()));
          return;
        }
        match catch_unwind(AssertUnwindSafe(f)) {
          Ok(value) => sender.complete(value),
          Err(payload) => sender.finish(Err(JoinError::panic(payload))),
        }
      }),
    };

    let n = self.workers.len();
    let idx = self.next.fetch_add(1, Ordering::Relaxed) % n;

    let worker = &self.workers[idx];
    worker.queue.lock().unwrap().push_back(task);
    worker.thread.unpark();

    handle
  }
}

static GLOBAL: OnceLock<BlockingPool> = OnceLock::new();

pub fn spawn_blocking<F, R>(f: F) -> JoinHandle<R>
where
  F: FnOnce() -> R + Send + 'static,
  R: Send + 'static,
{
  GLOBAL
    .get_or_init(|| {
      let n = std::env::var("LION_BLOCKING_THREADS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or_else(|| {
          std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(4)
        });
      BlockingPool::new(n)
    })
    .spawn(f)
}
