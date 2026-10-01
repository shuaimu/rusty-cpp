#![feature(prelude_import)]
#![allow(unused_imports)]
#![allow(unused_braces)]
#![allow(unused_variables)]
#![allow(unused_assignments)]
#![allow(dead_code)]
#![allow(unused_mut)]
extern crate std;
#[prelude_import]
use std::prelude::rust_2021::*;

use vstd::prelude::*;

mod collections {

    // Field order is drop order: the executor (its tasks and the injection
    // receiver) goes first, while the reactor guard is still in place.
    // Last: releases this runtime's thread-local registrations after everything
    // above is gone, so task destructors that spawn or wake still find it.
    // !Send: the executor may hold spawn_local tasks, which must be dropped on
    // this thread (see types/boxed_future.rs, OwnerThreadOnly).

    // What a Runtime installed in its thread's TLS, released on drop. Each entry is
    // cleared only if it is still this runtime's; the per-thread wake queues are
    // reset only if the cross-thread context was, i.e. if this was the thread's
    // runtime. Uses try_with throughout: a Runtime kept in a thread_local may drop
    // while the thread's other TLS is being destroyed.

    // Whether this thread already has a live Lion runtime (or a bare lion-reactor
    // Reactor entered). A thread whose TLS is being torn down counts as busy.

    // Before anything is created or installed: a refused Runtime::new leaves
    // this thread's runtime, and its TLS, untouched.

    // Every tick sets the idle park bound it wants, so no caller inherits
    // another's.

    // The reactor of a new runtime: over the embedder's backend if one was given,
    // else over Lion's mio backend (feature `mio`).

