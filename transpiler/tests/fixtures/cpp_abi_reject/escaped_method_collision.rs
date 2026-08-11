pub struct Owner;

impl Owner {
    #[cfg_attr(any(), cpp_abi(returns(std_string_bytes)))]
    pub fn r#class(value: u8) -> Vec<u8> {
        vec![value]
    }

    pub fn class_(value: u8) -> Vec<u8> {
        vec![value, value]
    }
}
