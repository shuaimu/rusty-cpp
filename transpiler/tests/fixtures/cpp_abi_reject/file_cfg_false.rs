#![cfg(any())]

#[cfg_attr(any(), cpp_abi(param(v, std_string_bytes), returns(std_string_bytes)))]
pub fn adapted(v: Vec<u8>) -> Vec<u8> {
    v
}
