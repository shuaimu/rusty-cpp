use vstd::prelude::*;
pub use dep_spec::model::*;

verus! {

pub open spec fn inv(w: &crate::widget::Widget) -> bool {
    w.v >= 0
}

} // verus!
