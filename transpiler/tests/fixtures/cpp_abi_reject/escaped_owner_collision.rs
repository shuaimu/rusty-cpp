pub struct r#class;

impl r#class {
    #[cfg_attr(any(), cpp_abi(returns(std_string_bytes)))]
    pub fn encode(value: u8) -> Vec<u8> {
        vec![value]
    }
}

pub struct class_;

impl class_ {
    pub fn ordinary(value: u8) -> Vec<u8> {
        vec![value, value]
    }
}
