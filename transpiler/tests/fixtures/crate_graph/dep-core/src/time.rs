//! A dependency type named like std's (lion-reactor's
//! `types::time::Duration`): a consumer's bare `Duration::` under
//! `use std::time::Duration;` is still std's.

#[derive(Clone, Copy)]
pub struct Duration {
    pub ticks: u64,
}

impl Duration {
    pub fn from_ticks(ticks: u64) -> Duration {
        Duration { ticks }
    }
}

pub fn tick_total() -> u64 {
    Duration::from_ticks(7).ticks
}
