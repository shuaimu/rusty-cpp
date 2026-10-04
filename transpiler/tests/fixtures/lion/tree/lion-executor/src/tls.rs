#![cfg_attr(verus_keep_ghost, verus::trusted)]

use std::cell::{Cell, RefCell};
use std::collections::HashSet;
use std::sync::{Arc, Mutex, PoisonError};
use crate::collections::VecDeque;
use crate::types::TaskId;
use lion_reactor::InterruptHandle;

thread_local! {
  static REACTOR_READY_QUEUE: RefCell<VecDeque<TaskId>> = RefCell::new(VecDeque::new());
  static TASK_READY_QUEUE: RefCell<VecDeque<TaskId>> = RefCell::new(VecDeque::new());
  static DEFER_QUEUE: RefCell<VecDeque<TaskId>> = RefCell::new(VecDeque::new());
  static CURRENT_TASK: Cell<Option<TaskId>> = const { Cell::new(None) };
  // Ids of tasks that are queued (woken) but not yet polled; dedups wakes.
  // Holds queued tasks only: `clear_notified` (at the start of every poll,
  // including the Invalid poll of a finished task's stale wake) removes the id.
  // Task ids are never reused, so a table indexed by id would grow with every
  // task ever spawned.
  static TASK_NOTIFIED: RefCell<HashSet<u64>> = RefCell::new(HashSet::new());
  static BLOCK_ON_YIELDED: Cell<bool> = const { Cell::new(false) };
}

pub(crate) fn set_block_on_yielded(v: bool) {
  BLOCK_ON_YIELDED.with(|c| c.set(v));
}

pub(crate) fn take_block_on_yielded() -> bool {
  BLOCK_ON_YIELDED.with(|c| {
    let v = c.get();
    c.set(false);
    v
  })
}

fn set_notified(task_id: TaskId) -> bool {
  if task_id.0 == u64::MAX {
    return true;
  }
  TASK_NOTIFIED.with(|n| n.borrow_mut().insert(task_id.0))
}

pub(crate) fn clear_notified(task_id: TaskId) {
  TASK_NOTIFIED.with(|n| {
    n.borrow_mut().remove(&task_id.0);
  });
}

pub(crate) fn push_reactor_ready(task_id: TaskId) {
  if set_notified(task_id) {
    REACTOR_READY_QUEUE.with(|q| {
      q.borrow_mut().push_back(task_id);
    });
  }
}

pub(crate) fn push_task_ready(task_id: TaskId) {
  if set_notified(task_id) {
    TASK_READY_QUEUE.with(|q| {
      q.borrow_mut().push_back(task_id);
    });
  }
}

pub(crate) fn push_deferred(task_id: TaskId) {
  DEFER_QUEUE.with(|q| {
  q.borrow_mut().push_back(task_id);
  });
}


pub(crate) fn take_reactor_ready() -> VecDeque<TaskId> {
  REACTOR_READY_QUEUE.with(|q| {
  std::mem::take(&mut *q.borrow_mut())
  })
}

pub(crate) fn take_task_ready() -> VecDeque<TaskId> {
  TASK_READY_QUEUE.with(|q| {
  std::mem::take(&mut *q.borrow_mut())
  })
}

pub(crate) fn take_deferred() -> VecDeque<TaskId> {
  DEFER_QUEUE.with(|q| {
  std::mem::take(&mut *q.borrow_mut())
  })
}

pub fn set_current_task(task_id: TaskId) {
  CURRENT_TASK.with(|c| c.set(Some(task_id)));
}

pub fn clear_current_task() {
  CURRENT_TASK.with(|c| c.set(None));
}

// Installs a current task for a scope and clears it on exit, including when
// the scope unwinds (a panicking block_on future must not leave its id behind).
pub(crate) struct CurrentTaskGuard;

impl CurrentTaskGuard {
  pub(crate) fn enter(task_id: TaskId) -> Self {
    set_current_task(task_id);
    CurrentTaskGuard
  }
}

impl Drop for CurrentTaskGuard {
  fn drop(&mut self) {
    clear_current_task();
  }
}

pub fn get_current_task() -> Option<TaskId> {
  CURRENT_TASK.with(|c| c.get())
}

pub fn defer_current() {
  CURRENT_TASK.with(|c| {
  if let Some(task_id) = c.get() {
    if task_id.0 == u64::MAX {
      set_block_on_yielded(true);
    }
    push_deferred(task_id);
  } else {
    panic!("defer_current called outside of task context");
  }
  });
}

pub(crate) fn has_deferred() -> bool {
  DEFER_QUEUE.with(|q| !q.borrow().is_empty())
}

pub(crate) fn has_reactor_ready() -> bool {
  REACTOR_READY_QUEUE.with(|q| !q.borrow().is_empty())
}

