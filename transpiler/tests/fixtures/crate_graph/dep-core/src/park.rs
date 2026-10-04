//! lion-executor's Runtime and Executor over lion-reactor: a crate `Driver`
//! wrapping the dependency's `Reactor`, each with a `park` taking a different
//! `Option<..>`; `impl Executor` blocks merged into the struct from
//! modules whose own imports their names resolve through; and a dependency's
//! generic enum matched by value (a move-only payload) and by variant.

pub mod rt {
    pub mod ticks {
        use dep_base::Ticks as BaseTicks;

        #[derive(Clone, Copy)]
        pub struct Ticks {
            inner: BaseTicks,
        }

        impl Ticks {
            pub fn zero() -> Ticks {
                Ticks { inner: BaseTicks(0) }
            }

            pub fn from_raw(n: u64) -> Ticks {
                Ticks { inner: BaseTicks(n) }
            }

            pub fn into_base(self) -> BaseTicks {
                self.inner
            }
        }
    }

    pub use ticks::Ticks;

    pub mod driver {
        use super::Ticks;
        use dep_base::Reactor as LowerReactor;

        pub struct Driver {
            pub inner: LowerReactor,
            pub idle: u64,
        }

        impl Driver {
            pub fn idle(&self) -> u64 {
                self.idle
            }

            // Same name as `LowerReactor::park`, a different parameter type.
            pub fn park(&mut self, timeout: Option<Ticks>) {
                self.inner.park(timeout.map(|t| t.into_base()));
            }
        }
    }
}

pub mod executor {
    use crate::park::rt::driver::Driver;

    pub struct Executor {
        pub driver: Driver,
    }

    pub mod new {
        use super::Executor;
        use crate::park::rt::driver::Driver;
        use dep_base::Reactor as LowerReactor;

        impl Executor {
            pub fn build(inner: LowerReactor, idle: u64) -> Executor {
                Executor { driver: Driver { inner, idle } }
            }
        }
    }

    pub mod ext {
        use super::Executor;
        use crate::park::rt::Ticks;

        // `executor` imports no `Ticks`: this module's import names it.
        impl Executor {
            pub fn park_action(&mut self, require: bool) -> u64 {
                let timeout = if require {
                    Some(Ticks::from_raw(self.driver.idle()))
                } else {
                    Some(Ticks::zero())
                };
                self.driver.park(timeout);
                self.driver.inner.n
            }
        }
    }
}

fn take(pair: (dep_base::Reactor, Box<u64>)) -> u64 {
    pair.0.n * 10 + *pair.1
}

pub fn started(k: u64) -> u64 {
    let created: dep_base::Outcome<(dep_base::Reactor, Box<u64>)> = dep_base::start(k);
    match created {
        dep_base::Outcome::Done(pair) => take(pair),
        dep_base::Outcome::Pending => 1000,
        dep_base::Outcome::Failed(e) => 2000 + e as u64,
    }
}

pub fn checked(k: u64) -> u64 {
    let r = dep_base::check(k);
    let mut hits = 0;
    match r {
        dep_base::Outcome::Done(()) => {
            hits += 1;
        }
        _ => {}
    }
    hits
}

pub fn park_total() -> u64 {
    let mut e = executor::Executor::build(dep_base::Reactor::new(), 7);
    e.park_action(true)
        + e.park_action(false)
        + started(4)
        + started(1)
        + started(0)
        + checked(2)
        + checked(3)
}
