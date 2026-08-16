#[cfg_attr(any(), cpp_import_namespace(rrr))]
use crate::rand::{randgen_rand_max, randgen_rand_raw};

pub fn crate_draw() -> f64 {
    randgen_rand_raw() as f64 / randgen_rand_max()
}
