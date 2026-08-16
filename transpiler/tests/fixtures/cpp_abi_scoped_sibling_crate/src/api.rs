#[cfg_attr(any(), cpp_abi(param(bytes, std_string_bytes), returns(std_string_bytes)))]
pub fn adapted(bytes: Vec<u8>) -> Vec<u8> {
    bytes
}

pub struct Picker;

impl Picker {
    #[cfg_attr(any(), cpp_abi(returns(std_string_bytes)))]
    pub fn choose() -> Vec<u8> {
        Vec::new()
    }
}
