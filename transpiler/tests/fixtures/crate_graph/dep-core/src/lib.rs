#[cfg(feature = "extra")]
mod extra;
pub mod backend;
pub mod spec;
pub mod widget;

pub use backend::{with_backend, Backend};
pub use spec::inv;
pub use widget::Widget;

pub fn total(widget: &Widget) -> u32 {
    widget.value() + bonus()
}

#[cfg(feature = "extra")]
fn bonus() -> u32 {
    extra::BONUS
}

#[cfg(not(feature = "extra"))]
fn bonus() -> u32 {
    0
}

/// A `Copy` key with a derived `Hash`.
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub struct Id(pub u64);

pub fn min_of(best: Option<u64>, d: u64) -> Option<u64> {
    match best {
        None => Some(d),
        Some(b) => {
            if d < b {
                Some(d)
            } else {
                Some(b)
            }
        }
    }
}

/// `Option<u64>` and `Id` are `Copy`: passing one by value leaves it usable.
pub fn scan(ids: &[Id]) -> u64 {
    let c0 = min_of(None, 7);
    let c1 = min_of(c0, 3);
    let c2 = min_of(c0, 9);
    let first = ids[0];
    let again = first;
    let mut repeats = 0u64;
    for id in ids {
        if *id == first {
            repeats += 1;
        }
    }
    c1.unwrap_or(0) * 1000 + c2.unwrap_or(0) * 100 + repeats * 10 + first.0 + again.0
}
