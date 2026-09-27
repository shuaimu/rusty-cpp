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
    }
}

pub mod stamp {
    #[derive(Clone, Copy)]
    pub struct Stamp(pub u64);
}

pub use reactor::Reactor;
pub use stamp::Stamp;
