export module rrr.inline_consumer;

import rrr.rand;

export namespace rrr {

#if RUSTYCPP_RUST
#[cfg_attr(any(), cpp_import_namespace(rrr))]
use crate::rand::{randgen_rand_max, randgen_rand_raw};

pub fn inline_draw() -> f64 {
    randgen_rand_raw() as f64 / randgen_rand_max()
}
#endif
/*RUSTYCPP:GEN-BEGIN id=rrr.inline_consumer version=1 rust_sha256=deadbeef*/
// stale generated consumer
/*RUSTYCPP:GEN-END id=rrr.inline_consumer*/

#if RUSTYCPP_RUST
pub fn inline_unrelated() -> bool {
    true
}
#endif
/*RUSTYCPP:GEN-BEGIN id=rrr.inline_unrelated version=1 rust_sha256=deadbeef*/
// stale generated unrelated control
/*RUSTYCPP:GEN-END id=rrr.inline_unrelated*/

} // export namespace rrr
