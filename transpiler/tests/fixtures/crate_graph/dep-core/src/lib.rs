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
