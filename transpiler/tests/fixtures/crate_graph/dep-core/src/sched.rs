//! Name-resolution shapes an executor crate uses (lion-executor under
//! --crate-graph): relative re-exports out of nested child modules, a child
//! module named like a dependency's module, a renamed dependency import beside
//! a same-named local type, a module re-exporting the same-named function it
//! declares, a nested glob-only module, a thread-local whose value type is
//! declared in a later module, a merged impl whose signature names a
//! dependency type imported only where the impl is written, and a projection
//! of a type parameter's associated type.

use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

use self::queue::channel;

pub mod types {
    mod reactor {
        use dep_base::Reactor as BaseReactor;

        pub struct Reactor {
            pub inner: BaseReactor,
            pub idle: u64,
        }

        impl Reactor {
            pub fn new() -> Self {
                Reactor { inner: BaseReactor::new(), idle: IDLE_MS }
            }
        }

        pub(crate) const IDLE_MS: u64 = 100;
    }

    mod task {
        pub struct Task {
            pub id: u64,
        }
    }

    pub mod waker {
        #[derive(Clone, Copy, PartialEq, Eq)]
        pub enum WakeSource {
            Reactor,
            Task,
        }

        pub fn is_reactor(source: WakeSource) -> bool {
            source == WakeSource::Reactor
        }
    }

    pub(crate) use reactor::{Reactor, IDLE_MS};
    pub(crate) use task::Task;
    pub(crate) use waker::WakeSource;
}

pub mod tls {
    use std::cell::RefCell;
    use super::queue::Sender;

    thread_local! {
        static LAST: RefCell<Option<Sender<u64>>> = RefCell::new(None);
    }

    pub fn remember(sender: Sender<u64>) -> u64 {
        let v = sender.v;
        LAST.with(|last| *last.borrow_mut() = Some(sender));
        v
    }
}

pub mod queue {
    mod channel {
        pub struct Sender<T> {
            pub v: T,
        }

        pub fn channel<T>(v: T) -> Sender<T> {
            Sender { v }
        }
    }

    pub use channel::{channel, Sender};
}

pub mod log {
    pub use dep_base::stamp::*;
}

pub mod exec {
    pub struct Exec {
        pub n: u64,
    }

    mod ext {
        use dep_base::Stamp;

        impl super::Exec {
            pub fn stamp(&self) -> Stamp {
                Stamp(self.n * 3)
            }
        }
    }
}

/// A pinned projection over any `F: Future`.
pub struct Wrap<F>(pub F);

impl<F: Future> Future for Wrap<F> {
    type Output = F::Output;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<F::Output> {
        unsafe { self.map_unchecked_mut(|s| &mut s.0) }.poll(cx)
    }
}

pub fn sched_total() -> u64 {
    let r = types::Reactor::new();
    let t = types::Task { id: 2 };
    let w = if types::waker::is_reactor(types::WakeSource::Reactor) { 1 } else { 0 };
    let queued = tls::remember(channel(7u64));
    let stamp = exec::Exec { n: 5 }.stamp();
    let logged = log::Stamp(4);
    r.idle + r.inner.n + t.id + w + types::IDLE_MS + queued + stamp.0 + logged.0
}
