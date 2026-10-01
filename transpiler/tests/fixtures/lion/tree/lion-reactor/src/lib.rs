// Plain-build lint allowances: imports/variables flagged unused here are used
// by ghost/proof code that plain `cargo build` (no Verus cfg) cannot see.
#![allow(unused_imports)]
#![allow(unused_variables)]
#![allow(unused_parens)]
#![allow(dead_code)]

pub mod spec;
pub mod types;
pub mod collections;
pub mod resource_slot;
pub mod resource_slot_wrapper;
pub mod resource_slab;
pub mod reactor;
pub mod handle;
pub mod readiness;
pub mod framework;
pub mod invariants;
pub mod proof;
pub mod alloc_verified;
pub mod os;
pub mod async_fd;

pub use async_fd::{AsyncFd, AsyncFdReadyGuard};
pub use handle::ReactorHandle;
pub use os::{OsBackend, OsEvent, OsInterrupt, RawFd};
#[cfg(feature = "mio")]
pub use os::MioBackend;
pub use reactor::{Reactor, ReactorGuard};
pub use types::{
  Duration, Instant, Interest, InterruptHandle, IoError, IoResult, ResourceId, Source, Waker,
};