pub(crate) fn has_task_ready() -> bool {
  TASK_READY_QUEUE.with(|q| !q.borrow().is_empty())
}

pub(crate) fn drain_task_ready_into(target: &mut VecDeque<TaskId>) {
  TASK_READY_QUEUE.with(|q| {
    let mut source = q.borrow_mut();
    while let Some(task_id) = source.pop_front() {
      target.push_back(task_id);
    }
  });
}

pub(crate) fn drain_reactor_ready_into(target: &mut VecDeque<TaskId>) {
  REACTOR_READY_QUEUE.with(|q| {
    let mut source = q.borrow_mut();
    while let Some(task_id) = source.pop_front() {
      target.push_back(task_id);
    }
  });
}

pub(crate) fn drain_deferred_into(target: &mut VecDeque<TaskId>) {
  DEFER_QUEUE.with(|q| {
    let mut source = q.borrow_mut();
    while let Some(task_id) = source.pop_front() {
      target.push_back(task_id);
    }
  });
}

// Task ids woken from threads other than the owner. Any thread pushes; only
// the owner drains, taking the whole batch under the lock and queueing it
// after releasing it. A plain Mutex<VecDeque> (std's, not the verified
// crate::collections wrapper): cross-thread wakes are rare next to owner-thread
// ones, and nothing here panics while holding the lock, so it cannot be
// poisoned; a poisoned lock is still entered rather than lose wakes.
pub(crate) struct CrossThreadQueue {
  ids: Mutex<std::collections::VecDeque<TaskId>>,
}

impl CrossThreadQueue {
  pub fn new() -> Arc<Self> {
    Arc::new(Self { ids: Mutex::new(std::collections::VecDeque::new()) })
  }

  pub fn push(&self, task_id: TaskId) {
    self.ids.lock().unwrap_or_else(PoisonError::into_inner).push_back(task_id);
  }

  fn take_all(&self) -> std::collections::VecDeque<TaskId> {
    std::mem::take(&mut *self.ids.lock().unwrap_or_else(PoisonError::into_inner))
  }
}

thread_local! {
  static CROSS_THREAD_CTX: RefCell<Option<(Arc<CrossThreadQueue>, InterruptHandle, std::thread::ThreadId)>> = RefCell::new(None);
}

pub(crate) fn set_cross_thread_ctx(queue: Arc<CrossThreadQueue>, interrupt: InterruptHandle, thread_id: std::thread::ThreadId) {
  CROSS_THREAD_CTX.with(|c| *c.borrow_mut() = Some((queue, interrupt, thread_id)));
}

pub(crate) fn get_cross_thread_ctx() -> (Arc<CrossThreadQueue>, InterruptHandle, std::thread::ThreadId) {
  CROSS_THREAD_CTX.with(|c| c.borrow().as_ref().expect("cross-thread context not set").clone())
}

pub(crate) fn drain_cross_thread() {
  let batch = CROSS_THREAD_CTX.with(|c| c.borrow().as_ref().map(|(queue, _, _)| queue.take_all()));
  for tid in batch.into_iter().flatten() {
    push_task_ready(tid);
  }
}

pub(crate) fn reset_interrupt() {
  CROSS_THREAD_CTX.with(|c| {
    if let Some((_, interrupt, _)) = c.borrow().as_ref() {
      interrupt.reset();
    }
  });
}

pub(crate) fn has_cross_thread_ctx() -> bool {
  CROSS_THREAD_CTX.try_with(|c| c.borrow().is_some()).unwrap_or(true)
}

// Runtime drop: if this thread's cross-thread context is still `queue`'s (the
// dropping runtime's), clear it and reset the per-thread executor state that
// runtime left behind (queued wake ids, the notified set, the current task and
// the block_on yield flag), so a later runtime on this thread starts clean.
// Task ids restart at 1 in every runtime, so a stale id would otherwise alias
// a new task. Anything else is another runtime's state and is left alone.
pub(crate) fn release_runtime_thread_state(queue: &Arc<CrossThreadQueue>) {
  let ours = CROSS_THREAD_CTX
    .try_with(|c| {
      let mut c = c.borrow_mut();
      let ours = c.as_ref().is_some_and(|(q, _, _)| Arc::ptr_eq(q, queue));
      if ours {
        *c = None;
      }
      ours
    })
    .unwrap_or(false);
  if !ours {
    return;
  }
  let _ = REACTOR_READY_QUEUE.try_with(|q| q.borrow_mut().clear());
  let _ = TASK_READY_QUEUE.try_with(|q| q.borrow_mut().clear());
  let _ = DEFER_QUEUE.try_with(|q| q.borrow_mut().clear());
  let _ = TASK_NOTIFIED.try_with(|n| n.borrow_mut().clear());
  let _ = CURRENT_TASK.try_with(|c| c.set(None));
  let _ = BLOCK_ON_YIELDED.try_with(|c| c.set(false));
}
