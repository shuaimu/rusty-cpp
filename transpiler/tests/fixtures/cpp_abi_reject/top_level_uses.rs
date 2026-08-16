#[cfg_attr(any(), cpp_abi(param(bytes, std_string_bytes), returns(std_string_bytes)))]
pub fn adapted(bytes: Vec<u8>) -> Vec<u8> {
    bytes
}

pub const CONST_FN_VALUE: fn(Vec<u8>) -> Vec<u8> = adapted;
pub static STATIC_FN_VALUE: fn(Vec<u8>) -> Vec<u8> = adapted;

pub struct Holder;

impl Holder {
    pub const ASSOCIATED_FN_VALUE: fn(Vec<u8>) -> Vec<u8> = adapted;
}

pub trait DefaultCall {
    fn call(bytes: Vec<u8>) -> Vec<u8> {
        adapted(bytes)
    }
}
