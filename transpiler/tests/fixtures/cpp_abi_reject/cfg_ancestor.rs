#[cfg(target_os = "linux")]
pub mod platform {
    #[cfg_attr(any(), cpp_abi(param(bytes, std_string_bytes), returns(std_string_bytes)))]
    pub fn adapted(bytes: Vec<u8>) -> Vec<u8> {
        bytes
    }
}

#[cfg(target_os = "linux")]
pub struct Codec;

#[cfg(target_os = "linux")]
impl Codec {
    #[cfg_attr(any(), cpp_abi(returns(std_string_bytes)))]
    pub fn encode(value: u8) -> Vec<u8> {
        vec![value]
    }
}
