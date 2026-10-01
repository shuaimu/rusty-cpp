#![cfg_attr(verus_keep_ghost, verus::trusted)]

use std::sync::Arc;
use std::task::{Wake, Waker};
use super::TaskId;
use crate::tls;
use vstd::prelude::*;

verus! {

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum WakeSource {
  Reactor,
  Task,
}

} // end verus!

// The waker the reactor stores for a task's io wait (lion-utility's TCP and UDP
// glue creates it while the task is being polled). It is the task's
// ExecutorWaker with source Reactor: woken on the owner thread, which is where
// the reactor wakes it, it pushes the task onto the reactor-ready queue, as the
// former hand-written vtable waker did. Woken on any other thread it goes
// through the cross-thread queue and interrupts the park; the vtable waker
// pushed onto the calling thread's own TLS queue instead, where no executor
// looks.
pub fn create_reactor_waker_for_current() -> Waker {
  let task_id = tls::get_current_task()
    .expect("create_reactor_waker_for_current called outside task context");
  create_waker(task_id, WakeSource::Reactor, false)
}

pub(crate) fn create_waker(task_id: TaskId, source: WakeSource, defer: bool) -> Waker {
  let (queue, interrupt, thread_id) = tls::get_cross_thread_ctx();
  Waker::from(ExecutorWaker::new(task_id, source, defer, queue, interrupt, thread_id))
}

pub struct ExecutorWaker {
  task_id: TaskId,
  source: WakeSource,
  defer: bool,
  cross_thread_queue: Arc<tls::CrossThreadQueue>,
  interrupt: lion_reactor::InterruptHandle,
  executor_thread_id: std::thread::ThreadId,
}

impl ExecutorWaker {
  pub(crate) fn new(
    task_id: TaskId,
    source: WakeSource,
    defer: bool,
    cross_thread_queue: Arc<tls::CrossThreadQueue>,
    interrupt: lion_reactor::InterruptHandle,
    executor_thread_id: std::thread::ThreadId,
  ) -> Arc<Self> {
    Arc::new(Self { task_id, source, defer, cross_thread_queue, interrupt, executor_thread_id })
  }

  fn wake_impl(&self) {
    if std::thread::current().id() == self.executor_thread_id {
      if self.defer {
        tls::push_deferred(self.task_id);
      } else {
        match self.source {
          WakeSource::Reactor => tls::push_reactor_ready(self.task_id),
          WakeSource::Task => tls::push_task_ready(self.task_id),
        }
      }
    } else {
      self.cross_thread_queue.push(self.task_id);
      self.interrupt.wake();
    }
  }

}

impl Wake for ExecutorWaker {
  fn wake(self: Arc<Self>) {
    self.wake_impl();
  }

  fn wake_by_ref(self: &Arc<Self>) {
    self.wake_impl();
  }
}
