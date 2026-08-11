module;

#include <rusty/rusty.hpp>

export module cpp_abi_inline;

import std;
import rusty;

export namespace cpp_abi_inline {

#if RUSTYCPP_RUST
#[cfg_attr(any(), cpp_abi(
    param(bytes, std_string_bytes),
    returns(std_string_bytes)
))]
pub fn echo_bytes(bytes: Vec<u8>) -> Vec<u8> {
    bytes
}
#endif
/*RUSTYCPP:GEN-BEGIN id=cpp_abi_inline.echo version=1 rust_sha256=deadbeef*/
// stale generated provider
/*RUSTYCPP:GEN-END id=cpp_abi_inline.echo*/

#if RUSTYCPP_RUST
#[cfg_attr(any(), cpp_abi_alias(std_vector))]
pub type InlineWeights = Vec<f64>;

pub struct InlineCodec {}

impl InlineCodec {
    #[cfg_attr(any(), cpp_abi(
        param(bytes, std_string_bytes),
        returns(std_string_bytes)
    ))]
    pub fn via_earlier(bytes: Vec<u8>) -> Vec<u8> {
        echo_bytes(bytes)
    }

    #[cfg_attr(any(), cpp_abi(param(
        weights,
        const_ref(InlineWeights)
    )))]
    pub fn count_weights(weights: &[f64]) -> u32 {
        weights.len() as u32
    }
}
#endif
/*RUSTYCPP:GEN-BEGIN id=cpp_abi_inline.codec version=1 rust_sha256=deadbeef*/
// stale generated consumer
/*RUSTYCPP:GEN-END id=cpp_abi_inline.codec*/

} // export namespace cpp_abi_inline
