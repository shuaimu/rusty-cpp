pub struct Codec(pub u8);

impl Codec {
    #[cfg_attr(any(), cpp_abi(returns(std_string_bytes)))]
    pub fn encode(value: u8) -> Vec<u8> {
        vec![value]
    }
}
