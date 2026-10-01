//! Root of the --crate-graph fixture: implements a dependency's dyn trait.

use std::time::Duration;

pub struct RootBackend {
    pub k: u32,
}

impl dep_core::Backend for RootBackend {
    fn poll(&mut self) -> u32 {
        self.k
    }
}

/// SRPC's epoll backend shape: an inherent method and the dependency trait's
/// method of one C++ signature through the trait's `RawFd` alias (the trait
/// one forwards), and a same-named pair whose bodies differ.
pub struct EpollLike {
    pub closed: u32,
}

impl EpollLike {
    pub fn close(&mut self, fd: i32) -> u32 {
        self.closed += fd as u32;
        self.closed
    }

    pub fn label(&self) -> u32 {
        100
    }
}

impl dep_core::FdBackend for EpollLike {
    fn close(&mut self, fd: dep_core::backend::RawFd) -> u32 {
        EpollLike::close(self, fd)
    }

    fn label(&self) -> u32 {
        2000
    }
}

/// Direct calls reach the inherent methods, the trait object the trait's:
/// (3 + 100) + (14 + 2000) = 2117.
pub fn fd_total() -> u32 {
    let mut direct = EpollLike { closed: 1 };
    let near = direct.close(2) + direct.label();
    near + dep_core::with_fd_backend(Box::new(EpollLike { closed: 10 }))
}

pub fn run() -> u32 {
    let mut widget = dep_core::Widget::new(3);
    widget.bump();
    let backend: Box<dyn dep_core::Backend> = Box::new(RootBackend { k: 5 });
    let ids = [dep_core::Id(1), dep_core::Id(2), dep_core::Id(1)];
    dep_core::total(&widget)
        + dep_core::with_backend(backend)
        + dep_core::scan(&ids) as u32
        + dep_core::sched_total() as u32
        + micros()
        + dep_core::tick_total() as u32
        + dep_core::park_total() as u32
        + fd_total()
}

/// std's `Duration`, beside `dep_core::time::Duration`.
pub fn micros() -> u32 {
    Duration::from_micros(1500).as_micros() as u32
}
