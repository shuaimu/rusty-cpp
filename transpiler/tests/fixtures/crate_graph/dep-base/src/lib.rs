//! A dependency whose top-level module names (`reactor`) and types
//! (`Reactor`) recur, nested, in `dep-core`.

pub mod reactor {
    pub struct Reactor {
        pub n: u64,
    }

    impl Reactor {
        pub fn new() -> Self {
            Reactor { n: 1 }
        }

        pub fn park(&mut self, timeout: Option<crate::Ticks>) {
            self.n += timeout.map(|t| t.0).unwrap_or(0);
        }
    }
}

pub mod ticks {
    #[derive(Clone, Copy)]
    pub struct Ticks(pub u64);
}

/// A generic data enum (lion-reactor's `IoResult<T>`, lion-executor-spec's
/// `PollResult<T>`).
pub mod outcome {
    pub enum Outcome<T> {
        Done(T),
        Pending,
        Failed(u32),
    }
}

/// A move-only payload (`Box`) inside a generic enum.
pub fn start(k: u64) -> Outcome<(Reactor, Box<u64>)> {
    if k == 0 {
        Outcome::Failed(3)
    } else if k == 1 {
        Outcome::Pending
    } else {
        Outcome::Done((Reactor { n: k }, Box::new(k * 2)))
    }
}

pub fn check(k: u64) -> Outcome<()> {
    if k % 2 == 0 {
        Outcome::Done(())
    } else {
        Outcome::Pending
    }
}

pub mod stamp {
    #[derive(Clone, Copy)]
    pub struct Stamp(pub u64);
}

pub use outcome::Outcome;
pub use reactor::Reactor;
pub use stamp::Stamp;
pub use ticks::Ticks;