    mod vec_deque {
        use std::collections::VecDeque as StdVecDeque;
        use vstd::prelude::*;
        use vstd::seq::Seq;
        pub struct VecDeque<T> {
            inner: StdVecDeque<T>,
        }
        impl<T: View> View for VecDeque<T> {
            type V = Seq<T::V>;
        }
        impl<T: View> VecDeque<T> {
            pub fn new() -> Self {
                VecDeque {
                    inner: StdVecDeque::new(),
                }
            }
            pub fn push_back(&mut self, value: T) {
                self.inner.push_back(value)
            }
            pub fn push_front(&mut self, value: T) {
                self.inner.push_front(value)
            }
            pub fn pop_front(&mut self) -> Option<T> {
                self.inner.pop_front()
            }
            pub fn pop_back(&mut self) -> Option<T> {
                self.inner.pop_back()
            }
            pub fn is_empty(&self) -> bool {
                self.inner.is_empty()
            }
            pub fn len(&self) -> usize {
                self.inner.len()
            }
            pub fn clear(&mut self) {
                self.inner.clear()
            }
        }
        impl<T: View> Default for VecDeque<T> {
            fn default() -> Self {
                VecDeque {
                    inner: StdVecDeque::new(),
                }
            }
        }
        impl<T> From<VecDeque<T>> for Vec<T> {
            fn from(deque: VecDeque<T>) -> Self {
                deque.inner.into()
            }
        }
    }
    mod mpsc_queue {
        use std::sync::mpsc::{channel, Receiver, Sender};
        use vstd::prelude::*;
        pub struct MpscSender<T> {
            sender: Sender<T>,
        }
        pub struct MpscReceiver<T> {
            receiver: Receiver<T>,
        }
        pub fn mpsc_queue<T>() -> (MpscSender<T>, MpscReceiver<T>) {
            let (sender, receiver) = channel();
            (MpscSender { sender }, MpscReceiver { receiver })
        }
        impl<T> MpscSender<T> {
            pub fn send(&self, item: T) -> bool {
                self.sender.send(item).is_ok()
            }
        }
        impl<T> Clone for MpscSender<T> {
            fn clone(&self) -> Self {
                Self {
                    sender: self.sender.clone(),
                }
            }
        }
        impl<T> MpscReceiver<T> {
            pub fn try_recv(&self) -> Option<T> {
                self.receiver.try_recv().ok()
            }
        }
    }
    mod task_slab {
        use crate::types::Task;
        pub type TaskSlab = lion_slab::Slab<Task>;
    }
    mod tid_ledger {
        use crate::types::TaskId;
        use lion_executor_spec::types::TID;
        use vstd::prelude::*;
        pub struct TidLedger {
            pub words: Vec<u64>,
        }
        impl TidLedger {
            pub fn new() -> Self {
                TidLedger { words: Vec::new() }
            }
            pub fn contains(&self, tid: TaskId) -> bool {
                let w64 = tid.0 / 64;
                {}
                if w64 >= self.words.len() as u64 {
                    return false;
                }
                let w = w64 as usize;
                let bit = (self.words[w] >> (tid.0 % 64)) & 1;
                bit == 1
            }
            pub fn mark(&mut self, tid: TaskId) {
                let w64 = tid.0 / 64;
                let b = tid.0 % 64;
                {}
                while (self.words.len() as u64) <= w64 {
                    self.words.push(0);
                }
                let w = w64 as usize;
                {}
                let word = self.words[w];
                let updated = word | (1u64 << b);
                self.words.set(w, updated);
                {}
            }
        }
    }
    pub use vec_deque::VecDeque;
    pub use mpsc_queue::{mpsc_queue, MpscSender, MpscReceiver};
    pub use task_slab::TaskSlab;
    pub use tid_ledger::TidLedger;
}
mod config {
    use vstd::prelude::*;
    pub struct RuntimeConfig {
        pub event_interval: usize,
    }
    #[automatically_derived]
    impl ::core::fmt::Debug for RuntimeConfig {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::debug_struct_field1_finish(
                f,
                "RuntimeConfig",
                "event_interval",
                &&self.event_interval,
            )
        }
    }
    #[automatically_derived]
    #[doc(hidden)]
    unsafe impl ::core::clone::TrivialClone for RuntimeConfig {}
    #[automatically_derived]
    impl ::core::clone::Clone for RuntimeConfig {
        #[inline]
        fn clone(&self) -> RuntimeConfig {
            let _: ::core::clone::AssertParamIsClone<usize>;
            *self
        }
    }
    #[automatically_derived]
    impl ::core::marker::Copy for RuntimeConfig {}
    impl Default for RuntimeConfig {
        fn default() -> Self {
            Self {
                event_interval: 128,
            }
        }
    }
}
mod executor {
    mod new {
        use lion_reactor::Reactor as LionReactor;
        use crate::collections::{VecDeque, MpscReceiver, TaskSlab, TidLedger};
        use crate::config::RuntimeConfig;
        use crate::proof::invariants::*;
        use crate::types::{Reactor, Task};
        use super::Executor;
        use vstd::prelude::*;
        impl Executor {
            pub fn new(
                reactor: LionReactor,
                injection_queue: MpscReceiver<Task>,
                config: RuntimeConfig,
            ) -> Self {
                let result = Executor {
                    task_slab: TaskSlab::new(),
                    local_queue: VecDeque::new(),
                    injection_queue,
                    reactor: Reactor::new(reactor),
                    event_interval: config.event_interval,
                    log: Ghost::assume_new_fallback(|| {
                        ::core::panicking::panic("internal error: entered unreachable code")
                    }),
                    ledger: TidLedger::new(),
                };
                {}
                result
            }
        }
    }
    mod enter {
        use crate::types::ReactorGuard;
        use super::Executor;
        use vstd::prelude::*;
        impl Executor {
            pub fn enter(&mut self) -> ReactorGuard {
                self.reactor.enter()
            }
        }
    }
    mod tick {
        use super::Executor;
        use crate::spec::log::*;
        use crate::spec::fifo_queue::*;
        use crate::proof::invariants::*;
        use crate::proof::preservation::*;
        use crate::proof::helpers::*;
        use crate::proof::fifo_helpers::*;
        use crate::types::PollResult;
        use vstd::prelude::*;
        impl Executor {
            pub fn pop_injection(&mut self) {
                let task_opt = self.pop_injection_action();
                {}
                if let Some(task) = task_opt {
                    let task_id = task.id();
                    self.task_slab.insert(task_id.0, task);
                    self.local_queue.push_back(task_id);
                    {}
                } else {
                    {}
                    return;
                }
                loop {
                    let task_opt = self.pop_injection_action();
                    {}
                    if let Some(task) = task_opt {
                        let task_id = task.id();
                        self.task_slab.insert(task_id.0, task);
                        self.local_queue.push_back(task_id);
                        {}
                    } else {
                        {}
                        break;
                    }
                }
            }
            fn poll_loop(&mut self, event_interval: usize, count: &mut usize) {
                if *count >= event_interval {
                    return;
                }
                match self.next_task() {
                    Some(task_id) => {
                        {}
                        self.poll_task(task_id);
                        {}
                        *count += 1;
                    }
                    None => {
                        return;
                    }
                }
                {}
                while *count < event_interval {
                    {}
                    match self.next_task() {
                        Some(task_id) => {
                            {}
                            self.poll_task(task_id);
                            {}
                        }
                        None => {
                            {}
                            break;
                        }
                    }
                    *count += 1;
                }
            }
            pub fn tick(&mut self) {
                let event_interval = self.event_interval;
                {}
                self.tick_begin_action();
                {}
                self.wake_deferred();
                {}
                self.pop_injection();
                {}
                let mut count: usize = 0;
                self.poll_loop(event_interval, &mut count);
                {}
                self.park();
                {}
                self.poll_loop(event_interval, &mut count);
                {}
                self.tick_end_action();
                {}
            }
        }
    }
    mod next_task {
        use crate::types::{TaskId, TaskView, TID};
        use crate::spec::log::*;
        use crate::spec::fifo_queue::*;
        use crate::proof::invariants::*;
        use crate::proof::helpers::*;
        use crate::proof::fifo_helpers::*;
        use super::Executor;
        use vstd::prelude::*;
        impl Executor {
            pub fn next_task(&mut self) -> Option<TaskId> {
                if let Some(task_id) = self.local_queue.pop_front() {
                    {}
                    return Some(task_id);
                }
                self.drain_reactor_ready_into_local();
                {}
                self.drain_task_ready_into_local();
                {}
                {}
                if let Some(task_id) = self.local_queue.pop_front() {
                    {}
                    return Some(task_id);
                }
                let pop_result = self.pop_injection_action();
                {}
                if let Some(task) = pop_result {
                    let task_id = task.id();
                    self.task_slab.insert(task_id.0, task);
                    self.local_queue.push_back(task_id);
                    {}
                    let result = self.local_queue.pop_front();
                    {}
                    return result;
                }
                {}
                None
            }
        }
    }
    mod poll_task {
        use crate::types::{TaskId, PollResult, Task, TID, TaskView};
        use crate::spec::log::*;
        use crate::proof::invariants::*;
        use crate::proof::helpers::*;
        use crate::tls;
        use super::Executor;
        use vstd::prelude::*;
        fn clear_task_notified(task_id: TaskId) {
            tls::clear_notified(task_id);
        }
        impl Executor {
            pub fn poll_task(&mut self, task_id: TaskId) {
                clear_task_notified(task_id);
                match self.task_slab.remove(task_id.0) {
                    Some(task) => {
                        {}
                        let (result, task_back) = self.poll_task_action(task_id, Some(task));
                        {}
                        match result {
                            PollResult::Ready(()) => {}
                            _ => {
                                let task_to_insert = task_back.unwrap();
                                self.task_slab.insert(task_id.0, task_to_insert);
                                {}
                            }
                        }
                    }
                    None => {
                        {}
                        self.poll_task_invalid_action(task_id);
                        {}
                    }
                }
            }
        }
    }
    mod park {
        use crate::types::TaskId;
        use crate::spec::log::*;
        use crate::spec::fifo_queue::*;
        use crate::proof::invariants::*;
        use crate::proof::helpers::*;
        use crate::proof::fifo_helpers::*;
        use super::Executor;
        use vstd::prelude::*;
        impl Executor {
            pub fn park(&mut self) {
                let has_deferred = self.has_deferred_action();
                let has_reactor_ready = self.has_reactor_ready_action();
                self.reset_and_drain_cross_thread_action();
                let has_task_ready = self.has_task_ready_action();
                let has_local_tasks = self.local_queue.len() > 0;
                let block_on_yielded = self.take_block_on_yielded_action();
                let require_timeout = !block_on_yielded
                    && !has_deferred
                    && !has_reactor_ready
                    && !has_task_ready
                    && !has_local_tasks;
                self.park_action(require_timeout);
                {}
                self.drain_reactor_ready_into_local();
                {}
                self.drain_task_ready_into_local();
                {}
            }
        }
    }
    mod wake_deferred {
        use crate::types::TaskId;
        use crate::spec::log::*;
        use crate::spec::fifo_queue::*;
        use crate::proof::invariants::*;
        use crate::proof::helpers::*;
        use crate::proof::fifo_helpers::*;
        use super::Executor;
        use vstd::prelude::*;
        impl Executor {
            pub fn wake_deferred(&mut self) {
                self.drain_deferred_into_local();
                {}
            }
        }
    }
    mod ext {
        use std::panic::{catch_unwind, AssertUnwindSafe};
        use std::task::Context;
        use crate::tls;
        use crate::types::{
            create_waker, Duration, Instant, PollResult, Task, TaskId, TID, TaskView, WakeSource,
        };
        use crate::spec::log::*;
        use crate::proof::invariants::*;
        use crate::proof::helpers::*;
        use super::Executor;
        use vstd::prelude::*;
        impl Executor {
            pub fn tick_begin_action(&mut self) {}
            pub fn tick_end_action(&mut self) {}
            fn log_poll_task_action(
                &mut self,
                task_id: TaskId,
                task: &Option<Task>,
                result: &PollResult<()>,
            ) {
            }
            fn log_pop_injection_action(&mut self, task: &Option<Task>) {}
            fn log_drain_task_wake_action(&mut self, ids: &Vec<TaskId>) {}
            fn log_drain_reactor_wake_action(&mut self, ids: &Vec<TaskId>) {}
            fn log_drain_deferred_action(&mut self, ids: &Vec<TaskId>) {}
        }
        fn poll_task_contained(task_id: TaskId, task: &mut Task) -> PollResult<()> {
            tls::set_current_task(task_id);
            let waker = create_waker(task_id, WakeSource::Task, false);
            let mut context = Context::from_waker(&waker);
            let result = match catch_unwind(AssertUnwindSafe(|| task.poll(&mut context))) {
                Ok(poll) => PollResult::from(poll),
                Err(_) => PollResult::Ready(()),
            };
            tls::clear_current_task();
            result
        }
        impl Executor {
            fn poll_future_raw(
                task_id: TaskId,
                task: Option<Task>,
            ) -> (PollResult<()>, Option<Task>) {
                match task {
                    Some(mut t) => {
                        let poll_result = poll_task_contained(task_id, &mut t);
                        (poll_result, Some(t))
                    }
                    None => (PollResult::Pending, None),
                }
            }
            pub fn poll_task_action(
                &mut self,
                task_id: TaskId,
                task: Option<Task>,
            ) -> (PollResult<()>, Option<Task>) {
                let has_task = task.is_some();
                let (result, task_back) = Self::poll_future_raw(task_id, task);
                self.log_poll_task_action(task_id, &task_back, &result);
                (result, task_back)
            }
            pub fn poll_task_invalid_action(&mut self, task_id: TaskId) {}
            fn try_recv_raw(rx: &crate::collections::MpscReceiver<Task>) -> Option<Task> {
                rx.try_recv()
            }
            pub fn pop_injection_action(&mut self) -> Option<Task> {
                let raw = Self::try_recv_raw(&self.injection_queue);
                match raw {
                    None => {
                        let none: Option<Task> = None;
                        self.log_pop_injection_action(&none);
                        {}
                        None
                    }
                    Some(task) => {
                        let task_id = task.id();
                        if self.ledger.contains(task_id) {
                            let none: Option<Task> = None;
                            self.log_pop_injection_action(&none);
                            {}
                            None
                        } else {
                            self.ledger.mark(task_id);
                            let some_task = Some(task);
                            self.log_pop_injection_action(&some_task);
                            {}
                            some_task
                        }
                    }
                }
            }
            pub fn park_action(&mut self, require_timeout: bool) {
                let timeout = if require_timeout {
                    Some(Duration::from_millis(self.reactor.idle_park_ms()))
                } else {
                    Some(Duration::zero())
                };
                self.reactor.flush_pending_deregister();
                self.reactor.park(timeout);
            }
            pub fn reset_and_drain_cross_thread_action(&self) {
                tls::reset_interrupt();
                tls::drain_cross_thread();
            }
            pub fn has_deferred_action(&self) -> bool {
                tls::has_deferred()
            }
            pub fn has_reactor_ready_action(&self) -> bool {
                tls::has_reactor_ready()
            }
            pub fn has_task_ready_action(&self) -> bool {
                tls::has_task_ready()
            }
            pub fn take_block_on_yielded_action(&self) -> bool {
                tls::take_block_on_yielded()
            }
            fn take_task_ready_from_tls() -> Vec<TaskId> {
                tls::drain_cross_thread();
                tls::take_task_ready().into()
            }
            fn take_reactor_ready_from_tls() -> Vec<TaskId> {
                tls::take_reactor_ready().into()
            }
            fn take_deferred_from_tls() -> Vec<TaskId> {
                tls::take_deferred().into()
            }
            fn filter_and_enqueue(&mut self, ids: Vec<TaskId>) -> Vec<TaskId> {
                let mut kept: Vec<TaskId> = Vec::new();
                let mut i: usize = 0;
                while i < ids.len() {
                    let t = ids[i];
                    if self.ledger.contains(t) {
                        kept.push(t);
                        self.local_queue.push_back(t);
                        {}
                    }
                    i = i + 1;
                }
                kept
            }
            pub fn drain_task_ready_into_local(&mut self) {
                let ids = Self::take_task_ready_from_tls();
                let kept = self.filter_and_enqueue(ids);
                self.log_drain_task_wake_action(&kept);
                {}
            }
            pub fn drain_reactor_ready_into_local(&mut self) {
                let ids = Self::take_reactor_ready_from_tls();
                let kept = self.filter_and_enqueue(ids);
                self.log_drain_reactor_wake_action(&kept);
                {}
            }
            pub fn drain_deferred_into_local(&mut self) {
                let ids = Self::take_deferred_from_tls();
                let kept = self.filter_and_enqueue(ids);
                self.log_drain_deferred_action(&kept);
                {}
            }
        }
    }
    use crate::collections::{VecDeque, MpscReceiver, TaskSlab, TidLedger};
    use crate::spec::log::Log;
    use crate::types::{Reactor, Task, TaskId};
    use vstd::prelude::*;
    pub struct Executor {
        pub task_slab: TaskSlab,
        pub local_queue: VecDeque<TaskId>,
        pub injection_queue: MpscReceiver<Task>,
        pub reactor: Reactor,
        pub event_interval: usize,
        pub log: Ghost<Log>,
        pub ledger: TidLedger,
    }
}
mod framework {
    pub mod action_safety {
        #[allow(unused_imports)]
        pub use lion_framework_spec::action_safety::*;
    }
    pub mod local_liveness {
        #[allow(unused_imports)]
        pub use lion_framework_spec::local_liveness::*;
    }
}
mod spec {
    pub mod log {
        #[allow(unused_imports)]
        pub use lion_executor_spec::events::*;
        #[allow(unused_imports)]
        pub use lion_executor_spec::log::*;
    }
    pub mod fifo_queue {
        #[allow(unused_imports)]
        pub use lion_executor_spec::fifo_queue::*;
    }
}
mod proof {
    pub mod invariants {
        use vstd::prelude::*;
        use crate::spec::log::*;
        use crate::spec::fifo_queue::*;
        use crate::types::{TID, TaskView, PollResult};
        use crate::framework::action_safety::*;
        use crate::framework::local_liveness::*;
    }
    pub mod preservation {
        use vstd::prelude::*;
        use crate::spec::log::*;
        use crate::spec::fifo_queue::*;
        use crate::proof::invariants::*;
        use crate::types::PollResult;
    }
    pub mod helpers {
        use vstd::prelude::*;
        use crate::spec::log::*;
        use crate::proof::invariants::*;
        use crate::types::{PollResult, TID, TaskView};
    }
    pub mod fifo_helpers {
        use vstd::prelude::*;
        use crate::spec::log::*;
        use crate::spec::fifo_queue::*;
        use crate::types::TID;
    }
}
pub mod types {
    mod duration {
        use lion_reactor::Duration as ReactorDuration;
        use vstd::prelude::*;
        pub struct Duration {
            inner: ReactorDuration,
        }
        #[automatically_derived]
        impl ::core::fmt::Debug for Duration {
            #[inline]
            fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
                ::core::fmt::Formatter::debug_struct_field1_finish(
                    f,
                    "Duration",
                    "inner",
                    &&self.inner,
                )
            }
        }
        #[automatically_derived]
        impl ::core::marker::Copy for Duration {}
        #[automatically_derived]
        #[doc(hidden)]
        unsafe impl ::core::clone::TrivialClone for Duration {}
        #[automatically_derived]
        impl ::core::clone::Clone for Duration {
            #[inline]
            fn clone(&self) -> Duration {
                let _: ::core::clone::AssertParamIsClone<ReactorDuration>;
                *self
            }
        }
        #[automatically_derived]
        impl ::core::marker::StructuralPartialEq for Duration {}
        #[automatically_derived]
        impl ::core::cmp::PartialEq for Duration {
            #[inline]
            fn eq(&self, other: &Duration) -> bool {
                self.inner == other.inner
            }
        }
        #[automatically_derived]
        impl ::core::cmp::Eq for Duration {
            #[inline]
            #[doc(hidden)]
            #[coverage(off)]
            fn assert_fields_are_eq(&self) {
                let _: ::core::cmp::AssertParamIsEq<ReactorDuration>;
            }
        }
        #[automatically_derived]
        impl ::core::cmp::PartialOrd for Duration {
            #[inline]
            fn partial_cmp(
                &self,
                other: &Duration,
            ) -> ::core::option::Option<::core::cmp::Ordering> {
                ::core::cmp::PartialOrd::partial_cmp(&self.inner, &other.inner)
            }
        }
        #[automatically_derived]
        impl ::core::cmp::Ord for Duration {
            #[inline]
            fn cmp(&self, other: &Duration) -> ::core::cmp::Ordering {
                ::core::cmp::Ord::cmp(&self.inner, &other.inner)
            }
        }
        #[automatically_derived]
        impl ::core::hash::Hash for Duration {
            #[inline]
            fn hash<__H: ::core::hash::Hasher>(&self, state: &mut __H) {
                ::core::hash::Hash::hash(&self.inner, state)
            }
        }
        impl View for Duration {
            type V = nat;
        }
        impl Duration {
            pub fn zero() -> Self {
                Duration {
                    inner: ReactorDuration::from_millis(0),
                }
            }
            pub fn from_millis(millis: u64) -> Self {
                Duration {
                    inner: ReactorDuration::from_millis(millis),
                }
            }
            pub fn from_secs(secs: u64) -> Self {
                Duration {
                    inner: ReactorDuration::from_secs(secs),
                }
            }
            pub fn as_millis(&self) -> u64 {
                self.inner.as_millis()
            }
            pub fn into_reactor(self) -> ReactorDuration {
                self.inner
            }
        }
        impl From<ReactorDuration> for Duration {
            fn from(d: ReactorDuration) -> Self {
                Duration { inner: d }
            }
        }
        impl From<Duration> for ReactorDuration {
            fn from(d: Duration) -> Self {
                d.inner
            }
        }
    }
    mod instant {
        use lion_reactor::Instant as ReactorInstant;
        use super::Duration;
        use vstd::prelude::*;
        pub struct Instant {
            inner: ReactorInstant,
        }
        #[automatically_derived]
        impl ::core::fmt::Debug for Instant {
            #[inline]
            fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
                ::core::fmt::Formatter::debug_struct_field1_finish(
                    f,
                    "Instant",
                    "inner",
                    &&self.inner,
                )
            }
        }
        #[automatically_derived]
        impl ::core::marker::Copy for Instant {}
        #[automatically_derived]
        #[doc(hidden)]
        unsafe impl ::core::clone::TrivialClone for Instant {}
        #[automatically_derived]
        impl ::core::clone::Clone for Instant {
            #[inline]
            fn clone(&self) -> Instant {
                let _: ::core::clone::AssertParamIsClone<ReactorInstant>;
                *self
            }
        }
        #[automatically_derived]
        impl ::core::marker::StructuralPartialEq for Instant {}
        #[automatically_derived]
        impl ::core::cmp::PartialEq for Instant {
            #[inline]
            fn eq(&self, other: &Instant) -> bool {
                self.inner == other.inner
            }
        }
        #[automatically_derived]
        impl ::core::cmp::Eq for Instant {
            #[inline]
            #[doc(hidden)]
            #[coverage(off)]
            fn assert_fields_are_eq(&self) {
                let _: ::core::cmp::AssertParamIsEq<ReactorInstant>;
            }
        }
        #[automatically_derived]
        impl ::core::cmp::PartialOrd for Instant {
            #[inline]
            fn partial_cmp(
                &self,
                other: &Instant,
            ) -> ::core::option::Option<::core::cmp::Ordering> {
                ::core::cmp::PartialOrd::partial_cmp(&self.inner, &other.inner)
            }
        }
        #[automatically_derived]
        impl ::core::cmp::Ord for Instant {
            #[inline]
            fn cmp(&self, other: &Instant) -> ::core::cmp::Ordering {
                ::core::cmp::Ord::cmp(&self.inner, &other.inner)
            }
        }
        #[automatically_derived]
        impl ::core::hash::Hash for Instant {
            #[inline]
            fn hash<__H: ::core::hash::Hasher>(&self, state: &mut __H) {
                ::core::hash::Hash::hash(&self.inner, state)
            }
        }
        impl View for Instant {
            type V = nat;
        }
        impl Instant {
            pub fn now() -> Self {
                Instant {
                    inner: ReactorInstant::now(),
                }
            }
            pub fn elapsed(&self) -> Duration {
                Duration::from(self.inner.elapsed())
            }
            pub fn less_than(&self, other: &Instant) -> bool {
                self.inner.inner < other.inner.inner
            }
            pub fn duration_since(&self, earlier: &Instant) -> Duration {
                Duration::from_millis(self.inner.inner.saturating_sub(earlier.inner.inner))
            }
        }
        impl From<ReactorInstant> for Instant {
            fn from(i: ReactorInstant) -> Self {
                Instant { inner: i }
            }
        }
    }
    mod task_id {
        use vstd::prelude::*;
        pub struct TaskId(pub u64);
        #[automatically_derived]
        impl ::core::fmt::Debug for TaskId {
            #[inline]
            fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
                ::core::fmt::Formatter::debug_tuple_field1_finish(f, "TaskId", &&self.0)
            }
        }
        #[automatically_derived]
        #[doc(hidden)]
        unsafe impl ::core::clone::TrivialClone for TaskId {}
        #[automatically_derived]
        impl ::core::clone::Clone for TaskId {
            #[inline]
            fn clone(&self) -> TaskId {
                let _: ::core::clone::AssertParamIsClone<u64>;
                *self
            }
        }
        #[automatically_derived]
        impl ::core::marker::Copy for TaskId {}
        #[automatically_derived]
        impl ::core::marker::StructuralPartialEq for TaskId {}
        #[automatically_derived]
        impl ::core::cmp::PartialEq for TaskId {
            #[inline]
            fn eq(&self, other: &TaskId) -> bool {
                self.0 == other.0
            }
        }
        #[automatically_derived]
        impl ::core::cmp::Eq for TaskId {
            #[inline]
            #[doc(hidden)]
            #[coverage(off)]
            fn assert_fields_are_eq(&self) {
                let _: ::core::cmp::AssertParamIsEq<u64>;
            }
        }
        #[automatically_derived]
        impl ::core::hash::Hash for TaskId {
            #[inline]
            fn hash<__H: ::core::hash::Hasher>(&self, state: &mut __H) {
                ::core::hash::Hash::hash(&self.0, state)
            }
        }
        impl View for TaskId {
            type V = nat;
        }
        impl DeepView for TaskId {
            type V = nat;
        }
    }
    mod boxed_future {
        use std::future::Future;
        use std::panic::{catch_unwind, AssertUnwindSafe};
        use std::pin::Pin;
        use std::task::{Context, Poll};
        use super::join_handle::{JoinError, JoinSender};
        use vstd::prelude::*;
        pub type BoxedFutureView = int;
        pub struct BoxedFuture {
            inner: Pin<Box<dyn Future<Output = ()> + Send + 'static>>,
        }
        impl View for BoxedFuture {
            type V = BoxedFutureView;
        }
        impl BoxedFuture {
            pub(crate) fn with_join_sender<F>(future: F, sender: JoinSender<F::Output>) -> Self
            where
                F: Future + Send + 'static,
                F::Output: Send + 'static,
            {
                BoxedFuture {
                    inner: Box::pin(TaskCell::new(future, sender)),
                }
            }
            pub(crate) fn local<F>(future: F, sender: JoinSender<F::Output>) -> Self
            where
                F: Future + 'static,
                F::Output: 'static,
            {
                BoxedFuture {
                    inner: Box::pin(OwnerThreadOnly(TaskCell::new(future, sender))),
                }
            }
            pub(crate) fn poll(&mut self, cx: &mut Context) -> Poll<()> {
                self.inner.as_mut().poll(cx)
            }
        }
        struct OwnerThreadOnly<F>(F);
        unsafe impl<F> Send for OwnerThreadOnly<F> {}
        impl<F: Future> Future for OwnerThreadOnly<F> {
            type Output = F::Output;
            fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<F::Output> {
                unsafe { self.map_unchecked_mut(|s| &mut s.0) }.poll(cx)
            }
        }
        pub(crate) struct TaskCell<F: Future> {
            future: Option<F>,
            sender: Option<JoinSender<F::Output>>,
            waker_registered: bool,
        }
        impl<F: Future> TaskCell<F> {
            pub(crate) fn new(future: F, sender: JoinSender<F::Output>) -> Self {
                TaskCell {
                    future: Some(future),
                    sender: Some(sender),
                    waker_registered: false,
                }
            }
        }
        impl<F: Future> Future for TaskCell<F> {
            type Output = ();
            fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
                let this = unsafe { self.get_unchecked_mut() };
                let sender = match this.sender.as_ref() {
                    Some(s) => s,
                    None => return Poll::Ready(()),
                };
                if !this.waker_registered {
                    sender.register_task_waker(cx.waker());
                    this.waker_registered = true;
                }
                let outcome = if sender.is_cancelled() {
                    Err(JoinError::cancelled())
                } else {
                    let future = this
                        .future
                        .as_mut()
                        .expect("unfinished task has its future");
                    let future = unsafe { Pin::new_unchecked(future) };
                    match catch_unwind(AssertUnwindSafe(|| future.poll(cx))) {
                        Ok(Poll::Pending) => return Poll::Pending,
                        Ok(Poll::Ready(value)) => Ok(value),
                        Err(payload) => Err(JoinError::panic(payload)),
                    }
                };
                let future = &mut this.future;
                let outcome = match catch_unwind(AssertUnwindSafe(|| *future = None)) {
                    Ok(()) => outcome,
                    Err(payload) => {
                        let _ = catch_unwind(AssertUnwindSafe(move || drop(outcome)));
                        Err(JoinError::panic(payload))
                    }
                };
                let sender = this.sender.take().expect("unfinished task has its sender");
                let _ = catch_unwind(AssertUnwindSafe(move || sender.finish(outcome)));
                Poll::Ready(())
            }
        }
    }
    mod task {
        use std::task::{Context, Poll};
        use super::{BoxedFuture, TaskId};
        use lion_executor_spec::types::TaskView;
        use vstd::prelude::*;
        pub struct Task {
            pub id: TaskId,
            pub future: BoxedFuture,
        }
        impl View for Task {
            type V = TaskView;
        }
        impl Task {
            pub fn new(id: TaskId, future: BoxedFuture) -> Self {
                Self { id, future }
            }
            pub fn id(&self) -> TaskId {
                self.id
            }
        }
        impl Task {
            pub(crate) fn poll(&mut self, cx: &mut Context) -> Poll<()> {
                self.future.poll(cx)
            }
        }
    }
    pub mod waker {
        use std::sync::Arc;
        use std::task::{Wake, Waker};
        use super::TaskId;
        use crate::tls;
        use vstd::prelude::*;
        pub enum WakeSource {
            Reactor,
            Task,
        }
        #[automatically_derived]
        #[doc(hidden)]
        unsafe impl ::core::clone::TrivialClone for WakeSource {}
        #[automatically_derived]
        impl ::core::clone::Clone for WakeSource {
            #[inline]
            fn clone(&self) -> WakeSource {
                *self
            }
        }
        #[automatically_derived]
        impl ::core::marker::Copy for WakeSource {}
        #[automatically_derived]
        impl ::core::marker::StructuralPartialEq for WakeSource {}
        #[automatically_derived]
        impl ::core::cmp::PartialEq for WakeSource {
            #[inline]
            fn eq(&self, other: &WakeSource) -> bool {
                let __self_discr = ::core::intrinsics::discriminant_value(self);
                let __arg1_discr = ::core::intrinsics::discriminant_value(other);
                __self_discr == __arg1_discr
            }
        }
        #[automatically_derived]
        impl ::core::cmp::Eq for WakeSource {
            #[inline]
            #[doc(hidden)]
            #[coverage(off)]
            fn assert_fields_are_eq(&self) {}
        }
        pub fn create_reactor_waker_for_current() -> Waker {
            let task_id = tls::get_current_task()
                .expect("create_reactor_waker_for_current called outside task context");
            create_waker(task_id, WakeSource::Reactor, false)
        }
        pub(crate) fn create_waker(task_id: TaskId, source: WakeSource, defer: bool) -> Waker {
            let (queue, interrupt, thread_id) = tls::get_cross_thread_ctx();
            Waker::from(ExecutorWaker::new(
                task_id, source, defer, queue, interrupt, thread_id,
            ))
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
                Arc::new(Self {
                    task_id,
                    source,
                    defer,
                    cross_thread_queue,
                    interrupt,
                    executor_thread_id,
                })
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
    }
    mod reactor {
        use lion_reactor::{Reactor as LionReactor, ReactorGuard as LionReactorGuard};
        use super::{Duration, Instant};
        use vstd::prelude::*;
        pub struct Reactor {
            inner: LionReactor,
            idle_park_ms: u64,
        }
        pub struct ReactorGuard {
            inner: LionReactorGuard,
        }
        impl Reactor {
            pub fn new(reactor: LionReactor) -> Self {
                Reactor {
                    inner: reactor,
                    idle_park_ms: IDLE_PARK_MS,
                }
            }
        }
        pub(crate) const IDLE_PARK_MS: u64 = 100;
        impl Reactor {
            pub(crate) fn idle_park_ms(&self) -> u64 {
                self.idle_park_ms
            }
            pub(crate) fn set_idle_park_ms(&mut self, ms: u64) {
                self.idle_park_ms = ms;
            }
            pub(crate) fn enter(&mut self) -> ReactorGuard {
                ReactorGuard {
                    inner: self.inner.enter(),
                }
            }
            pub(crate) fn park(&mut self, timeout: Option<Duration>) {
                self.inner.park(timeout.map(|d| d.into_reactor()));
            }
            pub(crate) fn next_deadline(&mut self) -> Option<Instant> {
                self.inner.next_deadline().map(Instant::from)
            }
            #[inline]
            pub(crate) fn flush_pending_deregister(&mut self) {
                self.inner.flush_pending_deregister();
            }
        }
    }
    pub mod join_handle {
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
            Panic(Mutex<Box<dyn Any + Send + 'static>>),
        }
        impl JoinError {
            /// A cancellation error. Public so adapters can map another runtime's join
            /// errors onto Lion's.
            pub fn cancelled() -> Self {
                JoinError {
                    repr: Repr::Cancelled,
                }
            }
            /// A panic error carrying `payload`, as returned by `std::panic::catch_unwind`.
            pub fn panic(payload: Box<dyn Any + Send + 'static>) -> Self {
                JoinError {
                    repr: Repr::Panic(Mutex::new(payload)),
                }
            }
            pub fn is_cancelled(&self) -> bool {
                #[allow(non_exhaustive_omitted_patterns)]
                match self.repr {
                    Repr::Cancelled => true,
                    _ => false,
                }
            }
            pub fn is_panic(&self) -> bool {
                #[allow(non_exhaustive_omitted_patterns)]
                match self.repr {
                    Repr::Panic(_) => true,
                    _ => false,
                }
            }
            /// The panic payload. Panics if the task was cancelled rather than panicked.
            pub fn into_panic(self) -> Box<dyn Any + Send + 'static> {
                self.try_into_panic()
                    .expect("`JoinError` reason is not a panic.")
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
                    (Repr::Cancelled, _) => f.write_fmt(format_args!("task was cancelled")),
                    (Repr::Panic(_), Some(msg)) => {
                        f.write_fmt(format_args!("task panicked with message {0:?}", msg))
                    }
                    (Repr::Panic(_), None) => f.write_fmt(format_args!("task panicked")),
                }
            }
        }
        impl std::fmt::Debug for JoinError {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                match (&self.repr, self.panic_message()) {
                    (Repr::Cancelled, _) => f.write_fmt(format_args!("JoinError::Cancelled")),
                    (Repr::Panic(_), Some(msg)) => {
                        f.write_fmt(format_args!("JoinError::Panic({0:?}, ...)", msg))
                    }
                    (Repr::Panic(_), None) => f.write_fmt(format_args!("JoinError::Panic(...)")),
                }
            }
        }
        impl std::error::Error for JoinError {}
        impl From<JoinError> for std::io::Error {
            fn from(e: JoinError) -> Self {
                std::io::Error::other(e)
            }
        }
        struct JoinShared<T> {
            cancelled: AtomicBool,
            state: Mutex<JoinState<T>>,
        }
        struct JoinState<T> {
            result: Option<Result<T, JoinError>>,
            join_waker: Option<Waker>,
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
                    state: Mutex::new(JoinState {
                        result: None,
                        join_waker: None,
                        task_waker: None,
                    }),
                });
                let handle = JoinHandle {
                    shared: shared.clone(),
                };
                let sender = JoinSender {
                    shared,
                    done: false,
                };
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
    }
    pub(crate) use lion_executor_spec::types::PollResult;
    pub(crate) use duration::Duration;
    pub(crate) use instant::Instant;
    pub(crate) use task_id::TaskId;
    pub(crate) use boxed_future::BoxedFuture;
    pub(crate) use task::Task;
    pub(crate) use lion_executor_spec::types::TaskView;
    pub(crate) use waker::{create_waker, WakeSource};
    pub(crate) use reactor::{Reactor, ReactorGuard, IDLE_PARK_MS};
    pub use join_handle::JoinHandle;
    pub use join_handle::JoinSender;
    use vstd::prelude::nat;
    pub use lion_executor_spec::types::TID;
    pub type DurationView = nat;
    pub type InstantView = nat;
}
mod handle {
    use std::future::Future;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicU64, Ordering};
    use lion_reactor::InterruptHandle;
    use crate::collections::MpscSender;
    use crate::types::{BoxedFuture, JoinHandle, JoinSender, Task, TaskId};
    pub(crate) struct InnerHandle {
        injection_queue: MpscSender<Task>,
        reactor_interrupt: InterruptHandle,
        task_id_counter: Arc<AtomicU64>,
        executor_thread_id: std::thread::ThreadId,
    }
    #[automatically_derived]
    impl ::core::clone::Clone for InnerHandle {
        #[inline]
        fn clone(&self) -> InnerHandle {
            InnerHandle {
                injection_queue: ::core::clone::Clone::clone(&self.injection_queue),
                reactor_interrupt: ::core::clone::Clone::clone(&self.reactor_interrupt),
                task_id_counter: ::core::clone::Clone::clone(&self.task_id_counter),
                executor_thread_id: ::core::clone::Clone::clone(&self.executor_thread_id),
            }
        }
    }
    impl InnerHandle {
        pub(crate) fn new(
            injection_queue: MpscSender<Task>,
            reactor_interrupt: InterruptHandle,
        ) -> Self {
            Self {
                injection_queue,
                reactor_interrupt,
                task_id_counter: Arc::new(AtomicU64::new(1)),
                executor_thread_id: std::thread::current().id(),
            }
        }
        pub(crate) fn runtime_id(&self) -> usize {
            Arc::as_ptr(&self.task_id_counter) as usize
        }
        pub fn spawn<T: Send + 'static>(
            &self,
            future: impl Future<Output = T> + Send + 'static,
        ) -> JoinHandle<T> {
            let task_id = TaskId(self.task_id_counter.fetch_add(1, Ordering::SeqCst));
            let (handle, sender) = JoinHandle::new();
            let task = Task::new(task_id, BoxedFuture::with_join_sender(future, sender));
            self.injection_queue.send(task);
            let _ = self.reactor_interrupt.wake();
            handle
        }
        pub fn spawn_local<F>(&self, future: F) -> JoinHandle<F::Output>
        where
            F: Future + 'static,
            F::Output: 'static,
        {
            if !(std::thread::current().id() == self.executor_thread_id) {
                {
                    ::core::panicking::panic_fmt(format_args!("spawn_local called off the runtime\'s thread: a !Send task must be spawned on the thread that owns its runtime"));
                }
            };
            let task_id = TaskId(self.task_id_counter.fetch_add(1, Ordering::SeqCst));
            let (handle, sender) = JoinHandle::new();
            let task = Task::new(task_id, BoxedFuture::local(future, sender));
            self.injection_queue.send(task);
            let _ = self.reactor_interrupt.wake();
            handle
        }
    }
}
pub mod tls {
    use std::cell::{Cell, RefCell};
    use std::collections::HashSet;
    use std::sync::{Arc, Mutex, PoisonError};
    use crate::collections::VecDeque;
    use crate::types::TaskId;
    use lion_reactor::InterruptHandle;
    const REACTOR_READY_QUEUE: ::std::thread::LocalKey<RefCell<VecDeque<TaskId>>> = {
        #[inline]
        fn __rust_std_internal_init_fn() -> RefCell<VecDeque<TaskId>> {
            RefCell::new(VecDeque::new())
        }
        unsafe {
            ::std::thread::LocalKey::new(
                const {
                    if ::std::mem::needs_drop::<RefCell<VecDeque<TaskId>>>() {
                        |__rust_std_internal_init| {
                            #[thread_local]
                            static __RUST_STD_INTERNAL_VAL: ::std::thread::local_impl::LazyStorage<
                                RefCell<VecDeque<TaskId>>,
                                (),
                            > = ::std::thread::local_impl::LazyStorage::new();
                            __RUST_STD_INTERNAL_VAL
                                .get_or_init(__rust_std_internal_init, __rust_std_internal_init_fn)
                        }
                    } else {
                        |__rust_std_internal_init| {
                            #[thread_local]
                            static __RUST_STD_INTERNAL_VAL: ::std::thread::local_impl::LazyStorage<
                                RefCell<VecDeque<TaskId>>,
                                !,
                            > = ::std::thread::local_impl::LazyStorage::new();
                            __RUST_STD_INTERNAL_VAL
                                .get_or_init(__rust_std_internal_init, __rust_std_internal_init_fn)
                        }
                    }
                },
            )
        }
    };
    const TASK_READY_QUEUE: ::std::thread::LocalKey<RefCell<VecDeque<TaskId>>> = {
        #[inline]
        fn __rust_std_internal_init_fn() -> RefCell<VecDeque<TaskId>> {
            RefCell::new(VecDeque::new())
        }
        unsafe {
            ::std::thread::LocalKey::new(
                const {
                    if ::std::mem::needs_drop::<RefCell<VecDeque<TaskId>>>() {
                        |__rust_std_internal_init| {
                            #[thread_local]
                            static __RUST_STD_INTERNAL_VAL: ::std::thread::local_impl::LazyStorage<
                                RefCell<VecDeque<TaskId>>,
                                (),
                            > = ::std::thread::local_impl::LazyStorage::new();
                            __RUST_STD_INTERNAL_VAL
                                .get_or_init(__rust_std_internal_init, __rust_std_internal_init_fn)
                        }
                    } else {
                        |__rust_std_internal_init| {
                            #[thread_local]
                            static __RUST_STD_INTERNAL_VAL: ::std::thread::local_impl::LazyStorage<
                                RefCell<VecDeque<TaskId>>,
                                !,
                            > = ::std::thread::local_impl::LazyStorage::new();
                            __RUST_STD_INTERNAL_VAL
                                .get_or_init(__rust_std_internal_init, __rust_std_internal_init_fn)
                        }
                    }
                },
            )
        }
    };
    const DEFER_QUEUE: ::std::thread::LocalKey<RefCell<VecDeque<TaskId>>> = {
        #[inline]
        fn __rust_std_internal_init_fn() -> RefCell<VecDeque<TaskId>> {
            RefCell::new(VecDeque::new())
        }
        unsafe {
            ::std::thread::LocalKey::new(
                const {
                    if ::std::mem::needs_drop::<RefCell<VecDeque<TaskId>>>() {
                        |__rust_std_internal_init| {
                            #[thread_local]
                            static __RUST_STD_INTERNAL_VAL: ::std::thread::local_impl::LazyStorage<
                                RefCell<VecDeque<TaskId>>,
                                (),
                            > = ::std::thread::local_impl::LazyStorage::new();
                            __RUST_STD_INTERNAL_VAL
                                .get_or_init(__rust_std_internal_init, __rust_std_internal_init_fn)
                        }
                    } else {
                        |__rust_std_internal_init| {
                            #[thread_local]
                            static __RUST_STD_INTERNAL_VAL: ::std::thread::local_impl::LazyStorage<
                                RefCell<VecDeque<TaskId>>,
                                !,
                            > = ::std::thread::local_impl::LazyStorage::new();
                            __RUST_STD_INTERNAL_VAL
                                .get_or_init(__rust_std_internal_init, __rust_std_internal_init_fn)
                        }
                    }
                },
            )
        }
    };
    const CURRENT_TASK: ::std::thread::LocalKey<Cell<Option<TaskId>>> = {
        const __RUST_STD_INTERNAL_INIT: Cell<Option<TaskId>> = { Cell::new(None) };
        unsafe {
            ::std::thread::LocalKey::new(
                const {
                    if ::std::mem::needs_drop::<Cell<Option<TaskId>>>() {
                        |_| {
                            #[thread_local]
                            static __RUST_STD_INTERNAL_VAL:
                                ::std::thread::local_impl::EagerStorage<Cell<Option<TaskId>>> =
                                ::std::thread::local_impl::EagerStorage::new(
                                    __RUST_STD_INTERNAL_INIT,
                                );
                            __RUST_STD_INTERNAL_VAL.get()
                        }
                    } else {
                        |_| {
                            #[thread_local]
                            static __RUST_STD_INTERNAL_VAL: Cell<Option<TaskId>> =
                                __RUST_STD_INTERNAL_INIT;
                            &__RUST_STD_INTERNAL_VAL
                        }
                    }
                },
            )
        }
    };
    const TASK_NOTIFIED: ::std::thread::LocalKey<RefCell<HashSet<u64>>> = {
        #[inline]
        fn __rust_std_internal_init_fn() -> RefCell<HashSet<u64>> {
            RefCell::new(HashSet::new())
        }
        unsafe {
            ::std::thread::LocalKey::new(
                const {
                    if ::std::mem::needs_drop::<RefCell<HashSet<u64>>>() {
                        |__rust_std_internal_init| {
                            #[thread_local]
                            static __RUST_STD_INTERNAL_VAL: ::std::thread::local_impl::LazyStorage<
                                RefCell<HashSet<u64>>,
                                (),
                            > = ::std::thread::local_impl::LazyStorage::new();
                            __RUST_STD_INTERNAL_VAL
                                .get_or_init(__rust_std_internal_init, __rust_std_internal_init_fn)
                        }
                    } else {
                        |__rust_std_internal_init| {
                            #[thread_local]
                            static __RUST_STD_INTERNAL_VAL: ::std::thread::local_impl::LazyStorage<
                                RefCell<HashSet<u64>>,
                                !,
                            > = ::std::thread::local_impl::LazyStorage::new();
                            __RUST_STD_INTERNAL_VAL
                                .get_or_init(__rust_std_internal_init, __rust_std_internal_init_fn)
                        }
                    }
                },
            )
        }
    };
    const BLOCK_ON_YIELDED: ::std::thread::LocalKey<Cell<bool>> = {
        const __RUST_STD_INTERNAL_INIT: Cell<bool> = { Cell::new(false) };
        unsafe {
            ::std::thread::LocalKey::new(
                const {
                    if ::std::mem::needs_drop::<Cell<bool>>() {
                        |_| {
                            #[thread_local]
                            static __RUST_STD_INTERNAL_VAL:
                                ::std::thread::local_impl::EagerStorage<Cell<bool>> =
                                ::std::thread::local_impl::EagerStorage::new(
                                    __RUST_STD_INTERNAL_INIT,
                                );
                            __RUST_STD_INTERNAL_VAL.get()
                        }
                    } else {
                        |_| {
                            #[thread_local]
                            static __RUST_STD_INTERNAL_VAL: Cell<bool> = __RUST_STD_INTERNAL_INIT;
                            &__RUST_STD_INTERNAL_VAL
                        }
                    }
                },
            )
        }
    };
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
        REACTOR_READY_QUEUE.with(|q| std::mem::take(&mut *q.borrow_mut()))
    }
    pub(crate) fn take_task_ready() -> VecDeque<TaskId> {
        TASK_READY_QUEUE.with(|q| std::mem::take(&mut *q.borrow_mut()))
    }
    pub(crate) fn take_deferred() -> VecDeque<TaskId> {
        DEFER_QUEUE.with(|q| std::mem::take(&mut *q.borrow_mut()))
    }
    pub fn set_current_task(task_id: TaskId) {
        CURRENT_TASK.with(|c| c.set(Some(task_id)));
    }
    pub fn clear_current_task() {
        CURRENT_TASK.with(|c| c.set(None));
    }
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
                {
                    ::core::panicking::panic_fmt(format_args!(
                        "defer_current called outside of task context"
                    ));
                };
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
    pub(crate) struct CrossThreadQueue {
        ids: Mutex<std::collections::VecDeque<TaskId>>,
    }
    impl CrossThreadQueue {
        pub fn new() -> Arc<Self> {
            Arc::new(Self {
                ids: Mutex::new(std::collections::VecDeque::new()),
            })
        }
        pub fn push(&self, task_id: TaskId) {
            self.ids
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .push_back(task_id);
        }
        fn take_all(&self) -> std::collections::VecDeque<TaskId> {
            std::mem::take(&mut *self.ids.lock().unwrap_or_else(PoisonError::into_inner))
        }
    }
    const CROSS_THREAD_CTX: ::std::thread::LocalKey<
        RefCell<
            Option<(
                Arc<CrossThreadQueue>,
                InterruptHandle,
                std::thread::ThreadId,
            )>,
        >,
    > = {
        #[inline]
        fn __rust_std_internal_init_fn() -> RefCell<
            Option<(
                Arc<CrossThreadQueue>,
                InterruptHandle,
                std::thread::ThreadId,
            )>,
        > {
            RefCell::new(None)
        }
        unsafe {
            ::std::thread::LocalKey::new(
                const {
                    if ::std::mem::needs_drop::<
                        RefCell<
                            Option<(
                                Arc<CrossThreadQueue>,
                                InterruptHandle,
                                std::thread::ThreadId,
                            )>,
                        >,
                    >() {
                        |__rust_std_internal_init| {
                            #[thread_local]
                            static __RUST_STD_INTERNAL_VAL: ::std::thread::local_impl::LazyStorage<
                                RefCell<
                                    Option<(
                                        Arc<CrossThreadQueue>,
                                        InterruptHandle,
                                        std::thread::ThreadId,
                                    )>,
                                >,
                                (),
                            > = ::std::thread::local_impl::LazyStorage::new();
                            __RUST_STD_INTERNAL_VAL
                                .get_or_init(__rust_std_internal_init, __rust_std_internal_init_fn)
                        }
                    } else {
                        |__rust_std_internal_init| {
                            #[thread_local]
                            static __RUST_STD_INTERNAL_VAL: ::std::thread::local_impl::LazyStorage<
                                RefCell<
                                    Option<(
                                        Arc<CrossThreadQueue>,
                                        InterruptHandle,
                                        std::thread::ThreadId,
                                    )>,
                                >,
                                !,
                            > = ::std::thread::local_impl::LazyStorage::new();
                            __RUST_STD_INTERNAL_VAL
                                .get_or_init(__rust_std_internal_init, __rust_std_internal_init_fn)
                        }
                    }
                },
            )
        }
    };
    pub(crate) fn set_cross_thread_ctx(
        queue: Arc<CrossThreadQueue>,
        interrupt: InterruptHandle,
        thread_id: std::thread::ThreadId,
    ) {
        CROSS_THREAD_CTX.with(|c| *c.borrow_mut() = Some((queue, interrupt, thread_id)));
    }
    pub(crate) fn get_cross_thread_ctx() -> (
        Arc<CrossThreadQueue>,
        InterruptHandle,
        std::thread::ThreadId,
    ) {
        CROSS_THREAD_CTX.with(|c| {
            c.borrow()
                .as_ref()
                .expect("cross-thread context not set")
                .clone()
        })
    }
    pub(crate) fn drain_cross_thread() {
        let batch =
            CROSS_THREAD_CTX.with(|c| c.borrow().as_ref().map(|(queue, _, _)| queue.take_all()));
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
        CROSS_THREAD_CTX
            .try_with(|c| c.borrow().is_some())
            .unwrap_or(true)
    }
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
}
pub mod blocking {
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
                let h = thread::spawn(move || loop {
                    loop {
                        let task = queue_clone.lock().unwrap().pop_front();
                        match task {
                            Some(t) => {
                                let _ = catch_unwind(AssertUnwindSafe(t.f));
                            }
                            None => break,
                        }
                    }
                    thread::park();
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
}
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
pub struct ExecutorHandle {
    inner: InnerHandle,
}
#[automatically_derived]
impl ::core::clone::Clone for ExecutorHandle {
    #[inline]
    fn clone(&self) -> ExecutorHandle {
        ExecutorHandle {
            inner: ::core::clone::Clone::clone(&self.inner),
        }
    }
}
impl ExecutorHandle {
    pub fn spawn<T: Send + 'static>(
        &self,
        future: impl Future<Output = T> + Send + 'static,
    ) -> types::JoinHandle<T> {
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
const CURRENT_HANDLE: ::std::thread::LocalKey<RefCell<Option<ExecutorHandle>>> = {
    #[inline]
    fn __rust_std_internal_init_fn() -> RefCell<Option<ExecutorHandle>> {
        RefCell::new(None)
    }
    unsafe {
        ::std::thread::LocalKey::new(
            const {
                if ::std::mem::needs_drop::<RefCell<Option<ExecutorHandle>>>() {
                    |__rust_std_internal_init| {
                        #[thread_local]
                        static __RUST_STD_INTERNAL_VAL: ::std::thread::local_impl::LazyStorage<
                            RefCell<Option<ExecutorHandle>>,
                            (),
                        > = ::std::thread::local_impl::LazyStorage::new();
                        __RUST_STD_INTERNAL_VAL
                            .get_or_init(__rust_std_internal_init, __rust_std_internal_init_fn)
                    }
                } else {
                    |__rust_std_internal_init| {
                        #[thread_local]
                        static __RUST_STD_INTERNAL_VAL: ::std::thread::local_impl::LazyStorage<
                            RefCell<Option<ExecutorHandle>>,
                            !,
                        > = ::std::thread::local_impl::LazyStorage::new();
                        __RUST_STD_INTERNAL_VAL
                            .get_or_init(__rust_std_internal_init, __rust_std_internal_init_fn)
                    }
                }
            },
        )
    }
};
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
    executor: RefCell<Box<Executor>>,
    handle: ExecutorHandle,
    _guard: ReactorGuard,
    _thread: ThreadBinding,
    _not_send: PhantomData<*const ()>,
}
struct ThreadBinding {
    runtime_id: usize,
    cross_thread_queue: Arc<tls::CrossThreadQueue>,
}
impl Drop for ThreadBinding {
    fn drop(&mut self) {
        let _ = CURRENT_HANDLE.try_with(|h| {
            if let Ok(mut h) = h.try_borrow_mut() {
                if h.as_ref()
                    .is_some_and(|x| x.inner.runtime_id() == self.runtime_id)
                {
                    *h = None;
                }
            }
        });
        tls::release_runtime_thread_state(&self.cross_thread_queue);
    }
}
fn runtime_is_live_on_this_thread() -> bool {
    CURRENT_HANDLE
        .try_with(|h| h.borrow().is_some())
        .unwrap_or(true)
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
    fn with_config(
        config: RuntimeConfig,
        backend: Option<Box<dyn OsBackend>>,
    ) -> std::io::Result<Self> {
        if runtime_is_live_on_this_thread() {
            return Err(std::io::Error::new(std::io::ErrorKind::AlreadyExists,
                        "a Lion runtime is already running on this thread; runtimes do not nest (run the new one on another thread)"));
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
        let handle = ExecutorHandle {
            inner: inner_handle,
        };
        CURRENT_HANDLE.with(|h| {
            *h.borrow_mut() = Some(handle.clone());
        });
        let thread = ThreadBinding {
            runtime_id: handle.inner.runtime_id(),
            cross_thread_queue,
        };
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
        let mut future = 'p: {
            super let mut pinned = future;
            break 'p unsafe { ::core::pin::Pin::new_unchecked(&mut pinned) };

            #[expect(unreachable_code)]
            ::core::pin::unreachable_pin_macro_type_constraint(pinned)
        };
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
        let ms = max_park
            .as_nanos()
            .div_ceil(1_000_000)
            .min(u64::MAX as u128) as u64;
        self.run_tick(ms);
    }
    fn run_tick(&self, idle_park_ms: u64) {
        if !tls::get_current_task().is_none() {
            {
                ::core::panicking::panic_fmt(format_args!("Runtime::tick called from inside a task or a block_on future: the runtime loop is not re-entrant"));
            }
        };
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
fn new_reactor(backend: Option<Box<dyn OsBackend>>) -> std::io::Result<(Reactor, InterruptHandle)> {
    let created = match backend {
        Some(backend) => Reactor::with_backend(backend),
        None => default_reactor()?,
    };
    match created {
        lion_reactor::IoResult::Ok(r) => Ok(r),
        lion_reactor::IoResult::Err(e) => {
            let e = e.into_io_error();
            Err(std::io::Error::new(
                e.kind(),
                ::alloc::__export::must_use({
                    ::alloc::fmt::format(format_args!("Failed to create reactor: {0}", e))
                }),
            ))
        }
    }
}
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
        if !(interval > 0) {
            {
                ::core::panicking::panic_fmt(format_args!("event_interval must be > 0"));
            }
        };
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
fn main() {}
